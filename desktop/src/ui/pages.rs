//! The four tabs of the window, composed like the Android screen.
//!
//! Same order and same blocks as `ProxyScreen.kt`: status, link, arguments,
//! log, author tile, footer. The desktop adds two tabs the phone does not need
//! (one link per network interface, and settings), but they use the same
//! primitives, so the window still reads as one product with the phone app.
//!
//! Every tab is pure presentation: state changes go through the same `Engine`
//! methods the tray uses, never through a second proxy.

use eframe::egui::{self, RichText};

use super::theme as palette;
use super::{links, qr, App};
use crate::engine::State;
use crate::platform;

const CARD_GAP: f32 = 12.0;

pub fn home(app: &mut App, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            status_card(app, ui);
            ui.add_space(CARD_GAP);
            link_card(app, ui);
            ui.add_space(CARD_GAP);
            args_card(app, ui);
            ui.add_space(CARD_GAP);
            log_card(app, ui);
            ui.add_space(16.0);

            let width = ui.available_width();
            if palette::author_tile(ui, "разработчик", "painhahaha", width).clicked() {
                platform::open_url(palette::CARD_URL);
                app.note = "Открываю визитку".into();
            }

            ui.add_space(6.0);
            palette::footer(ui, "painhubproxy · 2026");
            ui.add_space(12.0);
        });
}

fn status_card(app: &mut App, ui: &mut egui::Ui) {
    palette::card(ui, |ui| {
        let inner = ui.available_width();
        let state = app.engine.state();
        let running = matches!(state, State::Running);

        let (title, sub, danger) = match &state {
            State::Running => (
                "Работает".to_string(),
                format!("слушает {}:{}", app.primary_ip(), app.settings.port),
                false,
            ),
            State::Starting => ("Запуск…".to_string(), "поднимаю listener".to_string(), false),
            State::Stopped => (
                "Остановлен".to_string(),
                "нажмите «Запустить», чтобы поднять прокси".to_string(),
                false,
            ),
            State::Failed(reason) => ("Ошибка".to_string(), reason.clone(), true),
        };

        ui.horizontal(|ui| {
            palette::status_dot(ui, running);
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(title).size(16.0).strong().color(palette::INK));
                if danger {
                    ui.label(RichText::new(sub).size(11.5).color(palette::DANGER));
                } else {
                    palette::hint(ui, &sub);
                }
            });
        });

        ui.add_space(14.0);
        let active = app.engine.is_active();
        let label = if active { "Остановить" } else { "Запустить" };
        if palette::primary_button(ui, label, inner, true).clicked() {
            app.toggle();
        }

        if app.engine.link().is_some() {
            ui.add_space(8.0);
            if palette::ghost_button(ui, "Открыть ссылку в Telegram", inner).clicked() {
                platform::open_url(&app.primary_link());
                app.note = "Открываю ссылку в Telegram".into();
            }
        }
    });
}

fn link_card(app: &mut App, ui: &mut egui::Ui) {
    palette::card(ui, |ui| {
        let inner = ui.available_width();
        palette::caption(ui, "Ссылка для телефона");
        ui.add_space(8.0);

        let link = app.primary_link();
        palette::mono(ui, &link, 10.5);
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            if let Some(matrix) = app.matrix(&link) {
                qr::draw(ui, matrix, 148.0);
            } else {
                palette::hint(ui, "QR не построен");
            }

            ui.add_space(16.0);
            ui.vertical(|ui| {
                latency(app, ui);
                ui.add_space(8.0);
                palette::hint(
                    ui,
                    "Секрет привязан к этой установке, поэтому ссылку достаточно добавить один раз.",
                );
                ui.add_space(6.0);
                palette::hint(
                    ui,
                    "Телефон должен быть в той же Wi-Fi. Другие адреса — на вкладке «Подключение».",
                );
            });
        });

        ui.add_space(12.0);
        let half = ((inner - 8.0) / 2.0).max(120.0);
        ui.horizontal(|ui| {
            if palette::ghost_button(ui, "Копировать", half).clicked() {
                platform::copy_text(&link);
                app.note = "Ссылка скопирована в буфер обмена".into();
            }
            if palette::ghost_button(ui, "Открыть в Telegram", half).clicked() {
                platform::open_url(&link);
            }
        });
    });
}

