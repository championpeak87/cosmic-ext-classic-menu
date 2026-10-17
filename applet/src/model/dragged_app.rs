// SPDX-License-Identifier: GPL-3.0-only

use std::borrow::Cow;

use cosmic::iced::clipboard::mime::{AllowedMimeTypes, AsMimeTypes};

/// An app dragged out of the app view, identified by its id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraggedApp {
    pub app_id: String,
}

impl DraggedApp {
    /// Private to this applet, so only its own drop targets accept it.
    pub const MIME_TYPE: &'static str = "application/x-cosmic-ext-classic-menu-app";
}

impl AsMimeTypes for DraggedApp {
    fn available(&self) -> Cow<'static, [String]> {
        Cow::Owned(vec![Self::MIME_TYPE.to_owned()])
    }

    fn as_bytes(&self, mime_type: &str) -> Option<Cow<'static, [u8]>> {
        (mime_type == Self::MIME_TYPE).then(|| Cow::Owned(self.app_id.clone().into_bytes()))
    }
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
        Ok(DraggedApp { app_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_its_mime_type() {
        let app = DraggedApp {
            app_id: "org.gnome.Calculator".to_owned(),
        };
        let bytes = app.as_bytes(DraggedApp::MIME_TYPE).unwrap().into_owned();

        assert_eq!(
            DraggedApp::try_from((bytes, DraggedApp::MIME_TYPE.to_owned())),
            Ok(app)
        );
    }

    #[test]
    fn rejects_other_mime_types() {
        assert_eq!(
            DraggedApp::try_from((b"foo".to_vec(), "text/plain".to_owned())),
            Err(())
        );
    }
}
