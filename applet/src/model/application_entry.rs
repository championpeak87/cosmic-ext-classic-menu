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
    pub icon: Option<IconHandle>,
    pub comment: Option<String>,
    pub exec: Option<String>,
    pub category: Vec<String>,
    pub is_terminal: bool,
    pub item_id: Id,
    pub desktop_actions: Vec<DesktopAction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IconHandle {
    SvgHandle(cosmic::widget::svg::Handle),
    RasterHandle(cosmic::widget::image::Handle),
}

impl From<DesktopEntryData> for ApplicationEntry {
    fn from(app: DesktopEntryData) -> ApplicationEntry {
        ApplicationEntry {
            comment: get_comment(&app),
            is_terminal: get_is_terminal(&app),
            generic_name: get_generic_name(&app),
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
        } else {
            IconHandle::RasterHandle(Handle::from_path(named.path().unwrap()))
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
        IconHandle::SvgHandle(
            cosmic::widget::icon::from_name("application-x-executable")
                .size(32)
                .handle()
                .icon()
                .into_svg_handle()
                .unwrap(),
        )
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
