// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;

use cosmic::iced::core::text::{Ellipsize, EllipsizeHeightLimit, Wrapping};
use cosmic::iced::widget::{column, row};
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{container, scrollable, text};
use cosmic::{Element, theme};

use crate::applet::{Applet, Message};
use crate::config::{AppletConfig, ItemSize};
use crate::model::application_entry::ApplicationEntry;
use crate::widgets::VirtualizedAppList;

/// A virtualized grid of app icons, rendering only the rows in view.
///
/// Shares the scroll state and selection of [`VirtualizedAppList`], so only
/// one of them may be shown at a time.
pub struct VirtualizedAppGrid;

/// Dimensions of the grid, which depend on the configured item size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridMetrics {
    pub columns: usize,
    pub icon_size: u16,
    pub row_height: f32,
}

impl VirtualizedAppGrid {
    /// Buffer rows to render above/below viewport for smooth scrolling
    const RENDER_BUFFER: usize = 1;
    /// Width at which app descriptions in tooltips wrap.
    const TOOLTIP_MAX_WIDTH: f32 = 280.0;

    pub fn metrics(config: &AppletConfig) -> GridMetrics {
        let (columns, icon_size, row_height) = match config.grid_item_size {
            ItemSize::Small => (6, 32, 72.0),
            ItemSize::Medium => (5, 40, 88.0),
            ItemSize::Large => (4, 56, 108.0),
        };
        GridMetrics {
            columns,
            icon_size,
            row_height,
        }
    }

    pub fn view(applet: &Applet) -> Element<'_, Message> {
        let metrics = Self::metrics(&applet.config);
        let total_rows = applet.available_applications.len().div_ceil(metrics.columns);

        let visible_start = (applet.scroll_offset / metrics.row_height).floor() as usize;
        let viewport_height = applet.scroll_viewport_height.max(metrics.row_height);
        let visible_count = ((viewport_height / metrics.row_height).ceil() as usize) + 1;

        let render_start = visible_start.saturating_sub(Self::RENDER_BUFFER);
        let render_end = (visible_start + visible_count + Self::RENDER_BUFFER).min(total_rows);

        let mut rows: Vec<Element<'_, Message>> = Vec::new();

        // Spacers above and below keep the scroll position stable.
        if render_start > 0 {
            rows.push(Self::spacer(render_start, metrics));
        }

        for row_index in render_start..render_end {
            let first = row_index * metrics.columns;
            let mut cells: Vec<Element<'_, Message>> = applet
                .available_applications
                .iter()
                .enumerate()
                .skip(first)
                .take(metrics.columns)
                .map(|(index, app)| Self::create_app_cell(applet, index, app, metrics))
                .collect();
            // Keep cells of a partial last row the same width as the others.
            while cells.len() < metrics.columns {
                cells.push(
                    cosmic::widget::Space::new()
                        .width(Length::FillPortion(1))
                        .into(),
                );
            }
            rows.push(
                row(cells)
                    .height(Length::Fixed(metrics.row_height))
                    .into(),
            );
        }

        if render_end < total_rows {
            rows.push(Self::spacer(total_rows - render_end, metrics));
        }

        let grid = scrollable(column(rows).width(Length::Fill))
            .height(Length::Fill)
            .width(Length::Fill)
            .id(applet.scrollable_id.clone())
            .on_scroll(Message::ScrollUpdated);

        VirtualizedAppList::with_context_menu(applet, grid)
    }

    fn spacer(rows: usize, metrics: GridMetrics) -> Element<'static, Message> {
        cosmic::widget::Space::new()
            .width(Length::Fill)
            .height(Length::Fixed(rows as f32 * metrics.row_height))
            .into()
    }

    fn create_app_cell<'a>(
        applet: &'a Applet,
        index: usize,
        app: &'a Arc<ApplicationEntry>,
        metrics: GridMetrics,
    ) -> Element<'a, Message> {
        let space_xxs = theme::active().cosmic().space_xxs();

        let button = cosmic::widget::button::custom(
            column![
                VirtualizedAppList::create_icon_widget(app, metrics.icon_size),
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

        // Tiles have no room for descriptions, so they are shown on hover,
        // with the name, which may be cut off on the tile.
        let tile: Element<'a, Message> = match app
            .comment
            .as_deref()
            .filter(|_| applet.config.grid_show_description)
        {
            Some(comment) => cosmic::widget::tooltip(
                button,
                container(
                    column![
                        text::body(&app.name).font(cosmic::font::bold()),
                        text::caption(comment),
                    ]
                    .spacing(space_xxs),
                )
                .max_width(Self::TOOLTIP_MAX_WIDTH),
                cosmic::widget::tooltip::Position::Bottom,
            )
            .into(),
            None => button.into(),
        };

        container(VirtualizedAppList::drag_source(
            applet,
            app,
            cosmic::widget::mouse_area(tile).on_right_press(Message::ContextMenuTarget(index)),
        ))
        .padding(2)
        .width(Length::FillPortion(1))
        .height(Length::Fill)
        .into()
    }
}
