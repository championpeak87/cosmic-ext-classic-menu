use crate::{
    config::{CosmicClassicMenuConfig, RecentApplication},
    fl,
    model::application_entry::ApplicationEntry,
};
use std::{collections::HashMap, fmt::Display, string::String, sync::Arc};

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use serde::{Deserialize, Serialize};

use cosmic::{
    iced::{stream, Subscription},
    iced_futures::futures::{self, SinkExt},
};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::fmt::Debug;
use std::hash::Hash;
use tokio::sync::mpsc;

pub struct Apps;

impl Apps {
    pub async fn load_apps() -> Vec<Arc<ApplicationEntry>> {
        println!("Loading applications...");
        let locale = std::env::var("LANG")
            .ok()
            .and_then(|l| l.split(".").next().map(str::to_string));
        let mut all_entries: Vec<Arc<ApplicationEntry>> =
            cosmic::desktop::load_applications(locale.as_slice(), false, None)
                .into_iter()
                .map(Into::into)
                .map(Arc::new)
                .collect();
        all_entries.sort_by(|a, b| a.name.cmp(&b.name));

        all_entries
    }

    pub async fn load_filtered_apps(filter: String) -> Vec<Arc<ApplicationEntry>> {
        let matcher: SkimMatcherV2 = SkimMatcherV2::default();
        let mut search_result: Vec<(Option<i64>, Arc<ApplicationEntry>)> = Self::load_apps()
            .await
            .into_iter()
            .map(|app| (matcher.fuzzy_match(&app.name, &filter), app))
            .filter(|app| app.0.is_some())
            .collect();

        search_result.sort_by(|a, b| b.0.cmp(&a.0));

        search_result.into_iter().map(|(_, app)| app).collect()
    }

    pub async fn load_app_categories() -> Vec<ApplicationCategory> {
        use std::collections::HashSet;

        println!("Loading app categories...");
        let all_apps = Self::load_apps().await;
        let mut used_categories = HashSet::new();
        for app in &all_apps {
            for cat in &app.category {
                used_categories.insert(cat);
            }
        }

        // Všechny možné kategorie
        const APPS_CATEGORIES: &[ApplicationCategory] = &[
            ApplicationCategory::AUDIO,
            ApplicationCategory::VIDEO,
            ApplicationCategory::DEVELOPMENT,
            ApplicationCategory::GAMES,
            ApplicationCategory::GRAPHICS,
            ApplicationCategory::NETWORK,
            ApplicationCategory::OFFICE,
            ApplicationCategory::SCIENCE,
            ApplicationCategory::SETTINGS,
            ApplicationCategory::SYSTEM,
            ApplicationCategory::UTILITY,
        ];

        // Vyberte pouze ty, které jsou použité
        let mut categories = Vec::with_capacity(2 + APPS_CATEGORIES.len());
        categories.push(ApplicationCategory::ALL);
        categories.push(ApplicationCategory::RECENTLY_USED);
        for cat in APPS_CATEGORIES {
            if !cat.mime_name.is_empty() && used_categories.contains(&cat.mime_name.to_string()) {
                categories.push(cat.clone());
            }
        }
        categories
    }

    pub async fn get_recent_applications() -> Vec<Arc<ApplicationEntry>> {
        println!("Loading recent applications...");
        let recent_applications: &Vec<RecentApplication> =
            &CosmicClassicMenuConfig::config().recent_applications;
        let all_applications_entries: HashMap<String, Arc<ApplicationEntry>> = Self::load_apps()
            .await
            .into_iter()
            .map(|app| (app.id.clone(), app))
            .collect();

        // recent_applications.sort_by(|a, b| b.launch_count.cmp(&a.launch_count));
        recent_applications
            .iter()
            .filter_map(|app| all_applications_entries.get(&app.app_id).cloned())
            .collect()
    }

    pub async fn get_apps_of_category(category: ApplicationCategory) -> Vec<Arc<ApplicationEntry>> {
        println!("Getting apps of category: {}", category.mime_name);
        if category == ApplicationCategory::ALL {
            Self::load_apps().await
        } else if category == ApplicationCategory::RECENTLY_USED {
            Self::get_recent_applications().await
        } else {
            Self::load_apps()
                .await
                .into_iter()
                .filter(|app| app.category.contains(&category.mime_name.to_string()))
                .collect()
        }
    }
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
    pub icon_svg_bytes: &'static [u8],
    pub mime_name: &'static str,
}

impl ApplicationCategory {
    pub const ALL: ApplicationCategory = ApplicationCategory {
        display_name: "all-applications",
        icon_svg_bytes: include_bytes!("../../../res/icons/bundled/open-menu-symbolic.svg"),
        mime_name: "",
    };
    pub const RECENTLY_USED: ApplicationCategory = ApplicationCategory {
        display_name: "recently-used",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/document-open-recent-symbolic.svg"
        ),
        mime_name: "",
    };
    pub const AUDIO: ApplicationCategory = ApplicationCategory {
        display_name: "audio",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-audio-symbolic.svg"
        ),
        mime_name: "Audio",
    };
    pub const VIDEO: ApplicationCategory = ApplicationCategory {
        display_name: "video",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-video-symbolic.svg"
        ),
        mime_name: "Video",
    };
    pub const DEVELOPMENT: ApplicationCategory = ApplicationCategory {
        display_name: "development",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-engineering-symbolic.svg"
        ),
        mime_name: "Development",
    };
    pub const GAMES: ApplicationCategory = ApplicationCategory {
        display_name: "games",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-games-symbolic.svg"
        ),
        mime_name: "Game",
    };
    pub const GRAPHICS: ApplicationCategory = ApplicationCategory {
        display_name: "graphics",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-graphics-symbolic.svg"
        ),
        mime_name: "Graphics",
    };
    pub const NETWORK: ApplicationCategory = ApplicationCategory {
        display_name: "network",
        icon_svg_bytes: include_bytes!("../../../res/icons/bundled/network-workgroup-symbolic.svg"),
        mime_name: "Network",
    };
    pub const OFFICE: ApplicationCategory = ApplicationCategory {
        display_name: "office",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-office-symbolic.svg"
        ),
        mime_name: "Office",
    };
    pub const SCIENCE: ApplicationCategory = ApplicationCategory {
        display_name: "science",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-science-symbolic.svg"
        ),
        mime_name: "Science",
    };
    pub const SETTINGS: ApplicationCategory = ApplicationCategory {
        display_name: "settings",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/preferences-system-symbolic.svg"
        ),
        mime_name: "Settings",
    };
    pub const SYSTEM: ApplicationCategory = ApplicationCategory {
        display_name: "system",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-system-symbolic.svg"
        ),
        mime_name: "System",
    };
    pub const UTILITY: ApplicationCategory = ApplicationCategory {
        display_name: "utility",
        icon_svg_bytes: include_bytes!(
            "../../../res/icons/bundled/applications-utilities-symbolic.svg"
        ),
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
