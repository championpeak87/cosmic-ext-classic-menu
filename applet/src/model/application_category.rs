use std::{borrow::Cow, fmt::Display, path::PathBuf};

use crate::{fl, model::application_entry::ApplicationEntry};

#[derive(Clone, Debug, PartialEq)]
pub enum CategoryIcon {
    /// Bundled symbolic SVG icon.
    Bundled(Cow<'static, [u8]>),
    /// Icon looked up by name in the icon theme.
    Named(String),
    /// Icon loaded from an absolute path.
    Path(PathBuf),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApplicationCategory {
    pub display_name: Cow<'static, str>,
    pub icon: CategoryIcon,
    pub mime_name: Cow<'static, str>,
    pub permanent: bool,
    /// Desktop entry categories included in a custom category.
    pub include_categories: Vec<String>,
    /// Desktop file ids included in a custom category.
    pub include_filenames: Vec<String>,
}

impl ApplicationCategory {
    pub const ALL: ApplicationCategory = ApplicationCategory {
        display_name: Cow::Borrowed("all-applications"),
        icon: CategoryIcon::Bundled(Cow::Borrowed(include_bytes!(
            "../../../res/icons/bundled/open-menu-symbolic.svg"
        ))),
        mime_name: Cow::Borrowed(""),
        permanent: true,
        include_categories: Vec::new(),
        include_filenames: Vec::new(),
    };
    pub const RECENTLY_USED: ApplicationCategory = ApplicationCategory {
        display_name: Cow::Borrowed("recently-used"),
        icon: CategoryIcon::Bundled(Cow::Borrowed(include_bytes!(
            "../../../res/icons/bundled/document-open-recent-symbolic.svg"
        ))),
        mime_name: Cow::Borrowed(""),
        permanent: true,
        include_categories: Vec::new(),
        include_filenames: Vec::new(),
    };
    pub const AUDIO: ApplicationCategory = ApplicationCategory {
        display_name: Cow::Borrowed("audio"),
        icon: CategoryIcon::Bundled(Cow::Borrowed(include_bytes!(
            "../../../res/icons/bundled/applications-audio-symbolic.svg"
        ))),
        mime_name: Cow::Borrowed("Audio"),
        permanent: false,
        include_categories: Vec::new(),
        include_filenames: Vec::new(),
    };
    pub const VIDEO: ApplicationCategory = ApplicationCategory {
        display_name: Cow::Borrowed("video"),
        icon: CategoryIcon::Bundled(Cow::Borrowed(include_bytes!(
            "../../../res/icons/bundled/applications-video-symbolic.svg"
        ))),
        mime_name: Cow::Borrowed("Video"),
        permanent: false,
        include_categories: Vec::new(),
        include_filenames: Vec::new(),
    };
    pub const DEVELOPMENT: ApplicationCategory = ApplicationCategory {
        display_name: Cow::Borrowed("development"),
        icon: CategoryIcon::Bundled(Cow::Borrowed(include_bytes!(
            "../../../res/icons/bundled/applications-engineering-symbolic.svg"
        ))),
        mime_name: Cow::Borrowed("Development"),
        permanent: false,
        include_categories: Vec::new(),
        include_filenames: Vec::new(),
    };
    pub const GAMES: ApplicationCategory = ApplicationCategory {
        display_name: Cow::Borrowed("games"),
        icon: CategoryIcon::Bundled(Cow::Borrowed(include_bytes!(
            "../../../res/icons/bundled/applications-games-symbolic.svg"
        ))),
        mime_name: Cow::Borrowed("Game"),
        permanent: false,
        include_categories: Vec::new(),
        include_filenames: Vec::new(),
    };
    pub const GRAPHICS: ApplicationCategory = ApplicationCategory {
        display_name: Cow::Borrowed("graphics"),
        icon: CategoryIcon::Bundled(Cow::Borrowed(include_bytes!(
            "../../../res/icons/bundled/applications-graphics-symbolic.svg"
        ))),
        mime_name: Cow::Borrowed("Graphics"),
        permanent: false,
        include_categories: Vec::new(),
        include_filenames: Vec::new(),
    };
    pub const NETWORK: ApplicationCategory = ApplicationCategory {
        display_name: Cow::Borrowed("network"),
        icon: CategoryIcon::Bundled(Cow::Borrowed(include_bytes!(
            "../../../res/icons/bundled/network-workgroup-symbolic.svg"
        ))),
        mime_name: Cow::Borrowed("Network"),
        permanent: false,
        include_categories: Vec::new(),
        include_filenames: Vec::new(),
    };
    pub const OFFICE: ApplicationCategory = ApplicationCategory {
        display_name: Cow::Borrowed("office"),
        icon: CategoryIcon::Bundled(Cow::Borrowed(include_bytes!(
            "../../../res/icons/bundled/applications-office-symbolic.svg"
        ))),
        mime_name: Cow::Borrowed("Office"),
        permanent: false,
        include_categories: Vec::new(),
        include_filenames: Vec::new(),
    };
    pub const SCIENCE: ApplicationCategory = ApplicationCategory {
        display_name: Cow::Borrowed("science"),
        icon: CategoryIcon::Bundled(Cow::Borrowed(include_bytes!(
            "../../../res/icons/bundled/applications-science-symbolic.svg"
        ))),
        mime_name: Cow::Borrowed("Science"),
        permanent: false,
        include_categories: Vec::new(),
        include_filenames: Vec::new(),
    };
    pub const SETTINGS: ApplicationCategory = ApplicationCategory {
        display_name: Cow::Borrowed("settings"),
        icon: CategoryIcon::Bundled(Cow::Borrowed(include_bytes!(
            "../../../res/icons/bundled/preferences-system-symbolic.svg"
        ))),
        mime_name: Cow::Borrowed("Settings"),
        permanent: false,
        include_categories: Vec::new(),
        include_filenames: Vec::new(),
    };
    pub const SYSTEM: ApplicationCategory = ApplicationCategory {
        display_name: Cow::Borrowed("system"),
        icon: CategoryIcon::Bundled(Cow::Borrowed(include_bytes!(
            "../../../res/icons/bundled/applications-system-symbolic.svg"
        ))),
        mime_name: Cow::Borrowed("System"),
        permanent: false,
        include_categories: Vec::new(),
        include_filenames: Vec::new(),
    };
    pub const UTILITY: ApplicationCategory = ApplicationCategory {
        display_name: Cow::Borrowed("utility"),
        icon: CategoryIcon::Bundled(Cow::Borrowed(include_bytes!(
            "../../../res/icons/bundled/applications-utilities-symbolic.svg"
        ))),
        mime_name: Cow::Borrowed("Utility"),
        permanent: false,
        include_categories: Vec::new(),
        include_filenames: Vec::new(),
    };

    pub fn get_display_name(&self) -> String {
        match self.display_name {
            std::borrow::Cow::Borrowed("all-applications") => fl!("all-applications"),
            std::borrow::Cow::Borrowed("recently-used") => fl!("recently-used"),
            std::borrow::Cow::Borrowed("audio") => fl!("audio"),
            std::borrow::Cow::Borrowed("video") => fl!("video"),
            std::borrow::Cow::Borrowed("development") => fl!("development"),
            std::borrow::Cow::Borrowed("games") => fl!("games"),
            std::borrow::Cow::Borrowed("graphics") => fl!("graphics"),
            std::borrow::Cow::Borrowed("network") => fl!("network"),
            std::borrow::Cow::Borrowed("office") => fl!("office"),
            std::borrow::Cow::Borrowed("science") => fl!("science"),
            std::borrow::Cow::Borrowed("settings") => fl!("settings"),
            std::borrow::Cow::Borrowed("system") => fl!("system"),
            std::borrow::Cow::Borrowed("utility") => fl!("utility"),
            _ => self.display_name.to_string(),
        }
    }

    /// Returns true if the application belongs to this category.
    pub fn matches(&self, app: &ApplicationEntry) -> bool {
        if self.include_categories.is_empty() && self.include_filenames.is_empty() {
            !self.mime_name.is_empty() && app.category.iter().any(|c| *c == self.mime_name)
        } else {
            app.category.iter().any(|c| self.include_categories.contains(c))
                || self.include_filenames.contains(&app.desktop_file_id)
        }
    }
}

impl Display for ApplicationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.mime_name)
    }
}
