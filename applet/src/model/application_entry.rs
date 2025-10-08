use cosmic::desktop::DesktopEntryData;
use freedesktop_desktop_entry::{DesktopEntry, IconSource};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopAction {
    pub name: String,
    pub exec: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Represents an application entry in the Cosmic Classic Menu.
pub struct ApplicationEntry {
    pub name: String,
    pub id: String,
    pub icon: IconSource,
    pub comment: Option<String>,
    pub exec: Option<String>,
    pub category: Vec<String>,
    pub is_terminal: bool,
    pub desktop_actions: Vec<DesktopAction>,
}

impl Into<ApplicationEntry> for DesktopEntryData {
    fn into(self) -> ApplicationEntry {
        ApplicationEntry {
            comment: get_comment(&self),
            is_terminal: get_is_terminal(&self),
            id: self.id,
            name: self.name,
            icon: self.icon,
            exec: self.exec,
            category: self.categories,
            desktop_actions: self.desktop_actions.into_iter().map(From::from).collect(),
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
