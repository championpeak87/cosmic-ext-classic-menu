// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;

use cosmic::iced::core::text::{Ellipsize, EllipsizeHeightLimit, Wrapping};
use cosmic::iced::widget::{column, row};
use cosmic::iced::window::Id;
use cosmic::iced::{Alignment, ContentFit, Length, Vector};
use cosmic::widget::text;
use cosmic::widget::{DndSource, ListColumn, container, scrollable};
use cosmic::{Element, theme};

use crate::applet::{Applet, Message};
use crate::config::ItemSize;
use crate::widgets::right_release_guard::RightReleaseGuard;
use crate::model::application_entry::{ApplicationEntry, IconHandle};
use crate::model::dragged_app::DraggedApp;

/// A virtualized app list widget that only renders visible items for performance.
///
/// This widget improves performance when dealing with large application lists
/// by only rendering items that are currently visible in the viewport.
pub struct VirtualizedAppList;

impl VirtualizedAppList {
    /// Buffer items to render above/below viewport for smooth scrolling
    const RENDER_BUFFER: usize = 2;
    /// Font size of app descriptions.
    const DESCRIPTION_SIZE: u16 = 10;

    /// Icon size and row height of apps of `size`.
    pub fn item_metrics(size: ItemSize) -> (u16, u16) {
        match size {
            ItemSize::Small => (24, 36),
            ItemSize::Medium => (32, 48),
            ItemSize::Large => (48, 64),
        }
    }

    /// Creates a virtualized app list element
    ///
    /// # Arguments
    /// * `applet` - Reference to the applet containing app data and configuration
    ///
    /// # Returns
    /// A scrollable element containing only the visible app items
    pub fn view(applet: &Applet) -> Element<'_, Message> {
        let space_s = theme::active().cosmic().space_s();

        let item_height = f32::from(Self::item_metrics(applet.config.column_item_size).1);
        let scroll_offset = applet.scroll_offset;
        let total_items = applet.available_applications.len();

        // Calculate which items should be rendered based on scroll position
        let visible_start = (scroll_offset / item_height).floor() as usize;
        let viewport_height = applet.scroll_viewport_height.max(item_height);
        let visible_count = ((viewport_height / item_height).ceil() as usize) + 1;

        // Add buffer for smooth scrolling
        let render_start = visible_start.saturating_sub(Self::RENDER_BUFFER);
        let render_end = (visible_start + visible_count + Self::RENDER_BUFFER).min(total_items);

        // Build items to render
        let mut items: Vec<Element<'_, Message>> = Vec::new();

        // Add spacer above visible items to maintain scroll position
        if render_start > 0 {
            let spacer_height = (render_start as f32 * item_height) as u16;
            items.push(cosmic::widget::Space::new().width(Length::Fill).height(spacer_height).into());
        }

        // Add visible and buffered items
        for (original_index, app) in applet
            .available_applications
            .iter()
            .enumerate()
            .skip(render_start)
            .take(render_end - render_start)
        {
            items.push(Self::create_app_button(
                applet,
                original_index,
                app
            ));
        }

        // Add spacer below visible items
        if render_end < total_items {
            let remaining_height = ((total_items - render_end) as f32 * item_height) as u16;
            items.push(cosmic::widget::Space::new().width(Length::Fill).height(remaining_height).into());
        }

        // Build list column from items
        let app_list: ListColumn<Message> = items.into_iter().fold(
            cosmic::widget::list_column()
                .list_item_padding([0., space_s as f32]),
            |list, item| list.add(item),
        );

        let app_list = scrollable(container(app_list))
            .height(Length::Fill)
            .width(Length::Fill)
            .id(applet.scrollable_id.clone())
            .on_scroll(|viewport| Message::ScrollUpdated(viewport));

