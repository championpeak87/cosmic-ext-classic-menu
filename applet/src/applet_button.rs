use std::path::PathBuf;

use cosmic::iced::mouse::Interaction;
use cosmic::iced::Length;
use cosmic::iced::{widget::row, Alignment};
use cosmic::widget::mouse_area;
use cosmic::Element;

use crate::applet::{Applet, Message, PopupType};

const BUTTON_DEFAULT_ICON: &[u8] =
    include_bytes!("../../res/icons/bundled/applet-button/default.svg");

/// Represents the applet button component of the Cosmic Classic Menu.
pub struct AppletButton;

impl AppletButton {
    /// Creates a view for the applet button with only an icon.
    ///
    /// This function generates a button that displays only the applet's icon.
    /// Clicking the button triggers the `TogglePopup` message with the `MainMenu` popup type.
    /// Right-clicking the button triggers the `TogglePopup` message with the `ContextMenu` popup type.
    ///
    /// # Arguments
    /// * `applet` - A reference to the `CosmicClassicMenu` instance.
    ///
    /// # Returns
    /// An `Element<Message>` representing the icon-only applet button.
    pub fn view_icon_only(applet: &Applet) -> Element<'_, Message> {
        let button_icon: PathBuf = applet.config.button_icon.clone().into();
        let icon_handle = if button_icon.exists() {
            cosmic::widget::icon::from_path(button_icon)
        } else {
            cosmic::widget::icon::from_svg_bytes(BUTTON_DEFAULT_ICON)
        };

        mouse_area(
            applet
                .core
                .applet
                .icon_button_from_handle(icon_handle)
                .on_press(Message::TogglePopup(PopupType::MainMenu)),
        )
        .interaction(Interaction::Idle)
        .on_right_press(Message::TogglePopup(PopupType::ContextMenu))
        .into()
    }

    /// Creates a view for the applet button with only a label.
    ///
    /// This function generates a button that displays only the applet's label.
    /// Clicking the button triggers the `TogglePopup` message with the `MainMenu` popup type.
    /// Right-clicking the button triggers the `TogglePopup` message with the `ContextMenu` popup type.
    ///
    /// # Arguments
    /// * `applet` - A reference to the `CosmicClassicMenu` instance.
    ///
    /// # Returns
    /// An `Element<Message>` representing the label-only applet button.
    pub fn view_label_only(applet: &Applet) -> Element<'_, Message> {
        applet
            .core
            .applet
            .autosize_window(
                mouse_area(applet.core.applet.text_button(
                    applet.config.button_label.as_str(),
                    Message::TogglePopup(PopupType::MainMenu),
                ))
                .interaction(Interaction::Idle)
                .on_right_press(Message::TogglePopup(PopupType::ContextMenu)),
            )
            .into()
    }

    /// Creates a view for the applet button with both an icon and a label.
    ///
    /// This function generates a button that displays both the applet's icon and label.
    /// Clicking the button triggers the `TogglePopup` message with the `MainMenu` popup type.
    /// Right-clicking the button triggers the `TogglePopup` message with the `ContextMenu` popup type.
    ///
    /// # Arguments
    /// * `applet` - A reference to the `CosmicClassicMenu` instance.
    ///
    /// # Returns
    /// An `Element<Message>` representing the applet button with both an icon and a label.
    pub fn view_icon_and_label(applet: &Applet) -> Element<'_, Message> {
        let button_icon: PathBuf = applet.config.button_icon.clone().into();
        let icon_handle = if button_icon.exists() {
            cosmic::widget::icon::from_path(button_icon)
        } else {
            cosmic::widget::icon::from_svg_bytes(BUTTON_DEFAULT_ICON)
        };

        let suggested_size = applet.core.applet.suggested_size(icon_handle.symbolic);

        let content = row![
            cosmic::widget::icon(icon_handle)
                .width(Length::Fixed(suggested_size.0 as f32))
                .height(Length::Fixed(suggested_size.1 as f32)),
            cosmic::widget::Space::new(5, Length::Shrink),
            cosmic::widget::text(applet.config.button_label.as_str())
        ]
        .align_y(Alignment::Center);

        applet
            .core
            .applet
            .autosize_window(
                mouse_area(
                    cosmic::widget::button::custom(content)
                        .class(cosmic::theme::Button::AppletIcon)
                        .on_press(Message::TogglePopup(PopupType::MainMenu)),
                )
                .interaction(Interaction::Idle)
                .on_right_press(Message::TogglePopup(PopupType::ContextMenu)),
            )
            .auto_height(true)
            .auto_width(true)
            .into()
    }
}
