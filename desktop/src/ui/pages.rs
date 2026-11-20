//! The four tabs of the window.
//!
//! Everything here reads and writes `App` fields directly: the tabs are pure
//! presentation, and every state change goes through the same `Engine` methods
//! the tray uses. No tab starts a proxy of its own.

use eframe::egui::{self, RichText};

use super::theme as palette;
use super::{links, qr, App};
use crate::engine::State;
use crate::platform;

/// Overview: what the proxy is doing right now and the link to hand to a phone.
pub fn home(app: &mut App, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(4.0);

        let state = app.engine.state();
        let (color, text) = match &state {
            State::Running => (palette::OK, "Прокси работает".to_string()),
            State::Starting => (palette::WARN, "Запуск…".to_string()),
            State::Failed(reason) => (palette::BAD, format!("Ошибка: {reason}")),
            State::Stopped => (palette::MUTED, "Прокси остановлен".to_string()),
        };
        palette::status_line(ui, color, &text);
        ui.add_space(6.0);

        let address = links::address_of(&app.primary_ip(), app.settings.port);
        let link = app.primary_link();

        palette::section(ui, "Управление", |ui| {
            ui.horizontal(|ui| {
                let active = app.engine.is_active();
                if ui
                    .button(if active { "Остановить" } else { "Запустить" })
                    .clicked()
                {
                    app.toggle();
                }
                if ui.button("Перезапустить").clicked() {
                    app.restart();
                }
                if ui.button("Копировать ссылку").clicked() {
                    platform::copy_text(&link);
                    app.note = "Ссылка скопирована в буфер обмена".into();
                }
                if ui.button("Открыть в Telegram").clicked() {
                    platform::open_url(&link);
                }
            });
            ui.add_space(4.0);
            ui.label(RichText::new(address).monospace().size(13.0));
        });

        palette::section(ui, "Ссылка для телефона", |ui| {
            palette::hint(ui, "Отправьте себе в Telegram и откройте на телефоне — или отсканируйте QR из вкладки «Подключение».");
            ui.add_space(4.0);
            palette::mono(ui, &link, 11.5);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if let Some(matrix) = app.matrix(&link) {
                    qr::draw(ui, matrix, 168.0);
                } else {
                    palette::hint(ui, "QR не построен: слишком длинная ссылка.");
                }
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    palette::hint(ui, "Секрет привязан к этой установке и не меняется между запусками, поэтому ссылку достаточно добавить один раз.");
                    ui.add_space(6.0);
                    palette::hint(ui, "Если телефон не подключается, проверьте, что он в той же сети, и разрешите приложению работу в фоне.");
                });
            });
        });
    });
}

/// Every address the machine can be reached at, each with its own link and QR.
pub fn devices(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    palette::hint(
        ui,
        "Ссылка зависит от адреса, по которому телефон видит компьютер. Одна и та же сеть — одна ссылка; разные строки появляются, когда интерфейсов несколько (Wi-Fi и Ethernet, VPN).",
    );
    ui.add_space(8.0);

    egui::ScrollArea::vertical().show(ui, |ui| {
        let total = app.endpoints.len();
        for index in 0..total {
            let (ip, label, hint) = {
                let endpoint = &app.endpoints[index];
                (
                    endpoint.ip.clone(),
                    endpoint.label(),
                    endpoint.kind.hint().to_string(),
                )
            };
            let link = links::build_link(&ip, app.settings.port, &app.settings.secret);

            palette::section(ui, &format!("{label} · {ip}"), |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        palette::hint(ui, &hint);
                        ui.add_space(2.0);
                        palette::mono(ui, &link, 11.0);
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui.button("Копировать").clicked() {
                                platform::copy_text(&link);
                                app.note = format!("Ссылка {ip} скопирована");
                            }
                            if ui.button("Открыть в Telegram").clicked() {
                                platform::open_url(&link);
                            }
                        });
                    });

                    ui.add_space(12.0);
                    if let Some(matrix) = app.matrix(&link) {
                        qr::draw(ui, matrix, 132.0);
                    }
                });
            });
        }

        if total <= 1 {
            palette::hint(
                ui,
                "Кроме loopback других адресов не найдено: сеть не поднята или интерфейсы отключены.",
            );
        }
    });
}