/// Round trip to the Cloudflare edge the core dials, measured by `probe`.
///
/// Deliberately labelled: this is the leg this machine pays, not the number
/// Telegram shows, which also includes the phone's Wi-Fi hop and the CF→DC hop.
fn latency(app: &App, ui: &mut egui::Ui) {
    match app.prober.best() {
        Some(sample) => {
            let color = if sample.ms < 90.0 {
                palette::OK
            } else if sample.ms < 220.0 {
                palette::INK
            } else {
                palette::DANGER
            };
            ui.label(
                RichText::new(format!("{:.0} мс до Cloudflare", sample.ms))
                    .size(13.0)
                    .color(color),
            );
            palette::hint(
                ui,
                &format!(
                    "{}:{} · ответили {}/{} адресов · {:.0} с назад",
                    sample.target,
                    sample.port,
                    sample.answered,
                    sample.total,
                    sample.at.elapsed().as_secs_f64()
                ),
            );
            if sample.answered < sample.total {
                palette::hint(ui, "часть адресов из --cf-ip не ответила");
            }
        }
        None => {
            if app.settings.cf_ips().is_empty() {
                palette::hint(ui, "Задержка не измеряется: в аргументах нет --cf-ip");
            } else {
                palette::hint(ui, "Измеряю задержку…");
            }
        }
    }
}

fn args_card(app: &mut App, ui: &mut egui::Ui) {
    palette::card(ui, |ui| {
        let inner = ui.available_width();
        palette::caption(ui, "Параметры запуска");
        ui.add_space(8.0);

        let before = app.settings.extra_args.clone();
        ui.add(
            egui::TextEdit::multiline(&mut app.settings.extra_args)
                .desired_rows(3)
                .desired_width(f32::INFINITY)
                .font(egui::TextStyle::Monospace)
                .text_color(palette::INK),
        );
        if app.settings.extra_args != before {
            app.dirty = true;
        }

        ui.add_space(8.0);
        let half = ((inner - 8.0) / 2.0).max(120.0);
        ui.horizontal(|ui| {
            if palette::ghost_button(ui, "Сохранить", half).clicked() {
                app.save_settings();
            }
            if palette::ghost_button(ui, "Сохранить и перезапустить", half).clicked() {
                app.save_and_restart();
            }
        });

        if app.dirty {
            ui.add_space(6.0);
            ui.label(
                RichText::new("есть несохранённые изменения")
                    .size(11.0)
                    .color(palette::DANGER),
            );
        }
    });
}

fn log_card(app: &mut App, ui: &mut egui::Ui) {
    palette::card(ui, |ui| {
        let inner = ui.available_width();
        palette::caption(ui, "Журнал");
        ui.add_space(8.0);

        egui::ScrollArea::vertical()
            .id_salt("log-tail")
            .max_height(240.0)
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.set_width(inner);
                ui.add(
                    egui::Label::new(
                        RichText::new(&app.log_text)
                            .monospace()
                            .size(10.0)
                            .color(palette::INK),
                    )
                    .wrap(),
                );
            });
    });
}

/// Every address the machine can be reached at, each with its own link and QR.
pub fn devices(app: &mut App, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            palette::hint(
                ui,
                "Ссылка зависит от адреса, по которому телефон видит компьютер: одна сеть — одна ссылка. Разные строки появляются, когда интерфейсов несколько (Wi-Fi и Ethernet, VPN).",
            );
            ui.add_space(10.0);

            let total = app.endpoints.len();
            for index in 0..total {
                let (ip, label, note) = {
                    let endpoint = &app.endpoints[index];
                    (
                        endpoint.ip.clone(),
                        endpoint.label(),
                        endpoint.kind.hint().to_string(),
                    )
                };
                let link = links::build_link(&ip, app.settings.port, &app.settings.secret);

                palette::card(ui, |ui| {
                    palette::caption(ui, &format!("{label} · {ip}"));
                    ui.add_space(6.0);
                    palette::hint(ui, &note);
                    ui.add_space(8.0);
                    palette::mono(ui, &link, 10.0);
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if let Some(matrix) = app.matrix(&link) {
                            qr::draw(ui, matrix, 124.0);
                        }
                        ui.add_space(14.0);
                        ui.vertical(|ui| {
                            if palette::ghost_button(ui, "Копировать", 170.0).clicked() {
                                platform::copy_text(&link);
                                app.note = format!("Ссылка {ip} скопирована");
                            }
                            ui.add_space(6.0);
                            if palette::ghost_button(ui, "Открыть в Telegram", 170.0).clicked() {
                                platform::open_url(&link);
                            }
                        });
                    });
                });
                ui.add_space(CARD_GAP);
            }

            if total <= 1 {
                palette::hint(
                    ui,
                    "Кроме loopback других адресов не найдено: сеть не поднята или интерфейсы отключены.",
                );
            }
            ui.add_space(8.0);
        });
}

