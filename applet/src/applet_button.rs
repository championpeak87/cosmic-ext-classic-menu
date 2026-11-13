use std::path::PathBuf;

use cosmic::iced::core::text::Wrapping;
use cosmic::iced::Length;
use cosmic::iced::{widget::column, widget::row, Alignment};
use cosmic::widget::{container, mouse_area};
use cosmic::Element;

use crate::applet::{Applet, Message};
use crate::model::popup_type::PopupType;
use crate::widgets::ArrowCursor;

const BUTTON_DEFAULT_ICON: &[u8] =
    include_bytes!("../../res/icons/bundled/applet-button/default.svg");

/// Represents the applet button component of the Cosmic Classic Menu.
pub struct AppletButton;

/// Sizes of the applet button, so it fills the panel across, as other
/// applets do.
///
/// libcosmic sizes non-symbolic icons, such as the default and most custom
/// ones, bigger than symbolic ones, but pads them as symbolic ones, so the
/// button would be thicker than the panel. Here the padding goes with the
/// icon, so both make up the thickness of the panel.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ButtonMetrics {
    horizontal: bool,
    /// Size of the button across the panel: the panel's thickness.
    thickness: f32,
    icon_size: f32,
    /// Padding along the panel, at both ends of the button.
    padding: f32,
}

impl ButtonMetrics {
    fn of(context: &cosmic::applet::Context, symbolic: bool) -> Self {
        let horizontal = context.is_horizontal();
        // The panel is as thick as a symbolic icon with its padding.
        let (width, height) = context.suggested_size(true);
        let (_, cross_padding) = context.suggested_padding(true);
        let across = if horizontal { height } else { width };
        let thickness = f32::from(across) + 2. * f32::from(cross_padding);

        let (icon_width, icon_height) = context.suggested_size(symbolic);
        let icon_size = f32::from(if horizontal { icon_height } else { icon_width });
        let (padding, _) = context.suggested_padding(symbolic);
        ButtonMetrics {
            horizontal,
            thickness,
            icon_size: icon_size.min(thickness),
            padding: f32::from(padding),
        }
    }

    /// Width and height of a button `length` long along the panel, with its
    /// padding.
    fn size(&self, length: Length) -> (Length, Length) {
        let across = Length::Fixed(self.thickness);
        if self.horizontal {
            (length, across)
        } else {
            (across, length)
        }
    }
}

impl AppletButton {
    fn icon_handle(applet: &Applet) -> cosmic::widget::icon::Handle {
        let button_icon: PathBuf = applet.config.button_icon.clone().into();
        if button_icon.exists() {
            cosmic::widget::icon::from_path(button_icon)
        } else {
            cosmic::widget::icon::from_svg_bytes(BUTTON_DEFAULT_ICON)
        }
    }