/// Everything the core accepts, in the places it is actually edited.
pub fn settings(app: &mut App, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(4.0);

        palette::section(ui, "Сеть", |ui| {
            ui.horizontal(|ui| {
                ui.label("Порт");
                let mut port = app.settings.port;
                if ui
                    .add(egui::DragValue::new(&mut port).range(1..=65535).speed(1.0))
                    .changed()
                {
                    app.settings.port = port;
                    app.dirty = true;
                }
                ui.add_space(18.0);
                ui.label("Слушать на");
                let before = app.settings.host.clone();
                ui.radio_value(
                    &mut app.settings.host,
                    "0.0.0.0".to_string(),
                    "все интерфейсы (нужно для телефонов)",
                );
                ui.radio_value(
                    &mut app.settings.host,
                    "127.0.0.1".to_string(),
                    "только этот компьютер",
                );
                if app.settings.host != before {
                    app.dirty = true;
                }
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Адрес в ссылке");
                let before = app.settings.link_ip.clone();
                ui.add(
                    egui::TextEdit::singleline(&mut app.settings.link_ip)
                        .desired_width(170.0)
                        .hint_text("авто"),
                );
                if app.settings.link_ip != before {
                    app.dirty = true;
                }
                if ui.button("Определить").clicked() {
                    if let Some(ip) = platform::detect_lan_ip() {
                        app.settings.link_ip = ip.to_string();
                        app.dirty = true;
                    }
                }
            });
            palette::hint(
                ui,
                "Этот адрес попадает в «основную» ссылку и в значок в трее; на вкладке «Подключение» перечислены все адреса.",
            );
        });

        palette::section(ui, "Секрет", |ui| {
            ui.horizontal(|ui| {
                if app.show_secret {
                    let before = app.settings.secret.clone();
                    ui.add(
                        egui::TextEdit::singleline(&mut app.settings.secret)
                            .desired_width(330.0)
                            .font(egui::TextStyle::Monospace),
                    );
                    if app.settings.secret != before {
                        app.dirty = true;
                    }
                } else {
                    ui.label(
                        RichText::new("•".repeat(32))
                            .monospace()
                            .color(palette::MUTED),
                    );
                }
                if ui
                    .button(if app.show_secret { "Скрыть" } else { "Показать" })
                    .clicked()
                {
                    app.show_secret = !app.show_secret;
                }
                if ui.button("Сгенерировать новый").clicked() {
                    app.settings.regenerate_secret();
                    app.show_secret = true;
                    app.dirty = true;
                }
            });
            palette::hint(
                ui,
                "Новый секрет меняет ссылку: старую запись в Telegram придётся удалить и добавить заново.",
            );
        });

        palette::section(ui, "Дополнительные аргументы ядра", |ui| {
            let before = app.settings.extra_args.clone();
            ui.add(
                egui::TextEdit::multiline(&mut app.settings.extra_args)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace),
            );
            if app.settings.extra_args != before {
                app.dirty = true;
            }
            palette::hint(
                ui,
                "Рабочая связка для сетей, где режут TLS по SNI kws*: --default-domains --cf-balance --pinned-upstream cfproxy --cf-disable-tls",
            );
        });

        palette::section(ui, "Поведение", |ui| {
            let mut autostart = app.settings.autostart;
            if ui
                .checkbox(&mut autostart, "Запускать при входе в систему")
                .changed()
            {
                app.set_autostart(autostart);
            }

            let before = app.settings.verbose;
            ui.checkbox(&mut app.settings.verbose, "Подробный журнал (debug)");
            if app.settings.verbose != before {
                app.dirty = true;
                palette::hint(ui, "Уровень журнала применится после перезапуска приложения.");
            }
        });

        ui.add_space(2.0);
        ui.horizontal(|ui| {
            if ui.button("Сохранить").clicked() {
                app.save_settings();
            }
            if ui.button("Сохранить и перезапустить").clicked() {
                app.save_and_restart();
            }
            if app.dirty {
                ui.label(
                    RichText::new("есть несохранённые изменения")
                        .size(11.0)
                        .color(palette::WARN),
                );
            }
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button("Открыть config.json").clicked() {
                platform::open_path(&app.settings.config_path());
            }
            if ui.button("Открыть журнал").clicked() {
                platform::open_path(&app.settings.log_path());
            }
        });
    });
}

/// Tail of `proxy.log` — the same file the tray's "Открыть журнал" shows.
pub fn log(app: &mut App, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("Обновить").clicked() {
            let path = app.settings.log_path();
            app.log_text = super::read_tail(&path, 96 * 1024);
        }
        if ui.button("Открыть файл").clicked() {
            platform::open_path(&app.settings.log_path());
        }
        ui.checkbox(&mut app.autoscroll, "Прокручивать вниз");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            palette::hint(ui, "обновляется автоматически");
        });
    });

    ui.add_space(4.0);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(app.autoscroll)
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(RichText::new(&app.log_text).monospace().size(11.0)).wrap(),
            );
        });
}
