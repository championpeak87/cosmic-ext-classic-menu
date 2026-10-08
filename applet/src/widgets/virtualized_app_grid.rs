// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;

use cosmic::iced::core::text::{Ellipsize, EllipsizeHeightLimit, Wrapping};
use cosmic::iced::widget::{column, row};
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{container, scrollable, text};
use cosmic::{Element, theme};

use crate::applet::{Applet, Message};
use crate::model::application_entry::ApplicationEntry;
use crate::widgets::VirtualizedAppList;

/// A virtualized grid of app icons, rendering only the rows in view.
///
/// Shares the scroll state and selection of [`VirtualizedAppList`], so only
/// one of them may be shown at a time.
pub struct VirtualizedAppGrid;

impl VirtualizedAppGrid {
    pub const COLUMNS: usize = 5;
    pub const ROW_HEIGHT: f32 = 88.0;
    const ICON_SIZE: u16 = 40;
    /// Buffer rows to render above/below viewport for smooth scrolling
    const RENDER_BUFFER: usize = 1;

    pub fn view(applet: &Applet) -> Element<'_, Message> {
        let total_rows = applet.available_applications.len().div_ceil(Self::COLUMNS);

        let visible_start = (applet.scroll_offset / Self::ROW_HEIGHT).floor() as usize;
        let viewport_height = applet.scroll_viewport_height.max(Self::ROW_HEIGHT);
        let visible_count = ((viewport_height / Self::ROW_HEIGHT).ceil() as usize) + 1;

        let render_start = visible_start.saturating_sub(Self::RENDER_BUFFER);
        let render_end = (visible_start + visible_count + Self::RENDER_BUFFER).min(total_rows);

        let mut rows: Vec<Element<'_, Message>> = Vec::new();

        // Spacers above and below keep the scroll position stable.
        if render_start > 0 {
            rows.push(Self::spacer(render_start));
        }

        for row_index in render_start..render_end {
            let first = row_index * Self::COLUMNS;
            let mut cells: Vec<Element<'_, Message>> = applet
                .available_applications
                .iter()
                .enumerate()
                .skip(first)
                .take(Self::COLUMNS)
                .map(|(index, app)| Self::create_app_cell(applet, index, app))
                .collect();
            // Keep cells of a partial last row the same width as the others.
            while cells.len() < Self::COLUMNS {
                cells.push(
                    cosmic::widget::Space::new()
                        .width(Length::FillPortion(1))
                        .into(),
                );
            }
            rows.push(
                row(cells)
                    .height(Length::Fixed(Self::ROW_HEIGHT))
                    .into(),
            );
        }

        if render_end < total_rows {
            rows.push(Self::spacer(total_rows - render_end));
        }

        let grid = scrollable(column(rows).width(Length::Fill))
            .height(Length::Fill)
            .width(Length::Fill)
            .id(applet.scrollable_id.clone())
            .on_scroll(Message::ScrollUpdated);

        VirtualizedAppList::with_context_menu(applet, grid)
    }

    fn spacer(rows: usize) -> Element<'static, Message> {
        cosmic::widget::Space::new()
            .width(Length::Fill)
            .height(Length::Fixed(rows as f32 * Self::ROW_HEIGHT))
            .into()
    }

    fn create_app_cell<'a>(
        applet: &'a Applet,
        index: usize,
        app: &'a Arc<ApplicationEntry>,
    ) -> Element<'a, Message> {
        let space_xxs = theme::active().cosmic().space_xxs();

        let button = cosmic::widget::button::custom(
            column![
                VirtualizedAppList::create_icon_widget(app, Self::ICON_SIZE),
                text(&app.name)
                    .size(12)
                    .wrapping(Wrapping::None)
                    .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)))
                    .align_x(Alignment::Center)
                    .width(Length::Fill),
            ]
            .spacing(space_xxs)
            .align_x(Alignment::Center),
        )
        .on_press(Message::ApplicationSelected(app.clone()))
        .class(if applet.selected_item_index == Some(index) {
            cosmic::theme::Button::Suggested
        } else {
            cosmic::theme::Button::AppletMenu
        })
        .padding(space_xxs)
        .width(Length::Fill)
        .height(Length::Fill);

        container(
            cosmic::widget::mouse_area(button)
                .on_right_press(Message::ContextMenuTarget(index)),
        )
        .padding(2)
        .width(Length::FillPortion(1))
        .height(Length::Fill)
        .into()
    }
}