/// Everything the core accepts, grouped the way the phone app groups it.
pub fn settings(app: &mut App, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            palette::card(ui, |ui| {
                palette::caption(ui, "Сеть");
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Порт").size(13.0).color(palette::INK));
                    let mut port = app.settings.port;
                    if ui
                        .add(egui::DragValue::new(&mut port).range(1..=65535).speed(1.0))
                        .changed()
                    {
                        app.settings.port = port;
                        app.dirty = true;
                    }
                });

                ui.add_space(6.0);
                palette::hint(ui, "Слушать на");
                let before = app.settings.host.clone();
                ui.radio_value(
                    &mut app.settings.host,
                    "0.0.0.0".to_string(),
                    "все интерфейсы — нужно для телефонов",
                );
                ui.radio_value(
                    &mut app.settings.host,
                    "127.0.0.1".to_string(),
                    "только этот компьютер",
                );
                if app.settings.host != before {
                    app.dirty = true;
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Адрес в ссылке").size(13.0).color(palette::INK));
                    let before = app.settings.link_ip.clone();
                    ui.add(
                        egui::TextEdit::singleline(&mut app.settings.link_ip)
                            .desired_width(160.0)
                            .hint_text("авто"),
                    );
                    if app.settings.link_ip != before {
                        app.dirty = true;
                    }
                    if palette::ghost_button(ui, "Определить", 120.0).clicked() {
                        if let Some(ip) = platform::detect_lan_ip() {
                            app.settings.link_ip = ip.to_string();
                            app.dirty = true;
                        }
                    }
                });
                ui.add_space(6.0);
                palette::hint(
                    ui,
                    "Этот адрес попадает в основную ссылку и в значок трея; на вкладке «Подключение» перечислены все адреса.",
                );
            });

            ui.add_space(CARD_GAP);
            palette::card(ui, |ui| {
                let inner = ui.available_width();
                palette::caption(ui, "Секрет");
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if app.show_secret {
                        let before = app.settings.secret.clone();
                        ui.add(
                            egui::TextEdit::singleline(&mut app.settings.secret)
                                .desired_width(320.0)
                                .font(egui::TextStyle::Monospace),
                        );
                        if app.settings.secret != before {
                            app.dirty = true;
                        }
                    } else {
                        ui.label(
                            RichText::new("•".repeat(32))
                                .monospace()
                                .size(12.0)
                                .color(palette::MUTED),
                        );
                    }
                });

                ui.add_space(8.0);
                let half = ((inner - 8.0) / 2.0).max(120.0);
                ui.horizontal(|ui| {
                    let label = if app.show_secret { "Скрыть" } else { "Показать" };
                    if palette::ghost_button(ui, label, half).clicked() {
                        app.show_secret = !app.show_secret;
                    }
                    if palette::ghost_button(ui, "Сгенерировать новый", half).clicked() {
                        app.settings.regenerate_secret();
                        app.show_secret = true;
                        app.dirty = true;
                    }
                });

                ui.add_space(8.0);
                palette::hint(
                    ui,
                    "Новый секрет меняет ссылку: старую запись в Telegram придётся удалить и добавить заново.",
                );
            });

            ui.add_space(CARD_GAP);
            palette::card(ui, |ui| {
                palette::caption(ui, "Поведение");
                ui.add_space(10.0);

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
                    palette::hint(ui, "уровень журнала применится после перезапуска приложения");
                }

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if palette::ghost_button(ui, "Открыть config.json", 190.0).clicked() {
                        platform::open_path(&app.settings.config_path());
                    }
                    if palette::ghost_button(ui, "Открыть журнал", 170.0).clicked() {
                        platform::open_path(&app.settings.log_path());
                    }
                });
            });

            ui.add_space(8.0);
            palette::hint(
                ui,
                "Настройки сохраняются в config.json. «Сохранить и перезапустить» на вкладке «Обзор» применяет их сразу.",
            );
            ui.add_space(12.0);
        });
}

/// The log full-height, for when the 240 px tail on the overview is not enough.
pub fn log(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if palette::ghost_button(ui, "Обновить", 120.0).clicked() {
            let path = app.settings.log_path();
            app.log_text = super::read_tail(&path, 96 * 1024);
        }
        if palette::ghost_button(ui, "Открыть файл", 150.0).clicked() {
            platform::open_path(&app.settings.log_path());
        }
        ui.checkbox(&mut app.autoscroll, "Прокручивать вниз");
    });

    ui.add_space(8.0);
    palette::card(ui, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(app.autoscroll)
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(&app.log_text)
                            .monospace()
                            .size(10.5)
                            .color(palette::INK),
                    )
                    .wrap()
                    .selectable(true),
                );
            });
    });
}
