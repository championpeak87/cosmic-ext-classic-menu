// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

use crate::fl;
use crate::model::place::Place;
use crate::model::system_tool::SystemTool;

/// An entry of the menu opened by right clicking the applet's button,
/// modeled on the one of the Windows start button.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ContextMenuItem {
    /// The installed apps, in the system settings.
    Applications,
    PowerSettings,
    /// About this system, in the system settings.
    System,
    Network,
    Bluetooth,
    Sound,
    Displays,
    Appearance,
    Disks,
    Terminal,
    SystemMonitor,
    SystemSettings,
    /// The home folder in the file manager.
    Files,
    /// The COSMIC launcher.
    Search,
    Store,
    /// This applet's settings.
    MenuSettings,
    /// A row of power buttons, see
    /// [`crate::config::AppletConfig::context_menu_power`].
    Power,
    /// A divider between entries. Unlike the others, it may appear several
    /// times.
    Separator,
}

impl ContextMenuItem {
    /// Every entry but the separator, see [`ContextMenuItem::is_repeatable`].
    pub const ALL: [ContextMenuItem; 17] = [
        ContextMenuItem::Applications,
        ContextMenuItem::PowerSettings,
        ContextMenuItem::System,
        ContextMenuItem::Network,
        ContextMenuItem::Bluetooth,
        ContextMenuItem::Sound,
        ContextMenuItem::Displays,
        ContextMenuItem::Appearance,
        ContextMenuItem::Disks,
        ContextMenuItem::Terminal,
        ContextMenuItem::SystemMonitor,
        ContextMenuItem::SystemSettings,
        ContextMenuItem::Files,
        ContextMenuItem::Search,
        ContextMenuItem::Store,
        ContextMenuItem::MenuSettings,
        ContextMenuItem::Power,
    ];

    /// Entries shown until the user picks their own, in the order of the
    /// Windows menu.
    pub const DEFAULT: [ContextMenuItem; 16] = [
        ContextMenuItem::Applications,
        ContextMenuItem::PowerSettings,
        ContextMenuItem::System,
        ContextMenuItem::Network,
        ContextMenuItem::Disks,
        ContextMenuItem::Separator,
        ContextMenuItem::Terminal,
        ContextMenuItem::Separator,
        ContextMenuItem::SystemMonitor,
        ContextMenuItem::SystemSettings,
        ContextMenuItem::Files,
        ContextMenuItem::Search,
        ContextMenuItem::Separator,
        ContextMenuItem::MenuSettings,
        ContextMenuItem::Separator,
        ContextMenuItem::Power,
    ];

    /// Whether the entry may appear several times, as it opens nothing.
    pub fn is_repeatable(self) -> bool {
        self == ContextMenuItem::Separator
    }

    pub fn display_name(self) -> String {
        match self {
            ContextMenuItem::Applications => fl!("context-applications"),
            ContextMenuItem::PowerSettings => fl!("context-power-settings"),
            ContextMenuItem::System => fl!("context-system"),
            ContextMenuItem::Network => fl!("context-network"),
            ContextMenuItem::Bluetooth => fl!("context-bluetooth"),
            ContextMenuItem::Sound => fl!("context-sound"),
            ContextMenuItem::Displays => fl!("context-displays"),
            ContextMenuItem::Appearance => fl!("context-appearance"),
            ContextMenuItem::Disks => fl!("disks-label"),
            ContextMenuItem::Terminal => fl!("place-terminal"),
            ContextMenuItem::SystemMonitor => fl!("system-monitor-label"),
            ContextMenuItem::SystemSettings => fl!("settings-label"),
            ContextMenuItem::Files => fl!("context-files"),
            ContextMenuItem::Search => fl!("context-search"),
            ContextMenuItem::Store => fl!("place-store"),
            ContextMenuItem::MenuSettings => fl!("context-menu-settings"),
            ContextMenuItem::Power => fl!("context-power"),
            ContextMenuItem::Separator => fl!("place-separator"),
        }
    }

