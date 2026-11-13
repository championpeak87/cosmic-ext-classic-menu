// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;

use cosmic::iced::core::text::{Ellipsize, EllipsizeHeightLimit, Wrapping};
use cosmic::iced::widget::{column, row};
use cosmic::iced::window::Id;
use cosmic::iced::{Alignment, Background, ContentFit, Length, Vector};
use cosmic::widget::text;
use cosmic::widget::{DndSource, ListColumn, container, scrollable};
use cosmic::{Element, theme};

use crate::applet::{Applet, FavoritesRegion, Message};
use crate::applet_menu::{AppletMenu, FavoriteSlotAxis};
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
    /// Width and height of the icon shown while dragging an app.
    pub(crate) const DRAG_PREVIEW_SIZE: f32 = 40.0;
    /// Font size of app descriptions.
    pub(crate) const DESCRIPTION_SIZE: u16 = 10;

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
        // One app per row, the apps leading all apps set apart from them.
        let rows = applet.app_rows();
        // Search results of pop-launcher lead the apps found.
        let header_height =
            AppletMenu::search_header_layout(applet).map_or(0., |layout| layout.height);
        let scroll_offset = (applet.scroll_offset - header_height).max(0.);
        let total_items = rows.rows();

        // Calculate which items should be rendered based on scroll position
        let visible_start = rows.row_at(scroll_offset);
        let viewport_height = applet.render_viewport_height().max(rows.row_height);
        let visible_count = ((viewport_height / rows.row_height).ceil() as usize) + 1;

        // Add buffer for smooth scrolling
        let render_start = visible_start.saturating_sub(Self::RENDER_BUFFER);
        let render_end = (visible_start + visible_count + Self::RENDER_BUFFER).min(total_items);
        // The separator is drawn if it is between the rows rendered, or
        // right after them; otherwise the spacers make room for it.
        let separator = rows
            .break_row()
            .filter(|row| render_start < *row && *row <= render_end);

        // The items before the separator, and after it.
        let mut before: Vec<Element<'_, Message>> = Vec::new();
        let mut after: Vec<Element<'_, Message>> = Vec::new();

        // Add spacer above visible items to maintain scroll position
        if render_start > 0 {
            let spacer_height = rows.top(render_start) as u16;
            before.push(cosmic::widget::Space::new().width(Length::Fill).height(spacer_height).into());
        }

        // Add visible and buffered items
        for (original_index, app) in applet
            .available_applications
            .iter()
            .enumerate()
            .skip(render_start)
            .take(render_end - render_start)
        {
            let item = Self::create_app_button(applet, original_index, app);
            if separator.is_some_and(|row| original_index >= row) {
                after.push(item);
            } else {
                before.push(item);
            }
        }

        // Add spacer below visible items
        if render_end < total_items {
            let remaining_height = (rows.height() - rows.top(render_end)) as u16;
            let spacer = cosmic::widget::Space::new().width(Length::Fill).height(remaining_height);
            if separator.is_some() {
                after.push(spacer.into());
            } else {
                before.push(spacer.into());
            }
        }

        // Build list column from items. The buttons pad their content
        // themselves, so their highlight reaches the list's edges.
        fn list<'a>(items: Vec<Element<'a, Message>>) -> ListColumn<'a, Message> {
            items.into_iter().fold(
                cosmic::widget::list_column().list_item_padding(0),
                |list, item| list.add(item),
            )
        }
        let mut app_list = column![container(list(before))];
        if separator.is_some() {
            app_list = app_list
                .push(AppletMenu::leading_separator())
                .push(container(list(after)));
        }

        let content = column![]
            .push_maybe(AppletMenu::search_header(applet))
            .push(app_list);
        let app_list = scrollable(content)
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
    pub(crate) fn create_app_button<'a>(
        applet: &'a Applet,
        index: usize,
        app: &'a Arc<ApplicationEntry>,
    ) -> Element<'a, Message> {
        let options = applet.config.app_list();
        let (icon_size, item_height) = Self::item_metrics(options.item_size);
        let description = app
            .comment
            .as_deref()
            .filter(|_| options.show_description)
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
        .class(crate::applet_menu::menu_item_class(
            applet.selected_item_index == Some(index),
        ))
        .padding([0, theme::active().cosmic().space_s()])
        .width(Length::Fill)
        .height(item_height);

        // The context menu itself wraps the whole scrollable, see
        // `with_context_menu`; this only marks which app it is for.
        let row = Self::drag_source(
            applet,
            app,
            cosmic::widget::mouse_area(button).on_right_press(Message::ContextMenuTarget(app.clone())),
        );
        let row = AppletMenu::leading_app(applet, index, row);

        if applet.favorite_rows().contains(&index) {
            AppletMenu::favorite_slot(
                applet,
                app,
                FavoritesRegion::List,
                FavoriteSlotAxis::Vertical,
                (Length::Fill, Length::Shrink),
                row,
            )
        } else {
            row
        }
    }

    /// Lets `app` be dragged out of `content`, e.g. into the favorites or
    /// the dock, which pins it.
    pub(crate) fn drag_source<'a>(
        applet: &Applet,
        app: &ApplicationEntry,
        content: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        let dragged_app = DraggedApp {
            app_id: app.id.clone(),
            desktop_file: app.path.clone(),
        };
        let drag_icon = app.icon.clone();

        let source = DndSource::new(content)
            .window(applet.popup.unwrap_or(Id::NONE))
            .drag_threshold(16.)
            .drag_content(move || dragged_app.clone())
            .on_start(Some(Message::AppDragStarted(
                app.id.clone(),
                drag_icon.clone(),
            )))
            .on_cancel(Some(Message::AppDragEnded))
            .on_finish(Some(Message::AppDragEnded));

        // On a layer surface the menu draws it itself, see `drag_preview`:
        // the panel doesn't show the icons of the drags it forwards.
        if applet.menu_is_layer_surface {
            return source.into();
        }

        source
            .drag_icon(move |_| {
                (
                    Self::drag_preview(drag_icon.as_ref()),
                    cosmic::iced::core::widget::tree::State::None,
                    // Center the icon on the pointer.
                    Vector::new(-Self::DRAG_PREVIEW_SIZE / 2., -Self::DRAG_PREVIEW_SIZE / 2.),
                )
            })
            .into()
    }

    /// The icon of a dragged app, with a small plus at its corner.
    pub(crate) fn drag_preview<'a, M: Clone + 'static>(
        icon: Option<&IconHandle>,
    ) -> Element<'a, M> {
        const ICON_SIZE: u16 = 32;
        const BADGE_SIZE: f32 = 14.0;

        let badge = container(text::caption("+"))
            .width(Length::Fixed(BADGE_SIZE))
            .height(Length::Fixed(BADGE_SIZE))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .class(cosmic::theme::Container::custom(|theme| {
                let cosmic = theme.cosmic();
                container::Style {
                    background: Some(Background::Color(cosmic.accent_color().into())),
                    text_color: Some(cosmic.on_accent_color().into()),
                    border: cosmic::iced::Border {
                        radius: (BADGE_SIZE / 2.).into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }));

        let size = Length::Fixed(Self::DRAG_PREVIEW_SIZE);
        cosmic::iced::widget::stack![
            container(Self::icon_widget(icon, ICON_SIZE))
                .width(size)
                .height(size)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
            container(badge)
                .width(size)
                .height(size)
                .align_x(Alignment::End)
                .align_y(Alignment::End),
        ]
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
            .on_open(Message::ContextMenuOpenChanged(true))
            .on_close(Message::ContextMenuOpenChanged(false))
            .on_surface_action(Message::ContextMenuAction)
            // No window on a layer surface, so it is drawn in the menu: the
            // panel cannot attach popups to them.
            .window_id(if applet.menu_is_layer_surface {
                Id::NONE
            } else {
                applet.popup.unwrap_or(Id::NONE)
            });

        RightReleaseGuard::new(context_menu, !has_target)
            .capture_escape(applet.context_menu_open)
            .into()
    }
}
