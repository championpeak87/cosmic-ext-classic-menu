use crate::fl;
use std::{string::String, sync::Arc};

use cosmic::desktop::DesktopEntryData;
use freedesktop_desktop_entry::DesktopEntry;

pub fn load_apps() -> Vec<Arc<DesktopEntryData>> {
    let locale = current_locale::current_locale().ok();
    let mut all_entries: Vec<Arc<DesktopEntryData>> =
        cosmic::desktop::load_applications(locale.as_slice(), false)
            .into_iter()
            .map(Arc::new)
            .collect();
    all_entries.sort_by(|a, b| a.name.cmp(&b.name));

    all_entries
}

pub fn get_comment(app: &Arc<DesktopEntryData>) -> Option<String> {
    if let Some(path) = &app.path {
        let locale = current_locale::current_locale().ok();
        let desktop_entry = DesktopEntry::from_path(path, Some(locale.as_slice()));

        if let Ok(entry) = desktop_entry {
            return Some(
                entry
                    .comment(locale.as_slice())
                    .unwrap_or_default()
                    .into_owned(),
            );
        }
    }

    None
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
    pub fn get_display_name(self) -> String {
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
}
