//! The administration window: live status, one link and QR per network
//! interface, editable settings, and a tail of the log file.
//!
//! The window is a *second* way to drive the same `Engine` the tray drives —
//! never a second proxy. Both read `Engine::state()` and both call the same
//! start/stop, so the tray and the window can never disagree about reality for
//! longer than one frame.

mod links;
mod pages;
mod qr;
mod theme;

use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use eframe::egui;
use tracing::info;

use crate::engine::{Engine, State};
use crate::platform;
use crate::probe::Prober;
use crate::settings::Settings;
use crate::tray::{Action, Tray};

pub use self::links::Endpoint;
use self::qr::QrMatrix;
use self::theme as palette;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Home,
    Devices,
    Settings,
    Log,
}

impl Tab {
    pub fn label(self) -> &'static str {
        match self {
            Self::Home => "Обзор",
            Self::Devices => "Подключение",
            Self::Settings => "Настройки",
            Self::Log => "Журнал",
        }
    }
}

pub struct App {
    pub settings: Settings,
    pub engine: Engine,
    /// Measures the Cloudflare leg; read-only for the window.
    pub prober: Prober,
    pub tab: Tab,
    /// Last user-visible feedback line ("Ссылка скопирована", errors, …).
    pub note: String,
    /// Settings edited in the UI but not written to disk yet.
    pub dirty: bool,
    pub show_secret: bool,
    pub endpoints: Vec<Endpoint>,
    pub log_text: String,
    pub autoscroll: bool,

