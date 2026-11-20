//! painhubproxy for the desktop: a tray app that serves the local MTProto
//! listener on the Wi-Fi network, so phones reach Telegram through this machine.
//!
//! Start-up order matters — settings first (they own the secret and the LAN
//! address), then logging, then the engine, then the tray. The tray is the last
//! thing to exist because its callbacks send into a channel that must already
//! have a reader, which is the loop at the bottom.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod engine;
mod platform;
mod settings;
mod tray;

use std::sync::mpsc;
use std::time::Duration;

use tracing::{info, warn};

use crate::engine::{Engine, State};
use crate::tray::{Action, Tray};

fn main() {
    if !platform::claim_single_instance() {
        platform::notify(
            "painhubproxy",
            "Приложение уже запущено — его значок находится в области уведомлений.",
        );
        return;
    }

    let mut settings = settings::Settings::load_or_create();
    platform::init_logging(&settings.log_path(), settings.verbose);

    info!(
        "painhubproxy desktop {} — настройки: {}, журнал: {}",
        env!("CARGO_PKG_VERSION"),
        settings.config_path().display(),
        settings.log_path().display()
    );

    let mut engine = Engine::new(settings.clone());
    engine.start();

    let (actions, incoming) = mpsc::channel::<Action>();
    let mut tray = match Tray::new(actions, settings.autostart) {
        Some(tray) => tray,
        None => {
            platform::notify(
                "painhubproxy",
                "Не удалось создать значок в области уведомлений.",
            );
            return;
        }
    };

    let mut shown_state: Option<State> = None;
    let mut shown_link: Option<String> = None;

    loop {
        platform::pump_messages();

        let mut quit = false;
        while let Ok(action) = incoming.try_recv() {
            match action {
                Action::CopyLink => match engine.link() {
                    Some(link) => {
                        platform::copy_text(&link);
                        info!("ссылка скопирована: {link}");
                    }
                    None => warn!("ссылки ещё нет — прокси не запущен"),
                },
                Action::OpenTelegram => match engine.link() {
                    Some(link) => {
                        info!("открываю ссылку в Telegram");
                        platform::open_url(&link);
                    }
                    None => warn!("ссылки ещё нет — прокси не запущен"),
                },
                Action::Toggle => {
                    if engine.is_active() {
                        info!("остановка по команде из меню");
                        engine.stop();
                    } else {
                        info!("запуск по команде из меню");
                        engine.start();
                    }
                }
                Action::Restart => {
                    info!("перезапуск прокси");
                    engine.stop();
                    engine.start();
                }
                Action::OpenLog => platform::open_path(&settings.log_path()),
                Action::OpenConfig => platform::open_path(&settings.config_path()),
                Action::ToggleAutostart => {
                    let next = !settings.autostart;
                    match platform::set_autostart(next) {
                        Ok(()) => {
                            settings.autostart = next;
                            settings.save();
                            tray.set_autostart(next);
                            info!("автозапуск: {}", if next { "включён" } else { "выключен" });
                        }
                        Err(e) => warn!("не удалось изменить автозапуск: {e}"),
                    }
                }
                Action::Quit => quit = true,
            }
        }

        if quit {
            info!("выход");
            engine.stop();
            return;
        }

        let state = engine.state();
        let link = engine.link();
        if shown_state.as_ref() != Some(&state) || shown_link != link {
            tray.refresh(&state, link.as_deref());
            shown_state = Some(state);
            shown_link = link;
        }

        // The message pump only dispatches what has already arrived, so the
        // loop still has to sleep; 50 ms is invisible for a tray menu and keeps
        // the process at effectively zero CPU.
        std::thread::sleep(Duration::from_millis(50));
    }
}