        Self::with_context_menu(applet, app_list)
    }

    /// Creates an individual app button with context menu
    ///
    /// # Arguments
    /// * `applet` - Reference to the applet
    /// * `index` - The index of the app in the list
    /// * `app` - The application entry
    /// * `space_l` - Large spacing value for icon size
    /// * `space_xl` - Extra large spacing value for button height
    ///
    /// # Returns
    /// An element containing a button with context menu
    fn create_app_button<'a>(
        applet: &'a Applet,
        index: usize,
        app: &'a Arc<ApplicationEntry>
    ) -> Element<'a, Message> {
        let (icon_size, item_height) = Self::item_metrics(applet.config.column_item_size);
        let description = app
            .comment
            .as_deref()
            .filter(|_| applet.config.column_show_description)
            .map(|comment| {
                text(comment)
                    .size(Self::DESCRIPTION_SIZE)
                    .wrapping(Wrapping::None)
                    .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)))
            });

        let button = cosmic::widget::button::custom(
            row![
                Self::create_icon_widget(app, icon_size),
                cosmic::widget::Space::new().width(5).height(Length::Fill),
                column![
                    text(&app.name)
                        .wrapping(Wrapping::None)
                        .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1))),
                ]
                .push_maybe(description),
            ]
            .align_y(Alignment::Center),
        )
        .on_press(Message::ApplicationSelected(app.clone()))
        .class(
            if applet.selected_item_index.is_some() && index == applet.selected_item_index.unwrap()
            {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::AppletMenu
            },
        )
        .width(Length::Fill)
        .height(item_height);

        // The context menu itself wraps the whole scrollable, see
        // `with_context_menu`; this only marks which app it is for.
        Self::drag_source(
            applet,
            app,
            cosmic::widget::mouse_area(button).on_right_press(Message::ContextMenuTarget(index)),
        )
    }

    /// Lets `app` be dragged out of `content`, e.g. into the favorites.
    pub(crate) fn drag_source<'a>(
        applet: &Applet,
        app: &ApplicationEntry,
        content: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        const DRAG_ICON_SIZE: u16 = 32;

        let dragged_app = DraggedApp {
            app_id: app.id.clone(),
        };
        let drag_icon = app.icon.clone();

        DndSource::new(content)
            .window(applet.popup.unwrap_or(Id::NONE))
            .drag_threshold(16.)
            .drag_content(move || dragged_app.clone())
            .drag_icon(move |_| {
                (
                    Self::icon_widget(drag_icon.as_ref(), DRAG_ICON_SIZE),
                    cosmic::iced::core::widget::tree::State::None,
                    // Center the icon on the pointer.
                    Vector::new(
                        -f32::from(DRAG_ICON_SIZE) / 2.,
                        -f32::from(DRAG_ICON_SIZE) / 2.,
                    ),
                )
            })
            .on_cancel(Some(Message::AppDragEnded))
            .on_finish(Some(Message::AppDragEnded))
            .into()
    }

    /// Creates the icon widget for an application
    ///
    /// # Arguments
    /// * `app` - The application entry
    /// * `space_l` - The space value for icon dimensions
    ///
    /// # Returns
    /// A container element with the app icon
    pub(crate) fn create_icon_widget<'a, M: 'a>(app: &ApplicationEntry, space_l: u16) -> Element<'a, M> {
        Self::icon_widget(app.icon.as_ref(), space_l)
    }

    /// Icon widget for `icon`, or the default icon without one.
    fn icon_widget<'a, M: 'a>(icon: Option<&IconHandle>, space_l: u16) -> Element<'a, M> {
        let default_icon = IconHandle::default();
        let icon_handle = icon.unwrap_or(&default_icon);

        match icon_handle {
            IconHandle::SvgHandle(handle) => container(
                cosmic::widget::svg(handle.clone())
                    .width(Length::Fixed(space_l.into()))
                    .height(Length::Fixed(space_l.into()))
                    .content_fit(ContentFit::Contain),
            )
            .into(),
            IconHandle::RasterHandle(handle) => container(
                cosmic::widget::image(handle.clone())
                    .width(Length::Fixed(space_l.into()))
                    .height(Length::Fixed(space_l.into()))
                    .content_fit(ContentFit::Contain),
            )
            .into(),
        }
    }

    /// Wraps a scrollable app view in the context menu of the app that was
    /// last right clicked in it.
    ///
    /// The context menu must sit outside the scrollable: inside, it would get
    /// the cursor position shifted by the scroll offset and open its popup
    /// that far below the pointer.
    pub(crate) fn with_context_menu<'a>(
        applet: &'a Applet,
        content: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        // Built by the applet when an app is right clicked.
        let context_menu = applet.context_menu.clone();
        let has_target = context_menu.is_some();

        // Reports right clicks that no app claimed, so empty space shows no
        // menu. Item handlers run before this one for the same click.
        let content =
            cosmic::widget::mouse_area(content).on_right_press(Message::ContextMenuRightPress);

        // The context menu panics without a menu, so it always gets one, and
        // the guard keeps it from opening when no app was right clicked.
        let context_menu = cosmic::widget::context_menu(content, Some(context_menu.unwrap_or_default()))
            .close_on_escape(true)
            .on_surface_action(Message::ContextMenuAction)
            .window_id(applet.popup.unwrap_or(Id::NONE));

        RightReleaseGuard::new(context_menu, !has_target).into()
    }
}