    tray: Option<Tray>,
    actions: Receiver<Action>,
    quitting: bool,
    qr_cache: HashMap<String, QrMatrix>,
    log_read_at: Instant,
    tray_shown: Option<(State, String)>,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        settings: Settings,
        engine: Engine,
        actions: Receiver<Action>,
        tray: Option<Tray>,
    ) -> Self {
        palette::apply(&cc.egui_ctx);

        let log_text = read_tail(&settings.log_path(), 96 * 1024);
        let endpoints = links::endpoints();
        let prober = Prober::start(settings.cf_ips(), settings.cf_port());

        Self {
            settings,
            engine,
            prober,
            tab: Tab::Home,
            note: String::new(),
            dirty: false,
            show_secret: false,
            endpoints,
            log_text,
            autoscroll: true,
            tray,
            actions,
            quitting: false,
            qr_cache: HashMap::new(),
            log_read_at: Instant::now(),
            tray_shown: None,
        }
    }

    /// Address written into the "main" link: the configured one, otherwise the
    /// first real network interface.
    pub fn primary_ip(&self) -> String {
        let configured = self.settings.link_ip.trim();
        if !configured.is_empty() {
            return configured.to_string();
        }
        self.endpoints
            .iter()
            .find(|endpoint| !matches!(endpoint.kind, links::Kind::Local))
            .map(|endpoint| endpoint.ip.clone())
            .unwrap_or_else(|| "127.0.0.1".to_string())
    }

    pub fn primary_link(&self) -> String {
        // While the core is running it reports the link it actually built —
        // including the real port when settings say `0`. Stopped, the link is
        // rebuilt from the settings so the window always has something to copy.
        if let Some(link) = self.engine.link() {
            return link;
        }
        links::build_link(&self.primary_ip(), self.settings.port, &self.settings.secret)
    }

    /// QR matrix for `link`, generated once and cached per link.
    pub fn matrix(&mut self, link: &str) -> Option<&QrMatrix> {
        if !self.qr_cache.contains_key(link) {
            let matrix = QrMatrix::build(link)?;
            self.qr_cache.insert(link.to_string(), matrix);
        }
        self.qr_cache.get(link)
    }

    pub fn toggle(&mut self) {
        if self.engine.is_active() {
            info!("остановка из окна");
            self.engine.stop();
            self.note = "Прокси остановлен".into();
        } else {
            info!("запуск из окна");
            self.engine.start();
            self.note = "Запуск…".into();
        }
    }

    pub fn restart(&mut self) {
        self.engine.stop();
        self.engine.start();
        self.note = "Перезапуск…".into();
    }

    pub fn set_autostart(&mut self, enabled: bool) {
        match platform::set_autostart(enabled) {
            Ok(()) => {
                self.settings.autostart = enabled;
                self.settings.save();
                if let Some(tray) = self.tray.as_mut() {
                    tray.set_autostart(enabled);
                }
                self.note = format!(
                    "Автозапуск {}",
                    if enabled { "включён" } else { "выключен" }
                );
            }
            Err(e) => self.note = format!("Автозапуск: {e}"),
        }
    }

    pub fn toggle_autostart(&mut self) {
        self.set_autostart(!self.settings.autostart);
    }

    pub fn save_settings(&mut self) {
        self.settings.save();
        self.dirty = false;
        self.refresh_targets();
        self.note = "Настройки сохранены".into();
    }

    pub fn save_and_restart(&mut self) {
        self.settings.save();
        self.dirty = false;
        self.refresh_targets();
        self.engine.apply_settings(self.settings.clone());
        self.engine.stop();
        self.engine.start();
        self.note = "Настройки сохранены, прокси перезапущен".into();
    }

    /// Re-derive everything that depends on saved settings: the address list,
    /// the cached QR matrices, and the addresses the latency probe dials.
    fn refresh_targets(&mut self) {
        self.endpoints = links::endpoints();
        self.qr_cache.clear();
        self.prober
            .retarget(self.settings.cf_ips(), self.settings.cf_port());
    }

    fn handle_actions(&mut self, ctx: &egui::Context) {
        while let Ok(action) = self.actions.try_recv() {
            match action {
                Action::ShowWindow => platform::show_main_window(),
                Action::ShowDevices => {
                    self.tab = Tab::Devices;
                    platform::show_main_window();
                }
                Action::CopyLink => {
                    platform::copy_text(&self.primary_link());
                    self.note = "Ссылка скопирована в буфер обмена".into();
                }
                Action::OpenTelegram => {
                    let link = self.primary_link();
                    platform::open_url(&link);
                    self.note = "Открываю ссылку в Telegram".into();
                }
                Action::Toggle => self.toggle(),
                Action::Restart => self.restart(),
                Action::OpenLog => platform::open_path(&self.settings.log_path()),
                Action::OpenConfig => platform::open_path(&self.settings.config_path()),
                Action::ToggleAutostart => self.toggle_autostart(),
                Action::Quit => {
                    info!("выход по команде из меню");
                    self.engine.stop();
                    self.quitting = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_actions(ctx);

        // The close button must not stop the proxy — this is a tray app first.
        // Minimising (not hiding) keeps the event loop running, so the tray
        // stays live and the window can always be restored from the tray menu
        // or the taskbar. Hiding the window would stop `update` from being
        // called at all, and then nothing could bring it back.
        if ctx.input(|input| input.viewport().close_requested()) && !self.quitting {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            self.note = "Свёрнуто — прокси продолжает работать".into();
        }

        self.maybe_read_log();
        self.sync_tray();
        self.header(ctx);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(palette::BG)
                    .inner_margin(egui::Margin::same(20.0)),
            )
            .show(ctx, |ui| match self.tab {
                Tab::Home => pages::home(self, ui),
                Tab::Devices => pages::devices(self, ui),
                Tab::Settings => pages::settings(self, ui),
                Tab::Log => pages::log(self, ui),
            });

        // A tray app is idle most of the time. The proxy does not need the
        // event loop at all — it runs on its own thread — so the window only has
        // to redraw often enough for the status to look alive: 400 ms when it is
        // on screen, and much less when it is minimised to the tray, where
        // nobody can see it anyway.
        let minimized = ctx.input(|input| input.viewport().minimized.unwrap_or(false));
        let interval = if minimized { 1500 } else { 400 };
        ctx.request_repaint_after(Duration::from_millis(interval));
    }
}

impl App {
    fn header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::none()
                    .fill(palette::BG)
                    .inner_margin(egui::Margin::same(20.0)),
            )
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("painhubproxy")
                        .size(24.0)
                        .strong()
                        .color(palette::INK),
                );
                ui.label(
                    egui::RichText::new("прокси для Telegram по локальной сети")
                        .size(13.0)
                        .color(palette::MUTED),
                );

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    for tab in [Tab::Home, Tab::Devices, Tab::Settings, Tab::Log] {
                        if palette::nav_item(ui, self.tab == tab, tab.label()).clicked() {
                            self.tab = tab;
                        }
                        ui.add_space(10.0);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !self.note.is_empty() {
                            ui.label(
                                egui::RichText::new(&self.note)
                                    .size(11.0)
                                    .color(palette::MUTED),
                            );
                        }
                    });
                });
            });
    }

    /// Push state into the tray menu, but only when it actually changed:
    /// `refresh` calls into the Win32 shell and there is no reason to do that
    /// 2.5 times a second.
    fn sync_tray(&mut self) {
        let state = self.engine.state();
        let link = self.primary_link();
        if self.tray_shown.as_ref() == Some(&(state.clone(), link.clone())) {
            return;
        }
        if let Some(tray) = self.tray.as_mut() {
            tray.refresh(&state, Some(&link));
        }
        self.tray_shown = Some((state, link));
    }

    fn maybe_read_log(&mut self) {
        if self.log_read_at.elapsed() < Duration::from_millis(700) {
            return;
        }
        self.log_read_at = Instant::now();
        self.log_text = read_tail(&self.settings.log_path(), 96 * 1024);
    }
}

/// Window icon, decoded from the PNG the repository already ships, so the
/// window matches the tray and the Android launcher.
pub fn window_icon() -> Option<egui::IconData> {
    const PNG: &[u8] = include_bytes!("../../../docs/icon.png");

    let image = image::load_from_memory(PNG).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    Some(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

/// Last `bytes` of a file as text.
///
/// The log rotates at 4 MB and is appended to forever, so the whole file is
/// never read: only the tail the window can actually show.
fn read_tail(path: &std::path::Path, bytes: u64) -> String {
    use std::io::{Read, Seek, SeekFrom};

    let Ok(mut file) = std::fs::File::open(path) else {
        return String::new();
    };
    let len = file.metadata().map(|meta| meta.len()).unwrap_or(0);
    if file
        .seek(SeekFrom::Start(len.saturating_sub(bytes)))
        .is_err()
    {
        return String::new();
    }

    let mut buffer = Vec::new();
    let _ = file.read_to_end(&mut buffer);
    let text = String::from_utf8_lossy(&buffer).into_owned();

    // A byte offset can land in the middle of a line; drop the broken head so
    // the view never starts with half a timestamp.
    match text.find('\n') {
        Some(index) if index + 1 < text.len() => text[index + 1..].to_string(),
        _ => text,
    }
}
