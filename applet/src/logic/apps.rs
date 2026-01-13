use crate::{
    config::{AppletConfig, RecentApplication},
    model::{application_category::ApplicationCategory, application_entry::ApplicationEntry},
};
use std::{collections::HashMap, string::String, sync::Arc};

use cached::{proc_macro::cached, UnboundCache};
use cosmic_app_list_config::AppListConfig;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

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
    ty = "UnboundCache<(), Vec<Arc<ApplicationEntry>>>",
    create = "{ UnboundCache::new() }"
)]
pub fn load_apps() -> Vec<Arc<ApplicationEntry>> {
    log::info!("Loading applications");
    let locale = std::env::var("LANG")
        .ok()
        .and_then(|l| l.split(".").next().map(str::to_string));
    let mut all_entries: Vec<Arc<ApplicationEntry>> =
        cosmic::desktop::load_applications(locale.as_slice(), false, None)
            .into_iter()
            .map(Into::into)
            .map(Arc::new)
            .collect();
    all_entries.sort_by_cached_key(|a| a.name.to_lowercase());

    log::info!("Applications fetched");
    all_entries
}

/// Strip diacritics (accents) from a string using NFD decomposition
fn strip_diacritics(s: &str) -> String {
    s.nfd()
        .filter(|c| !is_combining_mark(*c))
        .collect()
}

pub fn load_filtered_apps(filter: String) -> Vec<Arc<ApplicationEntry>> {
    let apps = load_apps();

    // If filter is empty, return all apps unsorted
    if filter.is_empty() {
        return apps;
    }

    let matcher = SkimMatcherV2::default();
    // Strip diacritics from filter to match accented characters
    let normalized_filter = strip_diacritics(&filter);

    let mut scored: Vec<(i64, Arc<ApplicationEntry>)> = apps
        .into_iter()
        .filter_map(|app| {
            // Strip diacritics from app fields for comparison
            let name_normalized = strip_diacritics(&app.name);
            let generic_name_normalized = app.generic_name.as_ref().map(|s| strip_diacritics(s));
            let comment_normalized = app.comment.as_ref().map(|s| strip_diacritics(s));

            // Match against the normalized name
            let name_score = matcher.fuzzy_match(&name_normalized, &normalized_filter);

            // Match against the normalized generic name
            let generic_name_score = generic_name_normalized
                .as_ref()
                .and_then(|d| matcher.fuzzy_match(d, &normalized_filter))
                .map(|x| x - 2); // penalize generic_name by 2

            // Match against the normalized comment
            let comment_score = comment_normalized
                .as_ref()
                .and_then(|d| matcher.fuzzy_match(d, &normalized_filter))
                .map(|x| x - 5); // penalize comment by 5

            // Take the best score from all fields
            let final_score = name_score.max(comment_score).max(generic_name_score);

            final_score.map(|score| (score, app))
        })
        .collect();

    // Highest score first
    scored.sort_unstable_by(|a, b| b.0.cmp(&a.0));

    scored.into_iter().map(|(_, app)| app).collect()
}

pub fn load_app_categories() -> Vec<ApplicationCategory> {
    use std::collections::HashSet;

    log::info!("Loading app categories...");
    let all_apps = load_apps();
    let used_categories: HashSet<&String> = all_apps.iter().flat_map(|app| &app.category).collect();

    // Define all app categories
    let apps_categories = [
        ApplicationCategory::ALL,
        ApplicationCategory::RECENTLY_USED,
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

    // Filter only available ones
    let categories = apps_categories
        .into_iter()
        .filter(|x| {
            x.permanent == true
                || (!x.mime_name.is_empty() && used_categories.contains(&x.mime_name.to_string()))
        })
        .collect();

    categories
}

pub fn get_recent_applications() -> Vec<Arc<ApplicationEntry>> {
    log::info!("Loading recent applications...");
    let mut recent_applications: Vec<RecentApplication> =
        AppletConfig::config().recent_applications;
    let all_applications_entries: HashMap<String, Arc<ApplicationEntry>> = load_apps()
        .into_iter()
        .map(|app| (app.id.clone(), app))
        .collect();

    recent_applications.sort_by(|a, b| b.launch_count.cmp(&a.launch_count));
    recent_applications
        .iter()
        .take(15) // take only first 15 recent entries, not to clutter the list
        .filter_map(|app| all_applications_entries.get(&app.app_id).cloned())
        .collect()
}

pub fn get_apps_of_category(category: ApplicationCategory) -> Vec<Arc<ApplicationEntry>> {
    log::info!("Getting apps of category: {}", category.mime_name);
    if category == ApplicationCategory::ALL {
        load_apps()
    } else if category == ApplicationCategory::RECENTLY_USED {
        get_recent_applications()
    } else {
        load_apps()
            .into_iter()
            .filter(|app| app.category.iter().any(|c| c == category.mime_name))
            .collect()
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
                for path in cosmic::desktop::fde::default_paths() {
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

pub fn is_app_in_favorites(app: &ApplicationEntry, config: &AppListConfig) -> bool {
    config.favorites.iter().any(|app_id| app.id.eq(app_id))
}