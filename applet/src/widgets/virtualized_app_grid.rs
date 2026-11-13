// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;

use cosmic::iced::core::text::{Ellipsize, EllipsizeHeightLimit, Wrapping};
use cosmic::iced::widget::{column, row};
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{container, scrollable, text};
use cosmic::{Element, theme};

use crate::applet::{Applet, FavoritesRegion, Message};
use crate::applet_menu::{AppletMenu, FavoriteSlotAxis};
use crate::config::ItemSize;
use crate::layout::AppListOptions;
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

    pub fn metrics(options: &AppListOptions) -> GridMetrics {
        // Rows fit the icon and two lines of the name, so that names are
        // seldom cut off.
        let (columns, icon_size, row_height) = match options.item_size {
            ItemSize::Small => (5, 32, 88.0),
            ItemSize::Medium => (4, 40, 100.0),
            ItemSize::Large => (3, 56, 120.0),
        };
        GridMetrics {
            columns,
            icon_size,
            row_height,
        }
    }

    pub fn view(applet: &Applet) -> Element<'_, Message> {
        let metrics = Self::metrics(applet.config.app_list());
        // The apps leading all apps are set apart from them, all apps
        // starting on a row of their own.
        let layout = applet.app_rows();
        let total_rows = layout.rows();

        // Search results of pop-launcher lead the apps found.
        let header_height =
            AppletMenu::search_header_layout(applet).map_or(0., |layout| layout.height);
        let scroll_offset = (applet.scroll_offset - header_height).max(0.);
        let visible_start = layout.row_at(scroll_offset);
        let viewport_height = applet.render_viewport_height().max(metrics.row_height);
        let visible_count = ((viewport_height / metrics.row_height).ceil() as usize) + 1;

        let render_start = visible_start.saturating_sub(Self::RENDER_BUFFER);
        let render_end = (visible_start + visible_count + Self::RENDER_BUFFER).min(total_rows);
        // Drawn if between the rows rendered or right after them; otherwise
        // the spacers make room for it.
        let separator = layout
            .break_row()
            .filter(|row| render_start < *row && *row <= render_end);

        let mut rows: Vec<Element<'_, Message>> = Vec::new();

        // Spacers above and below keep the scroll position stable.
        if render_start > 0 {
            rows.push(Self::spacer(layout.top(render_start)));
        }

        for row_index in render_start..render_end {
            if separator == Some(row_index) {
                rows.push(AppletMenu::leading_separator());
            }
            let range = layout.row_range(row_index);
            let mut cells: Vec<Element<'_, Message>> = applet
                .available_applications
                .iter()
                .enumerate()
                .skip(range.start)
                .take(range.len())
                .map(|(index, app)| Self::create_app_cell(applet, index, app, metrics))
                .collect();
            // Keep cells of a partial row the same width as the others.
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
        if separator == Some(render_end) {
            rows.push(AppletMenu::leading_separator());
        }

        if render_end < total_rows {
            rows.push(Self::spacer(layout.height() - layout.top(render_end)));
        }

        let content = column![]
            .push_maybe(AppletMenu::search_header(applet))
            .push(column(rows).width(Length::Fill));
        let grid = scrollable(content)
            .height(Length::Fill)
            .width(Length::Fill)
            .id(applet.scrollable_id.clone())
            .on_scroll(Message::ScrollUpdated);

        VirtualizedAppList::with_context_menu(applet, grid)
    }

    fn spacer(height: f32) -> Element<'static, Message> {
        cosmic::widget::Space::new()
            .width(Length::Fill)
            .height(Length::Fixed(height))
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
                    .wrapping(Wrapping::WordOrGlyph)
                    .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(2)))
                    .align_x(Alignment::Center)
                    .width(Length::Fill),
            ]
            .spacing(space_xxs)
            .align_x(Alignment::Center),
        )
        .on_press(Message::ApplicationSelected(app.clone()))
        .class(crate::applet_menu::menu_item_class(
            applet.selected_item_index == Some(index),
        ))
        .padding(space_xxs)
        .width(Length::Fill)
        .height(Length::Fill);

        // Tiles have no room for descriptions, so they are shown on hover,
        // with the name, which may be cut off on the tile.
        let tile: Element<'a, Message> = match app
            .comment
            .as_deref()
            .filter(|_| applet.config.app_list().show_description)
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

        let cell = VirtualizedAppList::drag_source(
            applet,
            app,
            cosmic::widget::mouse_area(tile).on_right_press(Message::ContextMenuTarget(app.clone())),
        );
        let cell = AppletMenu::leading_app(applet, index, cell);
        let cell = if applet.favorite_rows().contains(&index) {
            AppletMenu::favorite_slot(
                applet,
                app,
                FavoritesRegion::List,
                FavoriteSlotAxis::Horizontal,
                (Length::Fill, Length::Fill),
                cell,
            )
        } else {
            cell
        };

        container(cell)
        .padding(2)
        .width(Length::FillPortion(1))
        .height(Length::Fill)
        .into()
    }
}
