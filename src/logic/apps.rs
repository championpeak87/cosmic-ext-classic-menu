use crate::{fl, model::application_entry::ApplicationEntry};
use std::{fmt::Display, string::String};

use serde::{Deserialize, Serialize};
use cached::{proc_macro::cached, UnboundCache};

use cosmic::{
    iced::{stream, Subscription},
    iced_futures::futures::{self, SinkExt},
};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::fmt::Debug;
use std::hash::Hash;
use tokio::sync::mpsc;

#[cached(
    name = "APPS_CACHE",
    ty = "UnboundCache<(), Vec<ApplicationEntry>>",
    create = "{ UnboundCache::new() }"
)]
pub fn load_apps() -> Vec<ApplicationEntry> {
    let locale = current_locale::current_locale().ok();
    let mut all_entries: Vec<ApplicationEntry> =
        cosmic::desktop::load_applications(locale.as_slice(), false, None)
            .into_iter()
            .map(Into::into)
            .collect();
    all_entries.sort_by(|a, b| a.name.cmp(&b.name));

    all_entries
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Changed,
}

pub fn desktop_files<I: 'static + Hash + Copy + Send + Sync + Debug>(
    id: I,
) -> cosmic::iced::Subscription<Event> {
    Subscription::run_with_id(
        id,
        stream::channel(50, move |mut output| async move {
            let handle = tokio::runtime::Handle::current();
            let (tx, mut rx) = mpsc::channel(4);
            let mut last_update = std::time::Instant::now();

            // Automatically select the best implementation for your platform.
            // You can also access each implementation directly e.g. INotifyWatcher.
            let watcher = RecommendedWatcher::new(
                move |res: Result<notify::Event, notify::Error>| {
                    if let Ok(event) = res {
                        match event.kind {
                            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                                let now = std::time::Instant::now();
                                if now.duration_since(last_update).as_secs() > 3 {
                                    _ = handle.block_on(tx.send(()));
                                    last_update = now;
                                }
                            }

                            _ => (),
                        }
                    }
                },
                Config::default(),
            );

            if let Ok(mut watcher) = watcher {
                for path in freedesktop_desktop_entry::default_paths() {
                    let _ = watcher.watch(path.as_ref(), RecursiveMode::Recursive);
                }

                while rx.recv().await.is_some() {
                    _ = output.send(Event::Changed).await;
                }
            }

            futures::future::pending().await
        }),
    )
}

pub async fn get_current_user() -> Result<User, zbus::Error> {
    let uid = users::get_current_uid() as u64;

    let conn = zbus::Connection::system().await?;
    let user = accounts_zbus::UserProxy::from_uid(&conn, uid).await?;

    // Fetch all fields concurrently
    let (username, user_realname, profile_picture, uid, user_home, user_shell) = tokio::join!(
        user.user_name(),
        user.real_name(),
        user.icon_file(),
        user.uid(),
        user.home_directory(),
        user.shell()
    );

    Ok(User {
        username: username?,
        user_realname: user_realname?,
        profile_picture: profile_picture?,
        uid: uid?,
        user_home: user_home?,
        user_shell: user_shell?,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub user_realname: String,
    pub profile_picture: String,
    pub uid: u64,
    pub user_home: String,
    pub user_shell: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApplicationCategory {
    pub display_name: &'static str,
    pub icon_name: &'static str,
    pub mime_name: &'static str,
}

impl ApplicationCategory {
    pub const ALL: ApplicationCategory = ApplicationCategory {
        display_name: "all-applications",
        icon_name: "open-menu-symbolic",
        mime_name: "",
    };
    pub const RECENTLY_USED: ApplicationCategory = ApplicationCategory {
        display_name: "recently-used",
        icon_name: "document-open-recent-symbolic",
        mime_name: "",
    };
    pub const AUDIO: ApplicationCategory = ApplicationCategory {
        display_name: "audio",
        icon_name: "applications-audio-symbolic",
        mime_name: "Audio",
    };
    pub const VIDEO: ApplicationCategory = ApplicationCategory {
        display_name: "video",
        icon_name: "applications-video-symbolic",
        mime_name: "Video",
    };
    pub const DEVELOPMENT: ApplicationCategory = ApplicationCategory {
        display_name: "development",
        icon_name: "applications-engineering-symbolic",
        mime_name: "Development",
    };
    pub const GAMES: ApplicationCategory = ApplicationCategory {
        display_name: "games",
        icon_name: "applications-games-symbolic",
        mime_name: "Game",
    };
    pub const GRAPHICS: ApplicationCategory = ApplicationCategory {
        display_name: "graphics",
        icon_name: "applications-graphics-symbolic",
        mime_name: "Graphics",
    };
    pub const NETWORK: ApplicationCategory = ApplicationCategory {
        display_name: "network",
        icon_name: "network-workgroup-symbolic",
        mime_name: "Network",
    };
    pub const OFFICE: ApplicationCategory = ApplicationCategory {
        display_name: "office",
        icon_name: "applications-office-symbolic",
        mime_name: "Office",
    };
    pub const SCIENCE: ApplicationCategory = ApplicationCategory {
        display_name: "science",
        icon_name: "applications-science-symbolic",
        mime_name: "Science",
    };
    pub const SETTINGS: ApplicationCategory = ApplicationCategory {
        display_name: "settings",
        icon_name: "preferences-system-symbolic",
        mime_name: "Settings",
    };
    pub const SYSTEM: ApplicationCategory = ApplicationCategory {
        display_name: "system",
        icon_name: "applications-system-symbolic",
        mime_name: "System",
    };
    pub const UTILITY: ApplicationCategory = ApplicationCategory {
        display_name: "utility",
        icon_name: "applications-utilities-symbolic",
        mime_name: "Utility",
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
