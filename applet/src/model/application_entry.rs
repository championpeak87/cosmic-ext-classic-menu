use cosmic::{
    desktop::DesktopEntryData,
    widget::{icon::Named, image::Handle},
};
use freedesktop_desktop_entry::DesktopEntry;

#[derive(Clone, Debug)]
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
}

#[derive(Clone, Debug)]
pub enum IconHandle {
    SvgHandle(cosmic::widget::svg::Handle),
    RasterHandle(cosmic::widget::image::Handle),
}

impl Into<ApplicationEntry> for DesktopEntryData {
    fn into(self) -> ApplicationEntry {
        ApplicationEntry {
            comment: get_comment(&self),
            is_terminal: get_is_terminal(&self),
            generic_name: get_generic_name(&self),
            id: self.id,
            name: self.name,
            icon: match self.icon {
                freedesktop_desktop_entry::IconSource::Name(name) => Some(
                    cosmic::widget::icon::from_name(name.as_str())
                        .size(64)
                        .fallback(Some(cosmic::widget::icon::IconFallback::Names(vec![
                            "application-default".into(),
                            "application-x-executable".into(),
                        ])))
                        .prefer_svg(true)
                        .into(),
                ),
                freedesktop_desktop_entry::IconSource::Path(path) => {
                    Some(cosmic::widget::icon(cosmic::widget::icon::from_path(path.clone())).into())
                }
            },
            exec: self.exec,
            category: self.categories,
        }
    }
}

impl Into<IconHandle> for cosmic::widget::Icon {
    fn into(self) -> IconHandle {
        IconHandle::SvgHandle(cosmic::widget::svg::Handle::from(
            self.into_svg_handle().unwrap(),
        ))
    }
}

impl Into<IconHandle> for Named {
    fn into(self) -> IconHandle {
        if let Some(handle) = self.clone().icon().into_svg_handle() {
            IconHandle::SvgHandle(handle)
        } else {
            IconHandle::RasterHandle(Handle::from_path(self.path().unwrap()))
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