    /// Name of the symbolic icon from the icon theme, for the settings. The
    /// separator has none, being drawn as a line.
    pub fn icon_name(self) -> &'static str {
        match self {
            ContextMenuItem::Applications => "view-app-grid-symbolic",
            ContextMenuItem::PowerSettings => "battery-good-symbolic",
            ContextMenuItem::System => "help-about-symbolic",
            ContextMenuItem::Network => "network-workgroup-symbolic",
            ContextMenuItem::Bluetooth => "bluetooth-active-symbolic",
            ContextMenuItem::Sound => "audio-volume-high-symbolic",
            ContextMenuItem::Displays => "video-display-symbolic",
            ContextMenuItem::Appearance => "preferences-desktop-appearance-symbolic",
            ContextMenuItem::Disks => Place::Disks.icon_name(),
            ContextMenuItem::Terminal => Place::Terminal.icon_name(),
            ContextMenuItem::SystemMonitor => Place::SystemMonitor.icon_name(),
            ContextMenuItem::SystemSettings => Place::Settings.icon_name(),
            ContextMenuItem::Files => "system-file-manager-symbolic",
            ContextMenuItem::Search => "system-search-symbolic",
            ContextMenuItem::Store => Place::Store.icon_name(),
            ContextMenuItem::MenuSettings => "open-menu-symbolic",
            ContextMenuItem::Power => "system-shutdown-symbolic",
            ContextMenuItem::Separator => "",
        }
    }

    /// Opens what the entry stands for. The power buttons and the separator
    /// open nothing themselves.
    pub fn open(self) {
        let settings_page = |page: &'static str| SystemTool::command(page).perform();
        match self {
            ContextMenuItem::Applications => settings_page("cosmic-settings applications"),
            ContextMenuItem::PowerSettings => settings_page("cosmic-settings power"),
            ContextMenuItem::System => settings_page("cosmic-settings about"),
            ContextMenuItem::Network => settings_page("cosmic-settings network"),
            ContextMenuItem::Bluetooth => settings_page("cosmic-settings bluetooth"),
            ContextMenuItem::Sound => settings_page("cosmic-settings sound"),
            ContextMenuItem::Displays => settings_page("cosmic-settings displays"),
            ContextMenuItem::Appearance => settings_page("cosmic-settings appearance"),
            ContextMenuItem::Disks => Place::Disks.open(),
            ContextMenuItem::Terminal => Place::Terminal.open(),
            ContextMenuItem::SystemMonitor => Place::SystemMonitor.open(),
            ContextMenuItem::SystemSettings => Place::Settings.open(),
            // The default file manager.
            ContextMenuItem::Files => Place::Home.open(),
            ContextMenuItem::Search => SystemTool::command("cosmic-launcher").perform(),
            ContextMenuItem::Store => Place::Store.open(),
            ContextMenuItem::MenuSettings => SystemTool::APPLET_SETTINGS.perform(),
            ContextMenuItem::Power | ContextMenuItem::Separator => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ContextMenuItem;

    #[test]
    fn only_separators_repeat() {
        assert!(ContextMenuItem::Separator.is_repeatable());
        assert!(ContextMenuItem::ALL.iter().all(|item| !item.is_repeatable()));
    }

    #[test]
    fn default_entries_appear_once() {
        for item in ContextMenuItem::ALL {
            let count = ContextMenuItem::DEFAULT.iter().filter(|i| **i == item).count();
            assert!(count <= 1, "{item:?} appears {count} times");
        }
    }

    #[test]
    fn entries_survive_the_config() {
        let items = ContextMenuItem::DEFAULT.to_vec();
        let ron = ron::to_string(&items).unwrap();
        assert_eq!(ron::from_str::<Vec<ContextMenuItem>>(&ron).unwrap(), items);
    }
}