    fn icon(handle: cosmic::widget::icon::Handle, size: f32) -> Element<'static, Message> {
        let symbolic = handle.symbolic;
        let icon = cosmic::widget::icon(handle)
            .width(Length::Fixed(size))
            .height(Length::Fixed(size));
        if symbolic {
            // Colored as the panel's text, as libcosmic does.
            icon.class(cosmic::theme::Svg::Custom(std::rc::Rc::new(|theme| {
                cosmic::iced::widget::svg::Style {
                    color: Some(theme.cosmic().background(theme.transparent).on.into()),
                }
            })))
            .into()
        } else {
            icon.into()
        }
    }

    /// Style of the button, highlighted while its menu or context menu is
    /// `open` as if hovered, and as if pressed when hovered then.
    ///
    /// libcosmic's `AppletIcon` style ignores the button being selected.
    fn class(open: bool) -> cosmic::theme::Button {
        use cosmic::widget::button::Catalog;

        if !open {
            return cosmic::theme::Button::AppletIcon;
        }
        let style = &cosmic::theme::Button::AppletIcon;
        cosmic::theme::Button::Custom {
            active: Box::new(move |focused, theme| theme.hovered(focused, true, style)),
            disabled: Box::new(move |theme| theme.disabled(style)),
            hovered: Box::new(move |focused, theme| theme.pressed(focused, true, style)),
            pressed: Box::new(move |focused, theme| theme.pressed(focused, true, style)),
        }
    }

    /// The button around `content`, filling the panel across, opening the
    /// menu on a click and the context menu on a right click.
    fn button<'a>(
        applet: &Applet,
        content: impl Into<Element<'a, Message>>,
        metrics: ButtonMetrics,
        length: Length,
    ) -> Element<'a, Message> {
        let (width, height) = metrics.size(length);
        let padding = if metrics.horizontal {
            [0., metrics.padding]
        } else {
            [metrics.padding, 0.]
        };
        // Centered across the panel, and along it if the button has a fixed
        // length; otherwise it is as long as the content, so the window
        // sized to it does not grow.
        let along = match length {
            Length::Fixed(_) => Length::Fill,
            _ => Length::Shrink,
        };
        let (content_width, content_height) = if metrics.horizontal {
            (along, Length::Fill)
        } else {
            (Length::Fill, along)
        };
        ArrowCursor::new(mouse_area(
            cosmic::widget::button::custom(
                container(content)
                    .width(content_width)
                    .height(content_height)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .padding(padding),
            )
            .width(width)
            .height(height)
            .padding(0)
            .class(Self::class(applet.popup.is_some()))
            .on_press(Message::TogglePopup(PopupType::MainMenu)),
        )
        .on_right_press(Message::TogglePopup(PopupType::ContextMenu)))
        .into()
    }

    /// The label, which wraps to fit across a vertical panel.
    fn label(applet: &Applet, metrics: ButtonMetrics) -> Element<'_, Message> {
        let label = applet.core.applet.text(applet.config.button_label.as_str());
        if metrics.horizontal {
            label.wrapping(Wrapping::None).into()
        } else {
            label
                .wrapping(Wrapping::WordOrGlyph)
                .align_x(Alignment::Center)
                .width(Length::Fill)
                .into()
        }
    }

    /// The applet button with only the icon.
    ///
    /// Clicking it opens the menu, right-clicking it the context menu.
    pub fn view_icon_only(applet: &Applet) -> Element<'_, Message> {
        let handle = Self::icon_handle(applet);
        let metrics = ButtonMetrics::of(&applet.core.applet, handle.symbolic);
        let icon = Self::icon(handle, metrics.icon_size);
        Self::button(
            applet,
            icon,
            metrics,
            Length::Fixed(metrics.icon_size + 2. * metrics.padding),
        )
    }

    /// The applet button with only the label.
    ///
    /// Clicking it opens the menu, right-clicking it the context menu.
    pub fn view_label_only(applet: &Applet) -> Element<'_, Message> {
        let metrics = ButtonMetrics::of(&applet.core.applet, true);
        applet
            .core
            .applet
            .autosize_window(Self::button(
                applet,
                Self::label(applet, metrics),
                metrics,
                Length::Shrink,
            ))
            .into()
    }

    /// The applet button with the icon and the label, side by side along
    /// a horizontal panel, stacked across a vertical one.
    ///
    /// Clicking it opens the menu, right-clicking it the context menu.
    pub fn view_icon_and_label(applet: &Applet) -> Element<'_, Message> {
        let handle = Self::icon_handle(applet);
        let metrics = ButtonMetrics::of(&applet.core.applet, handle.symbolic);
        let icon = Self::icon(handle, metrics.icon_size);
        let label = Self::label(applet, metrics);
        let spacing = cosmic::theme::active().cosmic().space_xxs();

        let content: Element<'_, Message> = if metrics.horizontal {
            row![icon, label]
                .spacing(spacing)
                .align_y(Alignment::Center)
                .into()
        } else {
            column![icon, label]
                .spacing(spacing)
                .align_x(Alignment::Center)
                .into()
        };

        applet
            .core
            .applet
            .autosize_window(Self::button(applet, content, metrics, Length::Shrink))
            .auto_height(true)
            .auto_width(true)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::ButtonMetrics;
    use cosmic::applet::cosmic_panel_config::{PanelAnchor, PanelSize};
    use cosmic::applet::{Context, Size};

    fn context(size: PanelSize, anchor: PanelAnchor) -> Context {
        Context {
            size: Size::PanelSize(size),
            anchor,
            ..Context::default()
        }
    }

    /// The icon button, symbolic or not, is as big as the window libcosmic
    /// gives the applet, so it fills the panel without overflowing it.
    #[test]
    fn icon_buttons_fill_the_applet_window() {
        for size in [PanelSize::XS, PanelSize::S, PanelSize::M, PanelSize::L, PanelSize::XL] {
            for anchor in [PanelAnchor::Bottom, PanelAnchor::Left] {
                let context = context(size.clone(), anchor);
                let (window_width, window_height) = context.suggested_window_size();
                let (along, across) = if context.is_horizontal() {
                    (window_width.get(), window_height.get())
                } else {
                    (window_height.get(), window_width.get())
                };
                for symbolic in [true, false] {
                    let metrics = ButtonMetrics::of(&context, symbolic);
                    assert_eq!(metrics.thickness, across as f32, "{size:?} {anchor:?} {symbolic}");
                    assert_eq!(
                        metrics.icon_size + 2. * metrics.padding,
                        along as f32,
                        "{size:?} {anchor:?} {symbolic}"
                    );
                }
            }
        }
    }
}
