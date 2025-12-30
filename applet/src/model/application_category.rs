use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::fl;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApplicationCategory {
    pub display_name: &'static str,
    pub icon_svg_bytes: &'static [u8],
    pub mime_name: &'static str,
    pub permanent: bool,
}

impl ApplicationCategory {
    pub const ALL: ApplicationCategory = ApplicationCategory {
        display_name: "all-applications",
        icon_svg_bytes: include_bytes!("../../../res/icons/bundled/open-menu-symbolic.svg"),
        mime_name: "",
        permanent: true,
    };
    pub const RECENTLY_USED: ApplicationCategory = ApplicationCategory {
        display_name: "recently-used",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/document-open-recent-symbolic.svg"
        ),
        mime_name: "",
        permanent: true,
    };
    pub const AUDIO: ApplicationCategory = ApplicationCategory {
        display_name: "audio",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-audio-symbolic.svg"
        ),
        mime_name: "Audio",
        permanent: false,
    };
    pub const VIDEO: ApplicationCategory = ApplicationCategory {
        display_name: "video",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-video-symbolic.svg"
        ),
        mime_name: "Video",
        permanent: false,
    };
    pub const DEVELOPMENT: ApplicationCategory = ApplicationCategory {
        display_name: "development",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-engineering-symbolic.svg"
        ),
        mime_name: "Development",
        permanent: false,
    };
    pub const GAMES: ApplicationCategory = ApplicationCategory {
        display_name: "games",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-games-symbolic.svg"
        ),
        mime_name: "Game",
        permanent: false,
    };
    pub const GRAPHICS: ApplicationCategory = ApplicationCategory {
        display_name: "graphics",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-graphics-symbolic.svg"
        ),
        mime_name: "Graphics",
        permanent: false,
    };
    pub const NETWORK: ApplicationCategory = ApplicationCategory {
        display_name: "network",
        icon_svg_bytes: include_bytes!("../../../res/icons/bundled/network-workgroup-symbolic.svg"),
        mime_name: "Network",
        permanent: false,
    };
    pub const OFFICE: ApplicationCategory = ApplicationCategory {
        display_name: "office",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-office-symbolic.svg"
        ),
        mime_name: "Office",
        permanent: false,
    };
    pub const SCIENCE: ApplicationCategory = ApplicationCategory {
        display_name: "science",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-science-symbolic.svg"
        ),
        mime_name: "Science",
        permanent: false,
    };
    pub const SETTINGS: ApplicationCategory = ApplicationCategory {
        display_name: "settings",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/preferences-system-symbolic.svg"
        ),
        mime_name: "Settings",
        permanent: false,
    };
    pub const SYSTEM: ApplicationCategory = ApplicationCategory {
        display_name: "system",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-system-symbolic.svg"
        ),
        mime_name: "System",
        permanent: false,
    };
    pub const UTILITY: ApplicationCategory = ApplicationCategory {
        display_name: "utility",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-utilities-symbolic.svg"
        ),
        mime_name: "Utility",
        permanent: false,
    };

    pub fn get_display_name(&self) -> String {
        match self.display_name {
            "all-applications" => fl!("all-applications"),
            "recently-used" => fl!("recently-used"),
            "audio" => fl!("audio"),
            "video" => fl!("video"),
            "development" => fl!("development"),
            "games" => fl!("games"),
            "graphics" => fl!("graphics"),
            "network" => fl!("network"),
            "office" => fl!("office"),
            "science" => fl!("science"),
            "settings" => fl!("settings"),
            "system" => fl!("system"),
            "utility" => fl!("utility"),
            _ => self.display_name.to_string(),
        }
    }
}

impl Display for ApplicationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.mime_name)
    }
}
