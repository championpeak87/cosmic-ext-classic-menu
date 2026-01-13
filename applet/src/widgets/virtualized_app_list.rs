// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;

use cosmic::cosmic_theme::Spacing;
use cosmic::iced::window::Id;
use cosmic::iced::{Alignment, ContentFit, Length};
use cosmic::iced::widget::{column, row};
use cosmic::widget::{container, ListColumn, scrollable};
use cosmic::widget::text;
use cosmic::{Element, theme};

use crate::applet::{Applet, Message};
use crate::model::application_entry::ApplicationEntry;

/// A virtualized app list widget that only renders visible items for performance.
/// 
/// This widget improves performance when dealing with large application lists
/// by only rendering items that are currently visible in the viewport.
pub struct VirtualizedAppList;

impl VirtualizedAppList {
    /// Buffer items to render above/below viewport for smooth scrolling
    const RENDER_BUFFER: usize = 2;

    /// Creates a virtualized app list element
    /// 
    /// # Arguments
    /// * `applet` - Reference to the applet containing app data and configuration
    /// 
    /// # Returns
    /// A scrollable element containing only the visible app items
    pub fn view(applet: &Applet) -> Element<'_, Message> {
        let Spacing {
            space_l, space_xl, ..
        } = theme::active().cosmic().spacing;

        // Use actual button height from theme spacing
        let item_height = space_xl as f32;
        let scroll_offset = applet.scroll_offset;
        let total_items = applet.available_applications.len();
        
        // Calculate which items should be rendered based on scroll position
        let visible_start = (scroll_offset / item_height).floor() as usize;
        let viewport_height = 600.0; // Approximate viewport height
        let visible_count = ((viewport_height / item_height).ceil() as usize) + 1;
        
        // Add buffer for smooth scrolling
        let render_start = visible_start.saturating_sub(Self::RENDER_BUFFER);
        let render_end = (visible_start + visible_count + Self::RENDER_BUFFER).min(total_items);

        // Build items to render
        let mut items: Vec<Element<'_, Message>> = Vec::new();

        // Add spacer above visible items to maintain scroll position
        if render_start > 0 {
            let spacer_height = (render_start as f32 * item_height) as u16;
            items.push(cosmic::widget::Space::new(Length::Fill, spacer_height).into());
        }

        // Add visible and buffered items
        for (original_index, app) in applet
            .available_applications
            .iter()
            .enumerate()
            .skip(render_start)
            .take(render_end - render_start)
        {
            items.push(Self::create_app_button(applet, original_index, app, space_l, space_xl));
        }

        // Add spacer below visible items
        if render_end < total_items {
            let remaining_height = ((total_items - render_end) as f32 * item_height) as u16;
            items.push(cosmic::widget::Space::new(Length::Fill, remaining_height).into());
        }

        // Build list column from items
        let app_list: ListColumn<Message> = items.into_iter().fold(
            cosmic::widget::list_column().padding([0., 0.]),
            |list, item| list.add(item),
        );

        scrollable(app_list)
            .height(Length::Fill)
            .width(Length::FillPortion(5))
            .id(applet.scrollable_id.clone())
            .on_scroll(|viewport| {
                Message::ScrollUpdated(viewport)
            })
            .into()
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
        app: &'a Arc<ApplicationEntry>,
        space_l: u16,
        space_xl: u16,
    ) -> Element<'a, Message> {
        // Show comment only if item height is sufficient (at least 60 pixels)
        let show_comment = space_xl >= 40;

        let button = cosmic::widget::button::custom(
            row![
                Self::create_icon_widget(app, space_l),
                cosmic::widget::Space::new(5, Length::Fill),
                if show_comment {
                    column![
                        text(&app.name),
                        text(app.comment.as_deref().unwrap_or_default()).size(8.0),
                    ]
                    .padding([0, 0])
                } else {
                    column![text(&app.name)]
                        .padding([0, 0])
                },
            ]
            .align_y(Alignment::Center),
        )
        .on_press(Message::ApplicationSelected(app.clone()))
        .class(if applet.selected_item_index.is_some()
            && index == applet.selected_item_index.unwrap()
        {
            cosmic::theme::Button::Suggested
        } else {
            cosmic::theme::Button::AppletMenu
        })
        .width(Length::Fill)
        .height(space_xl);

        let context_menu = Self::create_context_menu(applet, app);

        let widget = cosmic::widget::context_menu(button, context_menu)
            .close_on_escape(true)
            .on_surface_action(Message::ContextMenuAction)
            .window_id(applet.popup.unwrap_or_else(|| Id::NONE));

        widget.into()
    }

    /// Creates the icon widget for an application
    /// 
    /// # Arguments
    /// * `app` - The application entry
    /// * `space_l` - The space value for icon dimensions
    /// 
    /// # Returns
    /// A container element with the app icon
    fn create_icon_widget(
        app: &Arc<ApplicationEntry>,
        space_l: u16,
    ) -> Element<'_, Message> {
        let default_icon = crate::model::application_entry::IconHandle::default();
        let icon_handle = app.icon.as_ref().unwrap_or(&default_icon);
        
        match icon_handle {
            crate::model::application_entry::IconHandle::SvgHandle(handle) => {
                container(
                    cosmic::widget::svg(handle.clone())
                        .width(Length::Fixed(space_l.into()))
                        .height(Length::Fixed(space_l.into()))
                        .content_fit(ContentFit::Contain),
                )
                .into()
            }
            crate::model::application_entry::IconHandle::RasterHandle(handle) => {
                container(
                    cosmic::widget::image(handle.clone())
                        .width(Length::Fixed(space_l.into()))
                        .height(Length::Fixed(space_l.into()))
                        .content_fit(ContentFit::Contain),
                )
                .into()
            }
        }
    }

    /// Creates the context menu for an application
    /// 
    /// # Arguments
    /// * `applet` - Reference to the applet
    /// * `app` - The application entry
    /// 
    /// # Returns
    /// An optional context menu with app actions
    fn create_context_menu<'a>(
        applet: &'a Applet,
        app: &'a Arc<ApplicationEntry>,
    ) -> Option<Vec<cosmic::widget::menu::Tree<Message>>> {
        // Use cached menu trees if available (built in Applet when apps are updated)
        applet
            .context_menus
            .get(&app.id)
            .cloned()
    }
}
