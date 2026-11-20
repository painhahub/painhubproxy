//! painhubproxy for the desktop: a tray app with an administration window that
//! serves the local MTProto listener on the Wi-Fi network, so phones reach
//! Telegram through this machine.
//!
//! Start-up order matters — settings first (they own the secret and the LAN
//! address), then logging, then the engine, then the window. The tray icon is
//! built before `run_native` because it installs the global menu and click
//! handlers that must exist before the event loop starts dispatching.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod engine;
mod platform;
mod probe;
mod settings;
mod tray;
mod ui;

use std::sync::mpsc;

use eframe::egui;
use tracing::info;

use crate::engine::Engine;
use crate::settings::Settings;
use crate::tray::Tray;

fn main() {
    if !platform::claim_single_instance() {
        platform::notify(
            "painhubproxy",
            "Приложение уже запущено — его значок находится в области уведомлений.",
        );
        return;
    }

    let settings = Settings::load_or_create();
    platform::init_logging(&settings.log_path(), settings.verbose);

    info!(
        "painhubproxy desktop {} — настройки: {}, журнал: {}",
        env!("CARGO_PKG_VERSION"),
        settings.config_path().display(),
        settings.log_path().display()
    );

    let mut engine = Engine::new(settings.clone());
    engine.start();

    let (actions_tx, actions_rx) = mpsc::channel();
    let tray = Tray::new(actions_tx, settings.autostart);
    if tray.is_none() {
        info!("значок в трее не создан — управление доступно только из окна");
    }

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1010.0, 720.0])
        .with_min_inner_size([780.0, 560.0])
        .with_title("painhubproxy");
    if let Some(icon) = ui::window_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let result = eframe::run_native(
        "painhubproxy",
        options,
        Box::new(move |cc| {
            Ok(Box::new(ui::App::new(
                cc,
                settings,
                engine,
                actions_rx,
                tray,
            )))
        }),
    );

    match result {
        Ok(()) => info!("окно закрыто, выход"),
        Err(e) => {
            tracing::error!("не удалось открыть окно: {e}");
            platform::notify(
                "painhubproxy",
                &format!("Не удалось открыть окно интерфейса:\n{e}"),
            );
        }
    }
}
