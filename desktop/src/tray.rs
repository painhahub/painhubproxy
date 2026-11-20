//! Tray icon and its menu.
//!
//! Events are not handled inline: `muda`/`tray-icon` invoke their handlers from
//! inside the Windows message dispatch, so they are forwarded over a channel and
//! acted on once the pump returns. That keeps start/stop out of a re-entrant
//! callback.

use std::collections::HashMap;
use std::sync::mpsc::Sender;

use muda::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::engine::State;
use crate::platform;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    CopyLink,
    OpenTelegram,
    Toggle,
    Restart,
    OpenLog,
    OpenConfig,
    ToggleAutostart,
    Quit,
}

pub struct Tray {
    icon: TrayIcon,
    status: MenuItem,
    toggle: MenuItem,
    autostart: CheckMenuItem,
}

impl Tray {
    pub fn new(actions: Sender<Action>, autostart: bool) -> Option<Self> {
        let status = MenuItem::new("Запуск…", false, None);
        let copy = MenuItem::new("Копировать ссылку", true, None);
        let open = MenuItem::new("Открыть в Telegram", true, None);
        let toggle = MenuItem::new("Остановить", true, None);
        let restart = MenuItem::new("Перезапустить", true, None);
        let log = MenuItem::new("Открыть журнал", true, None);
        let config = MenuItem::new("Открыть настройки", true, None);
        let autostart_item = CheckMenuItem::new("Автозапуск при входе", true, autostart, None);
        let quit = MenuItem::new("Выход", true, None);

        let menu = Menu::new();
        menu.append_items(&[
            &status,
            &PredefinedMenuItem::separator(),
            &copy,
            &open,
            &PredefinedMenuItem::separator(),
            &toggle,
            &restart,
            &PredefinedMenuItem::separator(),
            &log,
            &config,
            &autostart_item,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .ok()?;

        let mut ids: HashMap<MenuId, Action> = HashMap::new();
        ids.insert(copy.id().clone(), Action::CopyLink);
        ids.insert(open.id().clone(), Action::OpenTelegram);
        ids.insert(toggle.id().clone(), Action::Toggle);
        ids.insert(restart.id().clone(), Action::Restart);
        ids.insert(log.id().clone(), Action::OpenLog);
        ids.insert(config.id().clone(), Action::OpenConfig);
        ids.insert(autostart_item.id().clone(), Action::ToggleAutostart);
        ids.insert(quit.id().clone(), Action::Quit);

        let menu_actions = actions.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if let Some(action) = ids.get(&event.id) {
                let _ = menu_actions.send(*action);
            }
        }));

        // Left click is the shortest path to "use it": open the link in
        // Telegram. The menu stays on right click.
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = actions.send(Action::OpenTelegram);
            }
        }));

        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip("painhubproxy")
            .with_icon(load_icon()?)
            .build()
            .ok()?;

        Some(Self {
            icon,
            status,
            toggle,
            autostart: autostart_item,
        })
    }

    /// Push the current proxy state into the menu and the tooltip.
    pub fn refresh(&mut self, state: &State, link: Option<&str>) {
        let active = matches!(state, State::Running | State::Starting);
        self.toggle
            .set_text(if active { "Остановить" } else { "Запустить" });

        let address = link.and_then(platform::address_of_link);

        let status = match (&address, state) {
            (Some(address), _) => format!("{} — {address}", state.label()),
            (None, _) => state.label(),
        };
        self.status.set_text(status);

        // Windows truncates the tooltip at 128 UTF-16 units; an error message
        // can easily be longer than that.
        let tooltip = match (&address, state) {
            (Some(address), State::Running) => format!("painhubproxy — {address}"),
            _ => format!("painhubproxy — {}", state.label()),
        };
        let _ = self.icon.set_tooltip(Some(truncate(&tooltip, 120)));
    }

    pub fn set_autostart(&mut self, enabled: bool) {
        self.autostart.set_checked(enabled);
    }
}

fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    text.chars().take(limit.saturating_sub(1)).collect::<String>() + "…"
}

/// The app icon, decoded at runtime from the PNG the repository already ships
/// (`docs/icon.png`) so the tray never depends on a file next to the exe.
fn load_icon() -> Option<Icon> {
    const PNG: &[u8] = include_bytes!("../../docs/icon.png");
    const SIZE: u32 = 32;

    let source = image::load_from_memory(PNG).ok()?.to_rgba8();
    let scaled = image::imageops::resize(
        &source,
        SIZE,
        SIZE,
        image::imageops::FilterType::Lanczos3,
    );
    Icon::from_rgba(scaled.into_raw(), SIZE, SIZE).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooltip_is_truncated() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("abcdefghij", 5), "abcd…");
    }
}
