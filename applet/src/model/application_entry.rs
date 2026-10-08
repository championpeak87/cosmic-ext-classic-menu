use std::path::Path;

use cosmic::{
    desktop::{DesktopEntryData, fde::{DesktopEntry, IconSource}},
    widget::{Id, icon::Named, image::Handle},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopAction {
    pub name: String,
    pub exec: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Represents an application entry in the Cosmic Classic Menu.
pub struct ApplicationEntry {
    pub name: String,
    pub generic_name: Option<String>,
    pub id: String,
    /// Desktop file id as defined by the desktop entry specification,
    /// e.g. `wine-Programs-Foo-Bar.desktop` for `applications/wine/Programs/Foo/Bar.desktop`.
    pub desktop_file_id: String,
    pub icon: Option<IconHandle>,
    pub comment: Option<String>,
    pub exec: Option<String>,
    pub category: Vec<String>,
    pub is_terminal: bool,
    pub item_id: Id,
    pub desktop_actions: Vec<DesktopAction>,
    /// `name`, `generic_name` and `comment` without diacritics, computed once
    /// at load so searching does not redo it on every keystroke.
    pub search_name: String,
    pub search_generic_name: Option<String>,
    pub search_comment: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IconHandle {
    SvgHandle(cosmic::widget::svg::Handle),
    RasterHandle(cosmic::widget::image::Handle),
}

impl From<DesktopEntryData> for ApplicationEntry {
    fn from(app: DesktopEntryData) -> ApplicationEntry {
        use crate::logic::apps::strip_diacritics;

        let comment = get_comment(&app);
        let generic_name = get_generic_name(&app);
        ApplicationEntry {
            search_name: strip_diacritics(&app.name),
            search_generic_name: generic_name.as_deref().map(strip_diacritics),
            search_comment: comment.as_deref().map(strip_diacritics),
            comment,
            is_terminal: get_is_terminal(&app),
            generic_name,
            desktop_file_id: get_desktop_file_id(app.path.as_deref()),
            id: app.id,
            name: app.name,
            icon: match app.icon {
                IconSource::Name(name) => Some(
                    cosmic::widget::icon::from_name(name.as_str())
                        .size(64)
                        .fallback(Some(cosmic::widget::icon::IconFallback::Names(vec![
                            "application-default".into(),
                            "application-x-executable".into(),
                        ])))
                        .prefer_svg(true)
                        .into(),
                ),
                IconSource::Path(path) => {
                    Some(cosmic::widget::icon(cosmic::widget::icon::from_path(path.clone())).into())
                }
            },
            exec: app.exec,
            category: app.categories,
            item_id: Id::unique(),
            desktop_actions: app.desktop_actions.into_iter().map(From::from).collect(),
        }
    }
}

impl From<cosmic::widget::Icon> for IconHandle {
    fn from(icon: cosmic::widget::Icon) -> IconHandle {
        if let Some(icon_handle) = icon.into_svg_handle() {
            IconHandle::SvgHandle(cosmic::widget::svg::Handle::from(icon_handle))
        } else {
            IconHandle::default()
        }
    }
}

impl From<Named> for IconHandle {
    fn from(named: Named) -> IconHandle {
        if let Some(handle) = named.clone().icon().into_svg_handle() {
            IconHandle::SvgHandle(handle)
        } else if let Some(path) = named.path() {
            // PNG based icon themes, e.g. ubuntu-mono-light
            IconHandle::RasterHandle(Handle::from_path(path))
        } else {
            IconHandle::bundled_fallback()
        }
    }
}

impl From<cosmic::desktop::DesktopAction> for DesktopAction {
    fn from(value: cosmic::desktop::DesktopAction) -> Self {
        Self {
            exec: value.exec,
            name: value.name,
        }
    }
}

impl Default for IconHandle {
    fn default() -> Self {
        cosmic::widget::icon::from_name("application-x-executable")
            .size(32)
            .into()
    }
}

impl IconHandle {
    /// Used when the icon theme provides no icon at all, so it must not
    /// depend on the theme.
    fn bundled_fallback() -> Self {
        IconHandle::SvgHandle(cosmic::widget::svg::Handle::from_memory(
            &include_bytes!("../../../res/icons/bundled/applications-system-symbolic.svg")[..],
        ))
    }
}

fn get_comment(app: &DesktopEntryData) -> Option<String> {
    if let Some(path) = &app.path {
        let locale = std::env::var("LANG")
            .ok()
            .and_then(|l| l.split(".").next().map(str::to_string));
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

fn get_is_terminal(app: &DesktopEntryData) -> bool {
    if let Some(path) = &app.path {
        let locale = std::env::var("LANG")
            .ok()
            .and_then(|l| l.split(".").next().map(str::to_string));
        let desktop_entry = DesktopEntry::from_path(path, Some(locale.as_slice()));

        if let Ok(entry) = desktop_entry {
            return entry.terminal();
        }
    }

    false
}

fn get_generic_name(app: &DesktopEntryData) -> Option<String> {
    if let Some(path) = &app.path {
        let locale = [std::env::var("LANG")
            .ok()
            .and_then(|l| l.split(".").next().map(str::to_string))
            .unwrap_or_else(|| "en_US".to_string())];
        let desktop_entry = DesktopEntry::from_path(path, Some(locale.as_slice()));

        if let Ok(entry) = desktop_entry {
            return entry.generic_name(&locale).map(|name| name.into_owned());
        }
    }

    None
}

fn get_desktop_file_id(path: Option<&Path>) -> String {
    let Some(path) = path else {
        return String::new();
    };

    let components: Vec<_> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect();

    // the id is the path relative to the `applications` directory with `/` replaced by `-`
    match components.iter().rposition(|c| c == "applications") {
        Some(index) if index + 1 < components.len() => components[index + 1..].join("-"),
        _ => components.last().map(|c| c.to_string()).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_file_id_of_nested_entry() {
        assert_eq!(
            get_desktop_file_id(Some(Path::new(
                "/home/user/.local/share/applications/wine/Programs/Foo/Bar.desktop"
            ))),
            "wine-Programs-Foo-Bar.desktop"
        );
    }

    #[test]
    fn desktop_file_id_of_top_level_entry() {
        assert_eq!(
            get_desktop_file_id(Some(Path::new(
                "/usr/share/applications/com.cisco.secureclient.gui.desktop"
            ))),
            "com.cisco.secureclient.gui.desktop"
        );
    }
}
