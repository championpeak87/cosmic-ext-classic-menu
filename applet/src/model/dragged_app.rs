// SPDX-License-Identifier: GPL-3.0-only

use std::borrow::Cow;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use cosmic::iced::clipboard::mime::{AllowedMimeTypes, AsMimeTypes};

/// An app dragged out of the app view, identified by its id.
///
/// Also offered as the URL of its desktop file, which the dock accepts to
/// pin the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraggedApp {
    pub app_id: String,
    /// Path of the app's desktop file.
    pub desktop_file: Option<PathBuf>,
}

impl DraggedApp {
    /// Private to this applet, so only its own drop targets accept it.
    pub const MIME_TYPE: &'static str = "application/x-cosmic-ext-classic-menu-app";
    /// Accepted by the dock, file managers and other apps.
    pub const URI_LIST_MIME_TYPE: &'static str = "text/uri-list";
}

impl AsMimeTypes for DraggedApp {
    fn available(&self) -> Cow<'static, [String]> {
        let mut mime_types = vec![Self::MIME_TYPE.to_owned()];
        if self.desktop_file.is_some() {
            mime_types.push(Self::URI_LIST_MIME_TYPE.to_owned());
        }
        Cow::Owned(mime_types)
    }

    fn as_bytes(&self, mime_type: &str) -> Option<Cow<'static, [u8]>> {
        match mime_type {
            Self::MIME_TYPE => Some(Cow::Owned(self.app_id.clone().into_bytes())),
            Self::URI_LIST_MIME_TYPE => {
                let desktop_file = host_path(self.desktop_file.as_deref()?);
                let uri_list = format!("{}\r\n", file_url(&desktop_file));
                Some(Cow::Owned(uri_list.into_bytes()))
            }
            _ => None,
        }
    }
}

/// `path` as seen outside of the Flatpak sandbox, which mounts the host's
/// files under `/run/host`.
fn host_path(path: &Path) -> PathBuf {
    match path.strip_prefix("/run/host") {
        Ok(relative) => Path::new("/").join(relative),
        Err(_) => path.to_owned(),
    }
}

/// `file://` URL of the absolute `path`, percent-encoding all but the
/// characters allowed unencoded in a URL path.
fn file_url(path: &Path) -> String {
    let mut url = String::from("file://");
    for &byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            url.push(char::from(byte));
        } else {
            url.push_str(&format!("%{byte:02X}"));
        }
    }
    url
}

impl AllowedMimeTypes for DraggedApp {
    fn allowed() -> Cow<'static, [String]> {
        Cow::Owned(vec![Self::MIME_TYPE.to_owned()])
    }
}

impl TryFrom<(Vec<u8>, String)> for DraggedApp {
    type Error = ();

    fn try_from((data, mime_type): (Vec<u8>, String)) -> Result<Self, Self::Error> {
        if mime_type != Self::MIME_TYPE {
            return Err(());
        }
        let app_id = String::from_utf8(data).map_err(|_| ())?;
        Ok(DraggedApp {
            app_id,
            desktop_file: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_its_mime_type() {
        let app = DraggedApp {
            app_id: "org.gnome.Calculator".to_owned(),
            desktop_file: None,
        };
        let bytes = app.as_bytes(DraggedApp::MIME_TYPE).unwrap().into_owned();

        assert_eq!(
            DraggedApp::try_from((bytes, DraggedApp::MIME_TYPE.to_owned())),
            Ok(app)
        );
    }

    #[test]
    fn offers_the_desktop_file_url() {
        let app = DraggedApp {
            app_id: "foo".to_owned(),
            desktop_file: Some(PathBuf::from("/usr/share/applications/My App+1.desktop")),
        };

        assert!(app.available().contains(&DraggedApp::URI_LIST_MIME_TYPE.to_owned()));
        assert_eq!(
            app.as_bytes(DraggedApp::URI_LIST_MIME_TYPE).unwrap().as_ref(),
            b"file:///usr/share/applications/My%20App%2B1.desktop\r\n"
        );
    }

    #[test]
    fn offers_the_hosts_url_from_the_flatpak_sandbox() {
        let app = DraggedApp {
            app_id: "foo".to_owned(),
            desktop_file: Some(PathBuf::from("/run/host/usr/share/applications/foo.desktop")),
        };

        assert_eq!(
            app.as_bytes(DraggedApp::URI_LIST_MIME_TYPE).unwrap().as_ref(),
            b"file:///usr/share/applications/foo.desktop\r\n"
        );
    }

    #[test]
    fn offers_no_url_without_a_desktop_file() {
        let app = DraggedApp {
            app_id: "foo".to_owned(),
            desktop_file: None,
        };

        assert!(!app.available().contains(&DraggedApp::URI_LIST_MIME_TYPE.to_owned()));
        assert_eq!(app.as_bytes(DraggedApp::URI_LIST_MIME_TYPE), None);
    }

    #[test]
    fn rejects_other_mime_types() {
        assert_eq!(
            DraggedApp::try_from((b"foo".to_vec(), "text/plain".to_owned())),
            Err(())
        );
    }
}
