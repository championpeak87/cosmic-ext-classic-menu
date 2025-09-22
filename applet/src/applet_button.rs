use cosmic::iced::{widget::row, Alignment, Length};
use cosmic::widget::mouse_area;
use cosmic::Element;

use once_cell::sync::Lazy;

use crate::applet::{CosmicClassicMenu, Message, PopupType};

static AUTOSIZE_MAIN_ID: Lazy<cosmic::widget::Id> =
    Lazy::new(|| cosmic::widget::Id::new("autosize-main"));

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
    pub fn view_icon_only(applet: &CosmicClassicMenu) -> Element<'_, Message> {
        let button_icon = &applet.config.button_icon;
        let suggested_size = applet.core.applet.suggested_size(true);

        mouse_area(
            applet.core.applet.icon_button_from_handle(
                cosmic::widget::icon::from_name(button_icon.as_str())
                    .symbolic(applet.config.symbolic_icon)
                    .size(suggested_size.0)
                    .into()
            )
            .on_press(Message::TogglePopup(PopupType::MainMenu))
        )
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
    pub fn view_label_only(applet: &CosmicClassicMenu) -> Element<'_, Message> {
        let button_label = &applet.config.button_label;

        let content = row!(
            applet.core.applet.text(button_label),
            cosmic::widget::Space::new(5, Length::Shrink),
            cosmic::widget::vertical_space().height(Length::Fixed(
                (applet.core.applet.suggested_size(true).1
                    + 2 * applet.core.applet.suggested_padding(true)) as f32
            ))
        )
        .align_y(Alignment::Center);

        cosmic::widget::autosize::autosize(
            mouse_area(
                cosmic::widget::button::custom(content)
                    .padding([0, applet.core.applet.suggested_padding(true)])
                    .class(cosmic::theme::Button::AppletIcon)
                    .on_press(Message::TogglePopup(PopupType::MainMenu)),
            )
            .on_right_press(Message::TogglePopup(PopupType::ContextMenu)),
            AUTOSIZE_MAIN_ID.clone(),
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
    pub fn view_icon_and_label(applet: &CosmicClassicMenu) -> Element<'_, Message> {
        let button_label = &applet.config.button_label;
        let button_icon = &applet.config.button_icon;

        let content = row!(
            cosmic::widget::icon::from_name(button_icon.as_str())
                .symbolic(applet.config.symbolic_icon),
            applet.core.applet.text(button_label),
            cosmic::widget::vertical_space().height(Length::Fixed(
                (applet.core.applet.suggested_size(true).1
                    + 2 * applet.core.applet.suggested_padding(true)) as f32
            ))
        )
        .align_y(Alignment::Center);

        cosmic::widget::autosize::autosize(
            mouse_area(
                cosmic::widget::button::custom(content)
                    .padding([0, applet.core.applet.suggested_padding(true)])
                    .class(cosmic::theme::Button::AppletIcon)
                    .on_press(Message::TogglePopup(PopupType::MainMenu)),
            )
            .on_right_press(Message::TogglePopup(PopupType::ContextMenu)),
            AUTOSIZE_MAIN_ID.clone(),
        )
        .into()
    }
}
