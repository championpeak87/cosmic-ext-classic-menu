use freedesktop_desktop_entry::{default_paths, get_languages_from_env, Iter, DesktopEntry};

pub fn get_apps() -> Vec<DesktopEntry> {
    let locales = get_languages_from_env();

    let entries = Iter::new(default_paths())
        .entries(Some(&locales))
        .collect::<Vec<_>>();

    entries
}

fn load_desktop_entries_from_app_ids<I, L>(ids: &[I], locales: &[L]) -> Vec<DesktopEntry>
where
    I: AsRef<str>,
    L: AsRef<str>,
{
    let entries = Iter::new(default_paths())
        .entries(Some(locales))
        .collect::<Vec<_>>();

    ids.iter()
        .map(|id| {
            freedesktop_desktop_entry::matching::find_entry_from_appid(entries.iter(), id.as_ref())
                .unwrap_or(&freedesktop_desktop_entry::DesktopEntry::from_appid(String::from(id)))
                .to_owned()
        })
        .collect_vec()
}