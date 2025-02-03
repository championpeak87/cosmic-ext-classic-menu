use std::{collections::HashSet, sync::Arc};

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

pub fn available_categories() -> HashSet<&'static str> {
    let categories: HashSet<&str> = vec![
        "Audio",
        "Video",
        "Development",
        "Education",
        "Game",
        "Graphics",
        "Network",
        "Office",
        "Science",
        "Settings",
        "System",
        "Utility",
    ]
    .into_iter()
    .collect();

    categories
}
