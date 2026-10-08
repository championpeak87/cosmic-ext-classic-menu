// SPDX-License-Identifier: GPL-3.0-only

use std::path::PathBuf;
use std::process;

use serde::{Deserialize, Serialize};

use crate::fl;
use crate::model::system_tool::SystemTool;

/// A shortcut to a folder or system tool, shown in place of the categories.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Place {
    Home,
    Documents,
    Downloads,
    Pictures,
    Music,
    Videos,
    Computer,
    Settings,
    Terminal,
    Store,
    SystemMonitor,
    Disks,
    /// A divider between places. Unlike the others, it may appear several
    /// times.
    Separator,
}

impl Place {
    /// Every place but [`Place::Separator`].
    pub const ALL: [Place; 12] = [
        Place::Home,
        Place::Documents,
        Place::Downloads,
        Place::Pictures,
        Place::Music,
        Place::Videos,
        Place::Computer,
        Place::Settings,
        Place::Terminal,
        Place::Store,
        Place::SystemMonitor,
        Place::Disks,
    ];

    /// Places shown until the user picks their own: folders first, then
    /// system apps.
    pub const DEFAULT: [Place; 10] = [
        Place::Home,
        Place::Documents,
        Place::Downloads,
        Place::Pictures,
        Place::Music,
        Place::Computer,
        Place::Separator,
        Place::Settings,
        Place::Terminal,
        Place::Store,
    ];

    pub fn display_name(self) -> String {
        match self {
            Place::Home => fl!("place-home"),
            Place::Documents => fl!("place-documents"),
            Place::Downloads => fl!("place-downloads"),
            Place::Pictures => fl!("place-pictures"),
            Place::Music => fl!("place-music"),
            Place::Videos => fl!("place-videos"),
            Place::Computer => fl!("place-computer"),
            Place::Settings => fl!("place-settings"),
            Place::Terminal => fl!("place-terminal"),
            Place::Store => fl!("place-store"),
            Place::SystemMonitor => fl!("place-system-monitor"),
            Place::Disks => fl!("place-disks"),
            Place::Separator => fl!("place-separator"),
        }
    }

    /// Name of the symbolic icon from the icon theme. Separators have none
    /// and are drawn as a line instead.
    pub fn icon_name(self) -> &'static str {
        match self {
            Place::Home => "user-home-symbolic",
            Place::Documents => "folder-documents-symbolic",
            Place::Downloads => "folder-download-symbolic",
            Place::Pictures => "folder-pictures-symbolic",
            Place::Music => "folder-music-symbolic",
            Place::Videos => "folder-videos-symbolic",
            Place::Computer => "computer-symbolic",
            Place::Settings => "preferences-system-symbolic",
            Place::Terminal => "utilities-terminal-symbolic",
            Place::Store => "system-software-install-symbolic",
            Place::SystemMonitor => "utilities-system-monitor-symbolic",
            Place::Disks => "drive-multidisk-symbolic",
            Place::Separator => "",
        }
    }

    /// The folder this place opens, if it is a folder.
    fn folder(self) -> Option<PathBuf> {
        match self {
            Place::Home => dirs::home_dir(),
            Place::Documents => dirs::document_dir(),
            Place::Downloads => dirs::download_dir(),
            Place::Pictures => dirs::picture_dir(),
            Place::Music => dirs::audio_dir(),
            Place::Videos => dirs::video_dir(),
            Place::Computer => Some(PathBuf::from("/")),
            _ => None,
        }
    }

    /// Whether the place can be opened here. Folders that are not set up
    /// on this system are hidden.
    pub fn is_available(self) -> bool {
        match self {
            Place::Home
            | Place::Documents
            | Place::Downloads
            | Place::Pictures
            | Place::Music
            | Place::Videos
            | Place::Computer => self.folder().is_some(),
            _ => true,
        }
    }

    pub fn open(self) {
        if let Some(folder) = self.folder() {
            spawn_on_host("xdg-open", &[&folder.to_string_lossy()]);
            return;
        }

        match self {
            Place::Settings => SystemTool::SYSTEM_SETTINGS.perform(),
            Place::Store => SystemTool::APP_STORE.perform(),
            Place::SystemMonitor => SystemTool::SYSTEM_MONITOR.perform(),
            Place::Disks => SystemTool::DISK_MANAGEMENT.perform(),
            Place::Terminal => {
                let command = terminal_command();
                let mut parts = command.split_whitespace();
                if let Some(program) = parts.next() {
                    spawn_on_host(program, &parts.collect::<Vec<_>>());
                }
            }
            _ => {}
        }
    }
}

/// The terminal set up in the COSMIC shortcuts, falling back to COSMIC's.
pub fn terminal_command() -> String {
    cosmic_settings_config::shortcuts::context()
        .ok()
        .and_then(|config| {
            cosmic_settings_config::shortcuts::system_actions(&config)
                .get(&cosmic_settings_config::shortcuts::action::System::Terminal)
                .cloned()
        })
        .unwrap_or_else(|| String::from("cosmic-term"))
}

/// Runs `program` outside of the Flatpak sandbox when inside one.
fn spawn_on_host(program: &str, args: &[&str]) {
    let result = if std::env::var("FLATPAK_ID").is_ok() {
        process::Command::new("flatpak-spawn")
            .arg("--host")
            .arg(program)
            .args(args)
            .spawn()
    } else {
        process::Command::new(program).args(args).spawn()
    };

    if let Err(e) = result {
        log::error!("Error launching '{program}': {e}");
    }
}
