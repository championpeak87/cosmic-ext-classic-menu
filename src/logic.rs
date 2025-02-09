use std::sync::Arc;
use crate::fl;

use cosmic::desktop::DesktopEntryData;

pub fn load_apps() -> Vec<Arc<DesktopEntryData>> {
    let locale = current_locale::current_locale().ok();
    let xdg_current_desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
    let mut all_entries: Vec<Arc<DesktopEntryData>> =
        cosmic::desktop::load_applications_filtered(locale.as_deref(), |entry| {
            entry.exec().is_some()
                && !entry.no_display()
                && xdg_current_desktop
                    .as_deref()
                    .as_ref()
                    .zip(entry.only_show_in())
                    .map(|(xdg_current_desktop, only_show_in)| {
                        only_show_in.contains(xdg_current_desktop)
                    })
                    .unwrap_or(true)
        })
        .into_iter()
        .map(Arc::new)
        .collect();
    all_entries.sort_by(|a, b| a.name.cmp(&b.name));

    all_entries
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ApplicationCategory {
    All,
    RecentlyUsed,
    Audio,
    Video,
    Development,
    Games,
    Graphics,
    Network,
    Office,
    Science,
    Settings,
    System,
    Utility,
}

impl ApplicationCategory {
    pub fn get_name(self) -> String {
        match self {
            ApplicationCategory::All => fl!("all-applications"),
            ApplicationCategory::RecentlyUsed => fl!("recently-used"),
            ApplicationCategory::Audio => fl!("audio"),
            ApplicationCategory::Video => fl!("video"),
            ApplicationCategory::Development => fl!("development"),
            ApplicationCategory::Games => fl!("games"),
            ApplicationCategory::Graphics => fl!("graphics"),
            ApplicationCategory::Network => fl!("network"),
            ApplicationCategory::Office => fl!("office"),
            ApplicationCategory::Science => fl!("science"),
            ApplicationCategory::Settings => fl!("settings"),
            ApplicationCategory::System => fl!("system"),
            ApplicationCategory::Utility => fl!("utility"),
        }
    }

    pub fn get_icon_name(self) -> &'static str {
        match self {
            ApplicationCategory::All => "open-menu-symbolic",
            ApplicationCategory::RecentlyUsed => "document-open-recent-symbolic",
            ApplicationCategory::Audio => "applications-audio-symbolic",
            ApplicationCategory::Video => "applications-video-symbolic",
            ApplicationCategory::Development => "applications-engineering-symbolic",
            ApplicationCategory::Games => "applications-games-symbolic",
            ApplicationCategory::Graphics => "applications-graphics-symbolic",
            ApplicationCategory::Network => "network-workgroup-symbolic",
            ApplicationCategory::Office => "applications-office-symbolic",
            ApplicationCategory::Science => "applications-science-symbolic",
            ApplicationCategory::Settings => "preferences-system-symbolic",
            ApplicationCategory::System => "applications-system-symbolic",
            ApplicationCategory::Utility => "applications-utilities-symbolic",
        }
    }

    pub fn get_mime_name(self) -> &'static str {
        match self {
            ApplicationCategory::All => "",
            ApplicationCategory::RecentlyUsed => "",
            ApplicationCategory::Audio => "Audio",
            ApplicationCategory::Video => "Video",
            ApplicationCategory::Development => "Development",
            ApplicationCategory::Games => "Game",
            ApplicationCategory::Graphics => "Graphics",
            ApplicationCategory::Network => "Network",
            ApplicationCategory::Office => "Office",
            ApplicationCategory::Science => "Science",
            ApplicationCategory::Settings => "Settings",
            ApplicationCategory::System => "System",
            ApplicationCategory::Utility => "Utility",
        }
    }

    pub fn into_iter() -> core::array::IntoIter<ApplicationCategory, 13> {
        [
            ApplicationCategory::All,
            ApplicationCategory::RecentlyUsed,
            ApplicationCategory::Audio,
            ApplicationCategory::Video,
            ApplicationCategory::Development,
            ApplicationCategory::Games,
            ApplicationCategory::Graphics,
            ApplicationCategory::Network,
            ApplicationCategory::Office,
            ApplicationCategory::Science,
            ApplicationCategory::Settings,
            ApplicationCategory::System,
            ApplicationCategory::Utility,
        ]
        .into_iter()
    }
}
