use std::{collections::HashSet, fs, path::Path, str::Split, sync::Arc};

use cosmic::{desktop::DesktopEntryData, iced_winit::graphics::image::image_rs::load};
use freedesktop_desktop_entry::{
    default_paths, get_languages_from_env, DesktopEntry, Iter, PathSource,
};

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

pub fn available_categories() -> HashSet<String> {
    let available_apps = load_apps();
    let all_categories = available_apps
        .into_iter()
        .fold(HashSet::new(), |mut acc, x| {
            if x.categories.get(0).is_some() {
                acc.insert(x.categories.get(0).unwrap().clone());
            }
            acc
        });

    dbg!(&all_categories);
    all_categories
}
