// SPDX-License-Identifier: GPL-3.0-only

//! Drag and drop editor of the blocks the menu is made of, see
//! [`cosmic_ext_classic_menu_applet::layout`].
//!
//! Renders a miniature of the menu popup. A block can be picked up and
//! dropped next to another one: above or below it in its column, or in a
//! new column to its left or right. Dropped between rows, it gets a row of
//! its own. New blocks are dragged in from the palette below the preview.
//! Clicking a block selects it, so the settings can show its options.
//! Handles between columns resize them, handles around the miniature
//! resize the menu.
//!
//! Blocks are drawn as a sketch of the real menu (icons and placeholder
//! text lines) rather than an exact rendering.

use std::borrow::Cow;

use crate::fl;
use cosmic::iced::core::text::{Ellipsize, EllipsizeHeightLimit, Wrapping};
use cosmic::iced::{mouse, Alignment, Background, Border, Color, Length, Padding, Point, Vector};
use cosmic::widget::{container, icon, mouse_area, text, tooltip, Space};
use cosmic::{Element, Theme};
use cosmic_ext_classic_menu_applet::config::{
    AppView, AppletConfig, FeaturedApplications, ItemSize, MenuLayout, Place,
};
use cosmic_ext_classic_menu_applet::layout::{
    AppListOptions, Block, BlockKind, BlockPath, Column, Drop, FeaturedOptions, Layout,
    PowerOptions, Row, Side, SidePanelContent, SidePanelOptions,
};
use cosmic_ext_classic_menu_applet::model::application_category::{ApplicationCategory, CategoryIcon};
use cosmic_ext_classic_menu_applet::model::power_action::PowerAction;

/// Size of the preview relative to the real menu.
const PREVIEW_SCALE: f32 = 0.5;
/// Cards of the layouts to pick per row, see `layout_picker`.
const LAYOUT_CARDS_PER_ROW: usize = 4;
/// Height of two lines of a caption, which a card's description takes.
const CARD_DESCRIPTION_HEIGHT: f32 = 32.0;
/// The menu's width and height are multiples of this, in pixels.
const MENU_SIZE_STEP: u16 = 10;
/// Thickness of the handles resizing the menu.
const MENU_HANDLE_SIZE: f32 = 10.0;
/// Length of the grip drawn on the handles along the preview's edges.
const MENU_GRIP_LENGTH: f32 = 36.0;
const BAR_HEIGHT: f32 = 26.0;
const GHOST_WIDTH: f32 = 110.0;
const GHOST_HEIGHT: f32 = 28.0;
const RESIZE_HANDLE_WIDTH: f32 = 9.0;
/// Width of narrow columns of icons, like the favorites bar.
const SIDEBAR_WIDTH: f32 = 22.0;
/// Width of the user block.
const USER_WIDTH: f32 = 72.0;
/// Height of the drop targets between rows.
const ROW_GAP_HEIGHT: f32 = 10.0;
/// How far the cursor moves before a press becomes a drag.
const DRAG_THRESHOLD: f32 = 4.0;

const USER_ICON: &[u8] = include_bytes!("../../res/icons/bundled/user-idle-symbolic.svg");
const STAR_ICON: &[u8] = include_bytes!("../../res/icons/bundled/starred-symbolic.svg");

/// Placeholder text widths for the sketched app list rows.
const APP_NAME_WIDTHS: [f32; 9] = [70.0, 54.0, 82.0, 46.0, 64.0, 76.0, 50.0, 68.0, 58.0];
/// Categories shown in the sketch, in the same order as the applet.
const CATEGORIES: [ApplicationCategory; 8] = [
    ApplicationCategory::AUDIO,
    ApplicationCategory::DEVELOPMENT,
    ApplicationCategory::GAMES,
    ApplicationCategory::GRAPHICS,
    ApplicationCategory::NETWORK,
    ApplicationCategory::OFFICE,
    ApplicationCategory::SYSTEM,
    ApplicationCategory::UTILITY,
];

/// Icon of a power action, as in the applet.
pub fn power_icon(action: PowerAction) -> &'static [u8] {
    match action {
        PowerAction::Logout => include_bytes!("../../res/icons/bundled/system-log-out-symbolic.svg"),
        PowerAction::Suspend => include_bytes!("../../res/icons/bundled/system-suspend-symbolic.svg"),
        PowerAction::Hibernate => include_bytes!("../../res/icons/bundled/system-hibernate-symbolic.svg"),
        PowerAction::Lock => include_bytes!("../../res/icons/bundled/system-lock-screen-symbolic.svg"),
        PowerAction::Reboot => include_bytes!("../../res/icons/bundled/system-reboot-symbolic.svg"),
        PowerAction::Shutdown => include_bytes!("../../res/icons/bundled/system-shutdown-symbolic.svg"),
    }
}

/// Name of a block, as shown in the settings.
pub fn block_label(block: &Block) -> String {
    match block {
        Block::SidePanel(options) => match options.content {
            SidePanelContent::Categories => fl!("layout-categories"),
            SidePanelContent::Places => fl!("layout-places"),
        },
        block => kind_label(block.kind()),
    }
}

/// Name of a kind of block, as shown in the settings.
pub fn kind_label(kind: BlockKind) -> String {
    match kind {
        BlockKind::User => fl!("layout-user"),
        BlockKind::Search => fl!("layout-search-field"),
        BlockKind::AppList => fl!("layout-app-list"),
        BlockKind::SidePanel => fl!("layout-side-panel"),
        BlockKind::Power => fl!("layout-power"),
        BlockKind::Featured => fl!("layout-featured"),
        BlockKind::Spacer => fl!("layout-spacer"),
        BlockKind::Divider => fl!("layout-divider"),
    }
}

fn kind_icon(kind: BlockKind) -> icon::Icon {
    match kind {
        BlockKind::User => icon::from_svg_bytes(USER_ICON).symbolic(true).icon(),
        BlockKind::Search => icon::from_name("system-search-symbolic").icon(),
        BlockKind::AppList => icon::from_name("view-app-grid-symbolic").icon(),
        BlockKind::SidePanel => icon::from_name("view-list-symbolic").icon(),
        BlockKind::Power => icon::from_svg_bytes(power_icon(PowerAction::Shutdown))
            .symbolic(true)
            .icon(),
        BlockKind::Featured => icon::from_svg_bytes(STAR_ICON).symbolic(true).icon(),
        BlockKind::Spacer => icon::from_name("view-more-horizontal-symbolic").icon(),
        BlockKind::Divider => icon::from_name("list-remove-symbolic").icon(),
    }
}

/// What is being dragged.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Dragged {
    /// A block of the menu.
    Block(BlockPath),
    /// A new block, from the palette.
    New(BlockKind),
}

/// A press on a block or the palette, which becomes a drag once the cursor
/// moves far enough.
#[derive(Debug, Clone, Copy)]
struct Drag {
    dragged: Dragged,
    /// Where the cursor was first seen in the window.
    origin: Option<Point>,
    active: bool,
}

/// A handle resizing the menu.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MenuHandle {
    Right,
    Bottom,
    BottomRight,
}

impl MenuHandle {
    fn interaction(self) -> mouse::Interaction {
        match self {
            MenuHandle::Right => mouse::Interaction::ResizingHorizontally,
            MenuHandle::Bottom => mouse::Interaction::ResizingVertically,
            MenuHandle::BottomRight => mouse::Interaction::ResizingDiagonallyDown,
        }
    }
}

/// A resize of the menu in progress.
#[derive(Debug, Clone, Copy)]
struct MenuResize {
    handle: MenuHandle,
    /// Size of the menu when the handle was grabbed.
    start_size: (u16, u16),
    /// Where the cursor first moved, which the drag is measured from.
    origin: Option<Point>,
    size: (u16, u16),
}

impl MenuResize {
    /// Size of the menu with the cursor moved by `delta` in the preview.
    fn size_at(&self, delta: Vector) -> (u16, u16) {
        let (start_width, start_height) = self.start_size;
        let (mut width, mut height) = self.start_size;
        if self.handle != MenuHandle::Bottom {
            // The preview is centered, so its edge moves by half of what its
            // width changes by. Twice the distance keeps it under the cursor.
            width = snapped(
                f32::from(start_width) + 2.0 * delta.x / PREVIEW_SCALE,
                AppletConfig::MENU_WIDTH_RANGE,
            );
        }
        if self.handle != MenuHandle::Right {
            height = snapped(
                f32::from(start_height) + delta.y / PREVIEW_SCALE,
                AppletConfig::MENU_HEIGHT_RANGE,
            );
        }
        (width, height)
    }
}

/// A resize of two neighbouring columns in progress.
#[derive(Debug, Clone, Copy)]
struct ColumnResize {
    row: usize,
    /// The left one of the two columns.
    left: usize,
    /// Share of the left column, in percent, when the handle was grabbed.
    start_share: u16,
    /// Width of both columns in the preview.
    width: f32,
    origin: Option<Point>,
    share: u16,
}

/// `value` rounded to a [`MENU_SIZE_STEP`] within `range`.
fn snapped(value: f32, range: std::ops::RangeInclusive<u16>) -> u16 {
    let step = f32::from(MENU_SIZE_STEP);
    (((value / step).round() * step) as u16).clamp(*range.start(), *range.end())
}

#[derive(Debug, Clone)]
pub enum Message {
    PresetSelected(MenuLayout),
    /// The layout made by the user was picked again.
    CustomSelected,
    DragStart(Dragged),
    /// The cursor moved to this position over the preview.
    PreviewMove(Point),
    DragEnter(Drop),
    DragLeave(Drop),
    /// The handle between the column at `left` and the next one of `row`
    /// was grabbed.
    ResizeStart { row: usize, left: usize },
    /// A handle resizing the menu was grabbed.
    MenuResizeStart(MenuHandle),
    /// The cursor moved to this position in the window.
    CursorMoved(Point),
    /// Left mouse button released anywhere in the window.
    DragEnd,
}

/// Change resulting from picking a layout or completing a drag.
#[derive(Debug, Clone)]
pub enum Change {
    Layout(Layout),
    /// Width and height of the menu in pixels.
    MenuSize(u16, u16),
}

#[derive(Debug, Default)]
pub struct LayoutEditor {
    drag: Option<Drag>,
    hovered: Option<Drop>,
    cursor: Option<Point>,
    /// The block whose options are shown.
    selected: Option<BlockPath>,
    resizing: Option<ColumnResize>,
    menu_resize: Option<MenuResize>,
}

impl LayoutEditor {
    /// Whether the mouse button is held on the editor, so it needs to hear
    /// of the cursor's moves and the button's release anywhere.
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some() || self.resizing.is_some() || self.menu_resize.is_some()
    }

    /// Whether a block is dragged, rather than just pressed.
    fn drag_active(&self) -> bool {
        self.drag.is_some_and(|drag| drag.active)
    }

    /// The selected block, if it still is in `layout`.
    pub fn selected(&self, layout: &Layout) -> Option<BlockPath> {
        self.selected.filter(|path| layout.get(*path).is_some())
    }

    pub fn select(&mut self, path: Option<BlockPath>) {
        self.selected = path;
    }

    /// Size of the menu in pixels, following a resize in progress.
    fn menu_size(&self, config: &AppletConfig) -> (u16, u16) {
        self.menu_resize
            .map_or_else(|| config.menu_size(), |resize| resize.size)
    }

    /// Width the editor needs: the miniature of the menu and the handles
    /// along its right edge, following a resize in progress.
    pub fn width(&self, config: &AppletConfig) -> f32 {
        self.preview_size(config).0 + MENU_HANDLE_SIZE
    }

    /// Width the editor needs for the saved size of the menu, ignoring a
    /// resize in progress.
    pub fn saved_width(config: &AppletConfig) -> f32 {
        f32::from(config.menu_size().0) * PREVIEW_SCALE + MENU_HANDLE_SIZE
    }

    /// Size of the miniature of the menu.
    fn preview_size(&self, config: &AppletConfig) -> (f32, f32) {
        let (width, height) = self.menu_size(config);
        (f32::from(width) * PREVIEW_SCALE, f32::from(height) * PREVIEW_SCALE)
    }

    /// The layout shown, following a resize of columns in progress.
    fn shown_layout<'a>(&self, config: &'a AppletConfig) -> Cow<'a, Layout> {
        match self.resizing {
            Some(resize) => {
                let mut layout = config.layout.clone();
                layout.resize_columns(resize.row, resize.left, resize.share);
                Cow::Owned(layout)
            }
            None => Cow::Borrowed(&config.layout),
        }
    }

    pub fn update(&mut self, message: Message, config: &AppletConfig) -> Option<Change> {
        match message {
            Message::PresetSelected(preset) => {
                self.selected = None;
                let layout = preset.layout();
                (layout != config.layout).then_some(Change::Layout(layout))
            }
            Message::CustomSelected => {
                self.selected = None;
                config
                    .custom_layout
                    .clone()
                    .filter(|layout| *layout != config.layout)
                    .map(Change::Layout)
            }
            Message::DragStart(dragged) => {
                self.drag = Some(Drag {
                    dragged,
                    origin: None,
                    active: false,
                });
                self.hovered = None;
                self.cursor = None;
                None
            }
            Message::PreviewMove(point) => {
                if self.drag.is_some() {
                    self.cursor = Some(point);
                }
                None
            }
            Message::DragEnter(drop) => {
                self.hovered = Some(drop);
                None
            }
            Message::DragLeave(drop) => {
                // Enter/leave of neighbouring targets may arrive in either order.
                if self.hovered == Some(drop) {
                    self.hovered = None;
                }
                None
            }
            Message::ResizeStart { row, left } => {
                let layout = &config.layout;
                let columns = &layout.rows.get(row)?.columns;
                let (a, b) = (columns.get(left)?.portion, columns.get(left + 1)?.portion);
                let fill_total: u16 = columns
                    .iter()
                    .filter(|column| column.fills_width())
                    .map(|column| column.portion)
                    .sum();
                let share = (u32::from(a) * 100 / u32::from((a + b).max(1))) as u16;
                self.resizing = Some(ColumnResize {
                    row,
                    left,
                    start_share: share,
                    width: self.preview_size(config).0 * f32::from(a + b)
                        / f32::from(fill_total.max(1)),
                    origin: None,
                    share,
                });
                None
            }
            Message::MenuResizeStart(handle) => {
                let size = config.menu_size();
                self.menu_resize = Some(MenuResize {
                    handle,
                    start_size: size,
                    origin: None,
                    size,
                });
                None
            }
            Message::CursorMoved(position) => {
                if let Some(resize) = &mut self.menu_resize {
                    let origin = *resize.origin.get_or_insert(position);
                    resize.size = resize.size_at(position - origin);
                }
                if let Some(resize) = &mut self.resizing {
                    let origin = *resize.origin.get_or_insert(position);
                    let delta = (position.x - origin.x) / resize.width.max(1.0) * 100.0;
                    resize.share = (f32::from(resize.start_share) + delta)
                        .round()
                        .clamp(f32::from(Column::MIN_SHARE), f32::from(100 - Column::MIN_SHARE))
                        as u16;
                }
                if let Some(drag) = &mut self.drag {
                    let origin = *drag.origin.get_or_insert(position);
                    let moved = position - origin;
                    if moved.x.hypot(moved.y) > DRAG_THRESHOLD {
                        drag.active = true;
                    }
                }
                None
            }
            Message::DragEnd => {
                if let Some(MenuResize {
                    size: (width, height),
                    ..
                }) = self.menu_resize.take()
                {
                    return ((width, height) != config.menu_size())
                        .then_some(Change::MenuSize(width, height));
                }

                if self.resizing.is_some() {
                    let layout = self.shown_layout(config).into_owned();
                    self.resizing = None;
                    return (layout != config.layout).then_some(Change::Layout(layout));
                }

                let drag = self.drag.take()?;
                self.cursor = None;
                let hovered = self.hovered.take();
                if !drag.active {
                    // A click selects the block.
                    if let Dragged::Block(path) = drag.dragged {
                        self.selected = Some(path);
                    }
                    return None;
                }

                let drop = hovered?;
                let mut layout = config.layout.clone();
                let placed = match drag.dragged {
                    Dragged::Block(from) => layout.move_block(from, drop),
                    Dragged::New(kind) => {
                        if kind.is_unique() && layout.contains(kind) {
                            return None;
                        }
                        layout.insert(kind.block(), drop)
                    }
                }?;
                self.selected = Some(placed);
                (layout != config.layout).then_some(Change::Layout(layout))
            }
        }
    }

    pub fn view(&self, config: &AppletConfig) -> Element<'static, Message> {
        let spacing = cosmic::theme::active().cosmic().spacing;
        let layout = self.shown_layout(config);

        let (width, height) = self.preview_size(config);
        let frame = container(self.rows(&layout))
            .padding(spacing.space_xxs)
            .width(Length::Fixed(width))
            .height(Length::Fixed(height))
            .clip(true)
            .class(cosmic::theme::Container::custom(frame_style));

        let mut preview = cosmic::iced::widget::stack![frame];
        if let (Some(drag), Some(cursor)) = (self.drag.filter(|drag| drag.active), self.cursor) {
            let label = match drag.dragged {
                Dragged::Block(path) => layout.get(path).map_or_else(String::new, block_label),
                Dragged::New(kind) => kind_label(kind),
            };
            preview = preview.push(ghost(label, cursor, (width, height)));
        }

        let preview = mouse_area(preview)
            .on_move(Message::PreviewMove)
            .interaction(match self.menu_resize {
                Some(resize) => resize.handle.interaction(),
                None if self.resizing.is_some() => mouse::Interaction::ResizingHorizontally,
                None if self.drag_active() => mouse::Interaction::Grabbing,
                None => mouse::Interaction::None,
            });

        // Handles along the edges that do not stay in place when resizing.
        let preview = cosmic::iced::widget::column![
            cosmic::iced::widget::row![
                preview,
                self.menu_handle(MenuHandle::Right, MENU_HANDLE_SIZE, height),
            ],
            cosmic::iced::widget::row![
                self.menu_handle(MenuHandle::Bottom, width, MENU_HANDLE_SIZE),
                self.menu_handle(MenuHandle::BottomRight, MENU_HANDLE_SIZE, MENU_HANDLE_SIZE),
            ],
        ];

        let (menu_width, menu_height) = self.menu_size(config);

        cosmic::iced::widget::column![
            // Above the preview, so it stays in place when resizing.
            text::body(format!("{menu_width} × {menu_height} px")),
            preview,
            self.palette(&layout),
            text::caption(fl!("layout-hint")),
            // Below the editor, where picking a whole layout goes after
            // editing this one.
            layout_picker(config),
        ]
        .spacing(spacing.space_s)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .into()
    }

    /// The rows of the menu, with drop targets between them while dragging.
    fn rows(&self, layout: &Layout) -> Element<'static, Message> {
        let mut items = Vec::new();
        for (index, row) in layout.rows.iter().enumerate() {
            if self.drag_active() {
                items.push(self.row_gap(index));
            }
            items.push(self.row(index, row));
        }
        if self.drag_active() {
            items.push(self.row_gap(layout.rows.len()));
        }

        cosmic::widget::column::with_children(items)
            .spacing(space_xxs())
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// A drop target making a new row at `index`.
    fn row_gap(&self, index: usize) -> Element<'static, Message> {
        let drop = Drop::NewRow(index);
        let hovered = self.hovered == Some(drop);
        let line = container(Space::new())
            .width(Length::Fill)
            .height(Length::Fixed(3.0))
            .class(cosmic::theme::Container::custom(move |theme: &Theme| {
                let cosmic = theme.cosmic();
                container::Style {
                    background: Some(Background::Color(if hovered {
                        cosmic.accent_color().into()
                    } else {
                        cosmic.bg_divider().into()
                    })),
                    border: Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }));

        mouse_area(
            container(line)
                .center_y(Length::Fixed(ROW_GAP_HEIGHT))
                .width(Length::Fill),
        )
        .on_enter(Message::DragEnter(drop))
        .on_exit(Message::DragLeave(drop))
        .into()
    }

    fn row(&self, row_index: usize, row: &Row) -> Element<'static, Message> {
        let tall = row.fills_height();
        let alone = row.columns.len() == 1 && row.columns[0].blocks.len() == 1;

        let mut items: Vec<Element<'static, Message>> = Vec::new();
        for (index, column) in row.columns.iter().enumerate() {
            if index > 0 && tall {
                let previous = &row.columns[index - 1];
                items.push(if previous.fills_width() && column.fills_width() {
                    self.resize_handle(row_index, index - 1)
                } else {
                    vertical_line()
                });
            }
            let path = |block| BlockPath {
                row: row_index,
                column: index,
                block,
            };
            let blocks: Vec<Element<'static, Message>> = column
                .blocks
                .iter()
                .enumerate()
                .map(|(block_index, block)| {
                    let context = SketchContext {
                        tall_row: tall,
                        wide_column: column.fills_width(),
                        alone,
                        banner: row.banner,
                    };
                    let block = self.block(path(block_index), block, context);
                    let padding = column.padding(block_index);
                    if padding.is_zero() {
                        return block;
                    }
                    // Scaled as the rest of the miniature.
                    let scaled = |side: u16| f32::from(side) * PREVIEW_SCALE;
                    container(block)
                        .padding(Padding {
                            top: scaled(padding.top),
                            right: scaled(padding.right),
                            bottom: scaled(padding.bottom),
                            left: scaled(padding.left),
                        })
                        .into()
                })
                .collect();
            items.push(
                cosmic::widget::column::with_children(blocks)
                    .spacing(space_xxs())
                    .width(if column.fills_width() {
                        Length::FillPortion(column.portion.max(1))
                    } else {
                        Length::Shrink
                    })
                    .height(if tall { Length::Fill } else { Length::Shrink })
                    .into(),
            );
        }

        let content = cosmic::widget::row::with_children(items)
            .spacing(space_xxs())
            .width(Length::Fill)
            .height(if tall { Length::Fill } else { Length::Shrink });

        if row.banner {
            container(content)
                .padding(2)
                .width(Length::Fill)
                .height(if tall { Length::Fill } else { Length::Shrink })
                .class(cosmic::theme::Container::custom(banner_style))
                .into()
        } else {
            content.into()
        }
    }

    /// A block, which can be picked up, selected, and dropped on.
    fn block(
        &self,
        path: BlockPath,
        block: &Block,
        context: SketchContext,
    ) -> Element<'static, Message> {
        let picked_up = self.drag_active() && self.drag.is_some_and(|drag| drag.dragged == Dragged::Block(path));
        let selected = self.selected == Some(path) && !self.drag_active();
        let (width, height) = block_size(block, context);

        let content: Element<'static, Message> = if picked_up {
            Space::new().into()
        } else {
            sketch(block, context)
        };
        let boxed = container(content)
            .width(width)
            .height(height)
            .clip(true)
            .class(cosmic::theme::Container::custom(move |theme| {
                slot_style(theme, !picked_up, selected)
            }));

        let content: Element<'static, Message> = if self.drag_active() && !picked_up {
            cosmic::iced::widget::stack![boxed, self.drop_zones(path)].into()
        } else if !self.is_dragging() {
            // Name the block on hover, as the sketch alone may be ambiguous.
            tooltip(boxed, text::caption(block_label(block)), tooltip::Position::Bottom).into()
        } else {
            boxed.into()
        };

        let mut area = mouse_area(content).on_press(Message::DragStart(Dragged::Block(path)));
        if !self.is_dragging() {
            area = area.interaction(mouse::Interaction::Grab);
        }
        area.into()
    }

    /// Targets over a block, for dropping another one on each of its sides.
    fn drop_zones(&self, path: BlockPath) -> Element<'static, Message> {
        let zone = |side: Side| -> Element<'static, Message> {
            let drop = Drop::Beside { path, side };
            let hovered = self.hovered == Some(drop);
            // A bar at the side the block goes to.
            let bar = container(Space::new())
                .width(match side {
                    Side::Above | Side::Below => Length::Fill,
                    Side::Before | Side::After => Length::Fixed(3.0),
                })
                .height(match side {
                    Side::Above | Side::Below => Length::Fixed(3.0),
                    Side::Before | Side::After => Length::Fill,
                })
                .class(cosmic::theme::Container::custom(move |theme: &Theme| {
                    container::Style {
                        background: hovered
                            .then(|| Background::Color(theme.cosmic().accent_color().into())),
                        ..Default::default()
                    }
                }));
            let zone = container(bar)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(match side {
                    Side::After => Alignment::End,
                    _ => Alignment::Start,
                })
                .align_y(match side {
                    Side::Below => Alignment::End,
                    _ => Alignment::Start,
                })
                .class(cosmic::theme::Container::custom(move |theme: &Theme| {
                    let mut accent: Color = theme.cosmic().accent_color().into();
                    accent.a = 0.2;
                    container::Style {
                        background: hovered.then_some(Background::Color(accent)),
                        ..Default::default()
                    }
                }));
            mouse_area(zone)
                .on_enter(Message::DragEnter(drop))
                .on_exit(Message::DragLeave(drop))
                .into()
        };

        cosmic::iced::widget::row![
            container(zone(Side::Before)).width(Length::FillPortion(1)).height(Length::Fill),
            cosmic::iced::widget::column![
                container(zone(Side::Above)).width(Length::Fill).height(Length::FillPortion(1)),
                container(zone(Side::Below)).width(Length::Fill).height(Length::FillPortion(1)),
            ]
            .width(Length::FillPortion(2))
            .height(Length::Fill),
            container(zone(Side::After)).width(Length::FillPortion(1)).height(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    /// The blocks to drag into the menu. Blocks the menu holds one of at
    /// most are left out while it has one.
    fn palette(&self, layout: &Layout) -> Element<'static, Message> {
        let chips = BlockKind::ALL
            .into_iter()
            .filter(|kind| !(kind.is_unique() && layout.contains(*kind)))
            .map(|kind| -> Element<'static, Message> {
                let chip = container(
                    cosmic::iced::widget::row![kind_icon(kind).size(14), text::caption(kind_label(kind))]
                        .spacing(space_xxs())
                        .align_y(Alignment::Center),
                )
                .padding([4, 8])
                .class(cosmic::theme::Container::custom(chip_style));
                let mut area = mouse_area(chip).on_press(Message::DragStart(Dragged::New(kind)));
                if !self.is_dragging() {
                    area = area.interaction(mouse::Interaction::Grab);
                }
                area.into()
            })
            .collect();

        cosmic::iced::widget::column![
            text::caption(fl!("layout-add-blocks")),
            cosmic::widget::flex_row(chips)
                .row_spacing(space_xxs())
                .column_spacing(space_xxs()),
        ]
        .spacing(space_xxs())
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .into()
    }

    /// A handle next to the preview, dragged to resize the menu.
    fn menu_handle(&self, handle: MenuHandle, width: f32, height: f32) -> Element<'static, Message> {
        let active = self.menu_resize.is_some_and(|resize| resize.handle == handle);
        let (grip_width, grip_height) = match handle {
            MenuHandle::Right => (3.0, MENU_GRIP_LENGTH),
            MenuHandle::Bottom => (MENU_GRIP_LENGTH, 3.0),
            MenuHandle::BottomRight => (6.0, 6.0),
        };
        let grip = container(Space::new())
            .width(Length::Fixed(grip_width))
            .height(Length::Fixed(grip_height))
            .class(cosmic::theme::Container::custom(move |theme: &Theme| {
                accent_or_divider(theme, active, 2.0)
            }));

        let area = mouse_area(
            container(grip).center_x(Length::Fixed(width)).center_y(Length::Fixed(height)),
        )
        .on_press(Message::MenuResizeStart(handle))
        .interaction(handle.interaction());

        if self.is_dragging() {
            area.into()
        } else {
            tooltip(area, text::caption(fl!("layout-resize-menu")), tooltip::Position::Bottom).into()
        }
    }

    /// The divider between two columns, dragged to change their widths.
    fn resize_handle(&self, row: usize, left: usize) -> Element<'static, Message> {
        let resizing = self
            .resizing
            .is_some_and(|resize| resize.row == row && resize.left == left);
        let line = container(Space::new())
            .width(Length::Fixed(if resizing { 2.0 } else { 1.0 }))
            .height(Length::Fill)
            .class(cosmic::theme::Container::custom(move |theme: &Theme| {
                accent_or_divider(theme, resizing, 0.0)
            }));

        // Wider than the line, so it is easy to grab.
        let handle = mouse_area(
            container(line)
                .center_x(Length::Fixed(RESIZE_HANDLE_WIDTH))
                .height(Length::Fill),
        )
        .on_press(Message::ResizeStart { row, left })
        .interaction(mouse::Interaction::ResizingHorizontally);

        if self.is_dragging() {
            handle.into()
        } else {
            tooltip(handle, text::caption(fl!("layout-resize-panes")), tooltip::Position::Bottom)
                .into()
        }
    }
}

/// Where a block is sketched, which tells how it is laid out, as in the
/// applet.
#[derive(Clone, Copy, Debug)]
struct SketchContext {
    tall_row: bool,
    wide_column: bool,
    alone: bool,
    banner: bool,
}

/// Size of the sketch of `block`.
fn block_size(block: &Block, context: SketchContext) -> (Length, Length) {
    let bar = Length::Fixed(BAR_HEIGHT);
    match block {
        Block::User(_) => (Length::Fixed(USER_WIDTH), bar),
        Block::Search(options) => (
            Length::Fill,
            Length::Fixed(match options.size {
                ItemSize::Small => BAR_HEIGHT - 4.0,
                ItemSize::Medium => BAR_HEIGHT,
                ItemSize::Large => BAR_HEIGHT + 6.0,
            }),
        ),
        Block::AppList(_) | Block::SidePanel(_) => (Length::Fill, Length::Fill),
        Block::Power(options) => {
            let length = options.actions.len().max(1) as f32 * 18.0 + 8.0;
            if options.vertical {
                (Length::Fixed(SIDEBAR_WIDTH), Length::Fixed(length))
            } else if context.wide_column || context.alone {
                (Length::Fill, bar)
            } else {
                (Length::Fixed(length), bar)
            }
        }
        Block::Featured(options) => {
            if options.vertical {
                (Length::Fixed(SIDEBAR_WIDTH), Length::Fill)
            } else {
                let rows = options.count().div_ceil(4).min(3) as f32;
                let heading = if options.show_heading { 12.0 } else { 0.0 };
                (Length::Fill, Length::Fixed(heading + rows * 16.0 + 8.0))
            }
        }
        Block::Spacer => {
            if context.tall_row {
                (Length::Fill, Length::Fill)
            } else {
                (Length::Fill, bar)
            }
        }
        Block::Divider => (Length::Fill, Length::Fixed(8.0)),
    }
}

/// A rough drawing of what the block looks like in the applet popup.
fn sketch(block: &Block, context: SketchContext) -> Element<'static, Message> {
    let spacing = cosmic::theme::active().cosmic().spacing;

    match block {
        Block::User(options) => {
            let avatar = options
                .show_avatar
                .then(|| icon::from_svg_bytes(USER_ICON).symbolic(true).icon().size(16));
            container(
                cosmic::iced::widget::row![]
                    .push_maybe(avatar)
                    .push(text_line(Length::Fixed(40.0), 6.0))
                    .spacing(space_xxs())
                    .align_y(Alignment::Center),
            )
            .center_y(Length::Fill)
            .padding([0, space_xxs()])
            .into()
        }

        Block::Search(_) => container(
            container(
                cosmic::iced::widget::row![
                    icon::from_name("system-search-symbolic").size(12),
                    text_line(Length::Fixed(72.0), 5.0),
                ]
                .spacing(spacing.space_xxs)
                .align_y(Alignment::Center),
            )
            .center_y(Length::Fill)
            .width(Length::Fill)
            .padding([0, spacing.space_xs])
            .class(cosmic::theme::Container::custom(search_style)),
        )
        .padding(3)
        .into(),

        Block::AppList(options) => apps_sketch(options),

        Block::SidePanel(options) => side_panel_sketch(options),

        Block::Power(options) => power_sketch(options, context.banner),

        Block::Featured(options) => featured_sketch(options),

        Block::Spacer => container(
            icon::from_name("view-more-horizontal-symbolic").size(10),
        )
        .center(Length::Fill)
        .class(cosmic::theme::Container::custom(fixed_style))
        .into(),

        Block::Divider => container(cosmic::widget::divider::horizontal::light())
            .center_y(Length::Fill)
            .padding([0, 4])
            .into(),
    }
}

fn power_sketch(options: &PowerOptions, banner: bool) -> Element<'static, Message> {
    let icons: Vec<Element<'static, Message>> = options
        .actions
        .iter()
        .map(|&action| {
            icon::from_svg_bytes(power_icon(action))
                .symbolic(true)
                .icon()
                .size(12)
                .into()
        })
        .collect();
    let content: Element<'static, Message> = if options.vertical {
        cosmic::widget::column::with_children(icons)
            .spacing(6)
            .align_x(Alignment::Center)
            .into()
    } else {
        cosmic::widget::row::with_children(icons)
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
    };
    let content = container(content).center(Length::Fill);
    if banner {
        content
            .class(cosmic::theme::Container::custom(|theme: &Theme| container::Style {
                icon_color: Some(theme.cosmic().on_accent_color().into()),
                ..Default::default()
            }))
            .into()
    } else {
        content.into()
    }
}

/// App rows or a grid, led by tinted favorite and recently used apps if
/// the list shows them.
fn apps_sketch(options: &AppListOptions) -> Element<'static, Message> {
    let spacing = cosmic::theme::active().cosmic().spacing;
    let leading = options.favorites_shown().min(2) + options.recents_shown().min(2);
    let heading = options
        .show_heading
        .then(|| container(text_line(Length::Fixed(60.0), 5.0)).padding(space_xxs()));

    let apps: Element<'static, Message> = match options.view {
        AppView::Column => {
            let rows = APP_NAME_WIDTHS.iter().enumerate().map(|(index, &width)| {
                let description = options
                    .show_description
                    .then(|| text_line(Length::Fixed(width * 1.4), 3.0));
                let row = cosmic::iced::widget::row![
                    app_tile(index),
                    cosmic::iced::widget::column![text_line(Length::Fixed(width), 5.0)]
                        .push_maybe(description)
                        .spacing(3),
                ]
                .spacing(spacing.space_xxs)
                .align_y(Alignment::Center)
                .height(Length::Fixed(22.0))
                .width(Length::Fill);
                let tinted = index < leading;
                container(row)
                    .padding([0, spacing.space_xxs])
                    .class(cosmic::theme::Container::custom(move |theme| tint_style(theme, tinted)))
                    .into()
            });
            cosmic::widget::column::with_children(rows.collect::<Vec<_>>())
                .padding([spacing.space_xxs, spacing.space_xxs])
                .into()
        }
        AppView::Grid => {
            let columns = 5;
            let rows = (0..6).map(|row| -> Element<'static, Message> {
                cosmic::widget::row::with_children((0..columns).map(|column| {
                    let index = row * columns + column;
                    let tinted = index < leading;
                    container(
                        cosmic::iced::widget::column![
                            app_tile(index),
                            text_line(Length::Fixed(24.0), 3.0),
                        ]
                        .spacing(3)
                        .align_x(Alignment::Center),
                    )
                    .padding(2)
                    .center_x(Length::FillPortion(1))
                    .class(cosmic::theme::Container::custom(move |theme| tint_style(theme, tinted)))
                    .into()
                }))
                .into()
            });
            cosmic::widget::column::with_children(rows.collect::<Vec<_>>())
                .spacing(6)
                .padding(space_xxs())
                .into()
        }
    };

    let mut content = cosmic::iced::widget::column![].push_maybe(heading).push(apps);
    if options.has_start_page() {
        // The button leading to all apps.
        content = content.push(Space::new().height(Length::Fill)).push(
            container(text_line(Length::Fixed(60.0), 5.0))
                .padding(space_xxs())
                .width(Length::Fill),
        );
    }
    content.height(Length::Fill).into()
}

fn side_panel_sketch(options: &SidePanelOptions) -> Element<'static, Message> {
    let spacing = cosmic::theme::active().cosmic().spacing;
    let entry = |icon: icon::Icon, selected: bool| -> Element<'static, Message> {
        container(
            cosmic::iced::widget::row![icon.size(10), text_line(Length::Fill, 5.0)]
                .spacing(spacing.space_xxs)
                .align_y(Alignment::Center),
        )
        .center_y(Length::Fixed(18.0))
        .width(Length::Fill)
        .padding([0, spacing.space_xxs])
        .class(cosmic::theme::Container::custom(move |theme| category_style(theme, selected)))
        .into()
    };
    let separator = || -> Element<'static, Message> {
        container(cosmic::widget::divider::horizontal::light())
            .center_y(Length::Fixed(8.0))
            .into()
    };

    let rows: Vec<Element<'static, Message>> = match options.content {
        SidePanelContent::Categories => {
            let mut rows = vec![entry(category_icon(&ApplicationCategory::ALL.icon), true)];
            if options.show_recently_used {
                rows.push(entry(category_icon(&ApplicationCategory::RECENTLY_USED.icon), false));
            }
            if options.show_favorites {
                rows.push(entry(category_icon(&ApplicationCategory::FAVORITES.icon), false));
            }
            rows.push(separator());
            rows.extend(
                CATEGORIES
                    .iter()
                    .map(|category| entry(category_icon(&category.icon), false)),
            );
            rows
        }
        SidePanelContent::Places => options
            .places
            .iter()
            .map(|place| match place {
                Place::Separator => separator(),
                Place::Spacer => Space::new().height(Length::Fill).into(),
                _ => entry(icon::from_name(place.icon_name()).icon(), false),
            })
            .collect(),
    };

    let panel = cosmic::widget::column::with_children(rows)
        .spacing(2)
        .padding(spacing.space_xxs);
    if options.tinted {
        container(panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .class(cosmic::theme::Container::custom(tinted_style))
            .into()
    } else {
        panel.into()
    }
}

fn featured_sketch(options: &FeaturedOptions) -> Element<'static, Message> {
    let count = options.count();
    if options.vertical {
        return cosmic::widget::column::with_children(
            (0..count.min(6)).map(app_tile).collect::<Vec<_>>(),
        )
        .spacing(6)
        .padding([space_xxs(), 2])
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .into();
    }

    let item = |index: usize| -> Element<'static, Message> {
        cosmic::iced::widget::row![app_tile(index), text_line(Length::Fill, 4.0)]
            .spacing(space_xxs())
            .align_y(Alignment::Center)
            .width(Length::FillPortion(1))
            .into()
    };
    let offset = match options.source {
        FeaturedApplications::RecentlyUsed => 5,
        FeaturedApplications::Favorites => 1,
    };
    let rows = (0..count.min(12)).collect::<Vec<_>>();
    let rows = rows.chunks(4).map(|chunk| -> Element<'static, Message> {
        let mut items: Vec<Element<'static, Message>> =
            chunk.iter().map(|index| item(index + offset)).collect();
        while items.len() < 4 {
            items.push(Space::new().width(Length::FillPortion(1)).into());
        }
        cosmic::widget::row::with_children(items)
            .spacing(space_xxs())
            .into()
    });

    let heading = options
        .show_heading
        .then(|| text_line(Length::Fixed(50.0), 5.0));
    cosmic::iced::widget::column![]
        .push_maybe(heading)
        .extend(rows)
        .spacing(4)
        .padding(space_xxs())
        .into()
}

/// Placeholder for a line of text.
fn text_line(width: Length, height: f32) -> Element<'static, Message> {
    container(Space::new())
        .width(width)
        .height(Length::Fixed(height))
        .class(cosmic::theme::Container::custom(text_line_style))
        .into()
}

/// Placeholder for an application icon, tinted so rows are told apart.
fn app_tile(index: usize) -> Element<'static, Message> {
    container(Space::new())
        .width(Length::Fixed(14.0))
        .height(Length::Fixed(14.0))
        .class(cosmic::theme::Container::custom(move |theme: &Theme| {
            let palette = &theme.cosmic().palette;
            let colors = [
                palette.accent_blue,
                palette.accent_orange,
                palette.accent_green,
                palette.accent_purple,
                palette.accent_red,
                palette.accent_yellow,
                palette.accent_indigo,
                palette.accent_pink,
            ];
            container::Style {
                background: Some(Background::Color(colors[index % colors.len()].into())),
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }))
        .into()
}

fn vertical_line() -> Element<'static, Message> {
    container(cosmic::widget::divider::vertical::light())
        .height(Length::Fill)
        .into()
}

fn space_xxs() -> u16 {
    cosmic::theme::active().cosmic().space_xxs()
}

/// Cards to switch between layouts, each with a miniature of its shape, and
/// the layout made by the user.
fn layout_picker(config: &AppletConfig) -> Element<'static, Message> {
    let selected = MenuLayout::matching(&config.layout);
    let card = |thumbnail_of: &Layout,
                name: String,
                description: String,
                is_selected: bool,
                message: Option<Message>|
     -> Element<'static, Message> {
        // Long translations get cut off on the card, so show them in full on hover.
        let full_text = format!("{name} · {description}");
        let card = cosmic::widget::button::custom(
            cosmic::iced::widget::column![
                thumbnail(thumbnail_of),
                // The image button colors its text with the accent, which
                // can hardly be read on the card; the usual text color can.
                text::body(name)
                    .wrapping(Wrapping::None)
                    .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)))
                    .class(cosmic::theme::Text::Custom(card_name_style)),
                // Up to two lines, so cards of short and long descriptions
                // stay the same height.
                text::caption(description)
                    .align_x(Alignment::Center)
                    .wrapping(Wrapping::WordOrGlyph)
                    .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(2)))
                    .height(Length::Fixed(CARD_DESCRIPTION_HEIGHT))
                    .class(cosmic::theme::Text::Custom(card_description_style)),
            ]
            .spacing(space_xxs())
            .align_x(Alignment::Center)
            .width(Length::Fill),
        )
        .class(cosmic::theme::Button::Image)
        .selected(is_selected)
        .padding(space_xxs())
        .width(Length::FillPortion(1))
        .on_press_maybe(message);

        tooltip(card, text::caption(full_text), tooltip::Position::Bottom).into()
    };

    let mut cards: Vec<Element<'static, Message>> = MenuLayout::ALL
        .into_iter()
        .map(|preset| {
            let (name, description) = layout_name(preset);
            card(
                &preset.layout(),
                name,
                description,
                selected == Some(preset),
                Some(Message::PresetSelected(preset)),
            )
        })
        .collect();

    // The current layout, if it is none of the presets, or the last one
    // made by the user.
    let custom = if selected.is_none() {
        Some(&config.layout)
    } else {
        config.custom_layout.as_ref()
    };
    cards.push(match custom {
        Some(layout) => card(
            layout,
            fl!("layout-custom"),
            fl!("layout-custom-description"),
            selected.is_none(),
            Some(Message::CustomSelected),
        ),
        None => card(
            &config.layout,
            fl!("layout-custom"),
            fl!("layout-custom-none"),
            false,
            None,
        ),
    });

    // In rows, as all of them side by side would be too narrow for their
    // names. The last row keeps the width of the others.
    let spacing = cosmic::theme::active().cosmic().space_xxs();
    let mut rows = Vec::new();
    let mut cards = cards.into_iter().peekable();
    while cards.peek().is_some() {
        let mut row: Vec<Element<'static, Message>> = cards.by_ref().take(LAYOUT_CARDS_PER_ROW).collect();
        while row.len() < LAYOUT_CARDS_PER_ROW {
            row.push(Space::new().width(Length::FillPortion(1)).into());
        }
        rows.push(
            cosmic::widget::row::with_children(row)
                .spacing(spacing)
                .width(Length::Fill)
                .into(),
        );
    }
    cosmic::widget::column::with_children(rows)
        .spacing(spacing)
        .width(Length::Fill)
        .into()
}

/// Text color of a layout's name on its card.
fn card_name_style(theme: &Theme) -> cosmic::iced::widget::text::Style {
    cosmic::iced::widget::text::Style {
        color: Some(theme.current_container().on.into()),
        ..Default::default()
    }
}

/// Text color of a layout's description on its card, less prominent than
/// its name.
fn card_description_style(theme: &Theme) -> cosmic::iced::widget::text::Style {
    let mut color: Color = theme.current_container().on.into();
    color.a *= 0.75;
    cosmic::iced::widget::text::Style {
        color: Some(color),
        ..Default::default()
    }
}

fn layout_name(layout: MenuLayout) -> (String, String) {
    match layout {
        MenuLayout::Classic => (fl!("layout-classic"), fl!("layout-classic-description")),
        MenuLayout::Retro => (fl!("layout-retro"), fl!("layout-retro-description")),
        MenuLayout::Compact => (fl!("layout-compact"), fl!("layout-compact-description")),
        MenuLayout::Cinnamon => (fl!("layout-sidebar"), fl!("layout-sidebar-description")),
        MenuLayout::Modern => (fl!("layout-modern"), fl!("layout-modern-description")),
        MenuLayout::Whistler => (fl!("layout-whistler"), fl!("layout-whistler-description")),
    }
}

/// Tiny wireframe of a layout's shape.
fn thumbnail(layout: &Layout) -> Element<'static, Message> {
    let rows = layout.rows.iter().map(|row| -> Element<'static, Message> {
        let tall = row.fills_height();
        if row.banner && !tall {
            return shape(Length::Fill, Length::Fixed(6.0), ShapeKind::Accent);
        }
        let columns = row.columns.iter().map(|column| -> Element<'static, Message> {
            let width = if column.fills_width() {
                Length::FillPortion(column.portion.max(1))
            } else {
                Length::Fixed(6.0)
            };
            let blocks = column.blocks.iter().filter_map(|block| -> Option<Element<'static, Message>> {
                Some(match block {
                    Block::AppList(_) => shape(Length::Fill, Length::Fill, ShapeKind::Pane),
                    Block::SidePanel(options) => shape(
                        Length::Fill,
                        Length::Fill,
                        if options.tinted { ShapeKind::Tinted } else { ShapeKind::Pane },
                    ),
                    Block::Featured(options) if options.vertical => {
                        shape(Length::Fill, Length::Fill, ShapeKind::Pane)
                    }
                    Block::Spacer | Block::Divider => return None,
                    _ => shape(Length::Fill, Length::Fixed(4.0), ShapeKind::Line),
                })
            });
            cosmic::widget::column::with_children(blocks.collect::<Vec<_>>())
                .spacing(2)
                .width(width)
                .height(if tall { Length::Fill } else { Length::Shrink })
                .into()
        });
        cosmic::widget::row::with_children(columns.collect::<Vec<_>>())
            .spacing(2)
            .width(Length::Fill)
            .height(if tall { Length::Fill } else { Length::Shrink })
            .into()
    });

    container(
        cosmic::widget::column::with_children(rows.collect::<Vec<_>>())
            .spacing(2)
            .height(Length::Fill),
    )
    .padding(3)
    .width(Length::Fixed(64.0))
    .height(Length::Fixed(52.0))
    .class(cosmic::theme::Container::custom(frame_style))
    .into()
}

#[derive(Clone, Copy)]
enum ShapeKind {
    Line,
    Pane,
    Tinted,
    Accent,
}

fn shape(width: Length, height: Length, kind: ShapeKind) -> Element<'static, Message> {
    container(Space::new())
        .width(width)
        .height(height)
        .class(cosmic::theme::Container::custom(move |theme: &Theme| {
            let cosmic = theme.cosmic();
            let color: Color = match kind {
                ShapeKind::Line => {
                    let mut color: Color = cosmic.on_bg_color().into();
                    color.a *= 0.3;
                    color
                }
                ShapeKind::Pane => cosmic.bg_component_color().into(),
                ShapeKind::Tinted => cosmic.primary_container_color().into(),
                ShapeKind::Accent => cosmic.accent_color().into(),
            };
            container::Style {
                background: Some(Background::Color(color)),
                border: Border {
                    radius: 1.5.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }))
        .into()
}

/// The name of the block following the cursor while dragging.
fn ghost(label: String, cursor: Point, preview_size: (f32, f32)) -> Element<'static, Message> {
    let (width, height) = preview_size;
    let x = (cursor.x - GHOST_WIDTH / 2.0).clamp(0.0, width - GHOST_WIDTH);
    let y = (cursor.y - GHOST_HEIGHT / 2.0).clamp(0.0, height - GHOST_HEIGHT);

    // Long names get cut off rather than spill out of the ghost.
    let label = text::body(label)
        .wrapping(Wrapping::None)
        .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)));
    let label = container(label)
        .padding([0, 8])
        .center(Length::Fill)
        .width(Length::Fixed(GHOST_WIDTH))
        .height(Length::Fixed(GHOST_HEIGHT))
        .class(cosmic::theme::Container::custom(ghost_style));

    container(label)
        .padding(Padding {
            top: y,
            left: x,
            ..Padding::ZERO
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn radius(theme: &Theme) -> cosmic::iced::border::Radius {
    theme.cosmic().radius_s().into()
}

fn accent_or_divider(theme: &Theme, accent: bool, radius: f32) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        background: Some(Background::Color(if accent {
            cosmic.accent_color().into()
        } else {
            cosmic.bg_divider().into()
        })),
        border: Border {
            radius: radius.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn frame_style(theme: &Theme) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        background: Some(Background::Color(cosmic.bg_color().into())),
        border: Border {
            color: cosmic.bg_divider().into(),
            width: 1.0,
            radius: cosmic.radius_m().into(),
        },
        ..Default::default()
    }
}

fn chip_style(theme: &Theme) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        text_color: Some(cosmic.on_bg_component_color().into()),
        icon_color: Some(cosmic.on_bg_component_color().into()),
        background: Some(Background::Color(cosmic.bg_component_color().into())),
        border: Border {
            color: cosmic.bg_component_divider().into(),
            width: 1.0,
            radius: cosmic.radius_xl().into(),
        },
        ..Default::default()
    }
}

fn text_line_style(theme: &Theme) -> container::Style {
    let mut color: Color = theme.cosmic().on_bg_component_color().into();
    color.a *= 0.3;
    container::Style {
        background: Some(Background::Color(color)),
        border: Border {
            radius: 2.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn search_style(theme: &Theme) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        icon_color: Some(cosmic.on_bg_component_color().into()),
        background: Some(Background::Color(cosmic.bg_color().into())),
        border: Border {
            color: cosmic.bg_component_divider().into(),
            width: 1.0,
            radius: cosmic.radius_xl().into(),
        },
        ..Default::default()
    }
}

/// Background of the favorite and recently used apps leading the list.
fn tint_style(theme: &Theme, tinted: bool) -> container::Style {
    if !tinted {
        return container::Style::default();
    }
    let mut accent: Color = theme.cosmic().accent_color().into();
    accent.a = 0.12;
    container::Style {
        background: Some(Background::Color(accent)),
        border: Border {
            radius: radius(theme),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn category_style(theme: &Theme, selected: bool) -> container::Style {
    let cosmic = theme.cosmic();
    if !selected {
        return container::Style::default();
    }
    container::Style {
        icon_color: Some(cosmic.on_accent_color().into()),
        background: Some(Background::Color(cosmic.accent_color().into())),
        border: Border {
            radius: radius(theme),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn banner_style(theme: &Theme) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        background: Some(Background::Color(cosmic.accent_color().into())),
        border: Border {
            radius: radius(theme),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn tinted_style(theme: &Theme) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        background: Some(Background::Color(cosmic.primary_container_color().into())),
        border: Border {
            radius: radius(theme),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn fixed_style(theme: &Theme) -> container::Style {
    let cosmic = theme.cosmic();
    let mut color: Color = cosmic.on_bg_color().into();
    color.a *= 0.4;
    container::Style {
        icon_color: Some(color),
        ..Default::default()
    }
}

fn slot_style(theme: &Theme, occupied: bool, selected: bool) -> container::Style {
    let cosmic = theme.cosmic();
    let background = occupied.then(|| Background::Color(cosmic.bg_component_color().into()));
    let border = if selected {
        Border {
            color: cosmic.accent_color().into(),
            width: 2.0,
            radius: radius(theme),
        }
    } else if occupied {
        Border {
            radius: radius(theme),
            ..Default::default()
        }
    } else {
        // Where the dragged block was.
        Border {
            color: cosmic.bg_divider().into(),
            width: 1.0,
            radius: radius(theme),
        }
    };

    container::Style {
        text_color: Some(cosmic.on_bg_component_color().into()),
        background,
        border,
        ..Default::default()
    }
}

fn ghost_style(theme: &Theme) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        text_color: Some(cosmic.on_accent_color().into()),
        background: Some(Background::Color(cosmic.accent_color().into())),
        border: Border {
            radius: radius(theme),
            ..Default::default()
        },
        shadow: cosmic::iced::Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
            offset: cosmic::iced::Vector::new(0.0, 2.0),
            blur_radius: 6.0,
        },
        ..Default::default()
    }
}

fn category_icon(category_icon: &CategoryIcon) -> icon::Icon {
    match category_icon {
        CategoryIcon::Bundled(bytes) => icon::from_svg_bytes(bytes.clone()).symbolic(true).icon(),
        CategoryIcon::Named(name) => icon::from_name(name.as_str()).icon(),
        CategoryIcon::Path(path) => icon::icon(icon::from_path(path.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic_ext_classic_menu_applet::layout::BlockKind;

    fn path(row: usize, column: usize, block: usize) -> BlockPath {
        BlockPath { row, column, block }
    }

    fn resized(handle: MenuHandle, moves: &[(f32, f32)], config: &AppletConfig) -> Option<Change> {
        let mut editor = LayoutEditor::default();
        editor.update(Message::MenuResizeStart(handle), config);
        for &(x, y) in moves {
            editor.update(Message::CursorMoved(Point::new(x, y)), config);
        }
        assert!(editor.is_dragging());
        let change = editor.update(Message::DragEnd, config);
        assert!(!editor.is_dragging());
        change
    }

    fn menu_size(change: Option<Change>) -> Option<(u16, u16)> {
        match change {
            Some(Change::MenuSize(width, height)) => Some((width, height)),
            _ => None,
        }
    }

    /// Drags `dragged` far enough onto `drop` and lets go.
    fn dropped(dragged: Dragged, drop: Drop, editor: &mut LayoutEditor, config: &AppletConfig) -> Option<Change> {
        editor.update(Message::DragStart(dragged), config);
        editor.update(Message::CursorMoved(Point::new(0.0, 0.0)), config);
        editor.update(Message::CursorMoved(Point::new(20.0, 0.0)), config);
        editor.update(Message::DragEnter(drop), config);
        editor.update(Message::DragEnd, config)
    }

    #[test]
    fn dropping_a_block_moves_it_and_selects_it() {
        let config = AppletConfig::default();
        let mut editor = LayoutEditor::default();
        let change = dropped(
            Dragged::Block(path(1, 0, 0)),
            Drop::NewRow(3),
            &mut editor,
            &config,
        );

        let Some(Change::Layout(layout)) = change else {
            panic!("no layout change: {change:?}");
        };
        assert_eq!(layout.rows[2].columns[0].blocks[0].kind(), BlockKind::Search);
        assert_eq!(editor.selected(&layout), Some(path(2, 0, 0)));
    }

    #[test]
    fn clicking_a_block_selects_it_without_moving_it() {
        let config = AppletConfig::default();
        let mut editor = LayoutEditor::default();
        editor.update(Message::DragStart(Dragged::Block(path(0, 0, 0))), &config);
        editor.update(Message::CursorMoved(Point::new(5.0, 5.0)), &config);
        editor.update(Message::DragEnter(Drop::NewRow(3)), &config);

        assert!(editor.update(Message::DragEnd, &config).is_none());
        assert_eq!(editor.selected(&config.layout), Some(path(0, 0, 0)));
    }

    #[test]
    fn new_blocks_come_from_the_palette() {
        let config = AppletConfig::default();
        let mut editor = LayoutEditor::default();
        let change = dropped(
            Dragged::New(BlockKind::Spacer),
            Drop::Beside {
                path: path(0, 0, 0),
                side: Side::After,
            },
            &mut editor,
            &config,
        );

        let Some(Change::Layout(layout)) = change else {
            panic!("no layout change: {change:?}");
        };
        assert_eq!(layout.rows[0].columns[1].blocks[0], Block::Spacer);
    }

    #[test]
    fn a_second_search_field_is_not_added() {
        let config = AppletConfig::default();
        let mut editor = LayoutEditor::default();
        let change = dropped(Dragged::New(BlockKind::Search), Drop::NewRow(0), &mut editor, &config);

        assert!(change.is_none());
    }

    #[test]
    fn resizing_columns_follows_the_cursor() {
        let config = AppletConfig::default();
        let mut editor = LayoutEditor::default();
        editor.update(Message::ResizeStart { row: 2, left: 0 }, &config);
        editor.update(Message::CursorMoved(Point::new(100.0, 0.0)), &config);
        // A tenth of the preview's width.
        editor.update(Message::CursorMoved(Point::new(130.0, 0.0)), &config);

        let Some(Change::Layout(layout)) = editor.update(Message::DragEnd, &config) else {
            panic!("no layout change");
        };
        let portions: Vec<u16> = layout.rows[2].columns.iter().map(|c| c.portion).collect();
        assert_eq!(portions, [72, 28]);
    }

    #[test]
    fn picking_the_custom_layout_brings_it_back() {
        let mut config = AppletConfig::default();
        let mut custom = MenuLayout::Classic.layout();
        custom.rows.reverse();
        config.set_menu_layout(custom.clone());
        config.set_menu_layout(MenuLayout::Retro.layout());

        let mut editor = LayoutEditor::default();
        assert!(matches!(
            editor.update(Message::CustomSelected, &config),
            Some(Change::Layout(layout)) if layout == custom
        ));
    }

    #[test]
    fn each_handle_resizes_its_own_axes() {
        let config = AppletConfig::default();
        let moves = [(100.0, 100.0), (150.0, 130.0)];

        // 50 preview pixels to the right change the centered preview's width
        // by 100, which is 200 pixels of the real menu.
        assert_eq!(menu_size(resized(MenuHandle::Right, &moves, &config)), Some((800, 700)));
        assert_eq!(menu_size(resized(MenuHandle::Bottom, &moves, &config)), Some((600, 760)));
        assert_eq!(
            menu_size(resized(MenuHandle::BottomRight, &moves, &config)),
            Some((800, 760))
        );
    }

    #[test]
    fn resizing_snaps_and_stops_at_the_bounds() {
        let config = AppletConfig::default();

        assert_eq!(
            menu_size(resized(MenuHandle::BottomRight, &[(0.0, 0.0), (3.2, 7.4)], &config)),
            Some((610, 710))
        );
        assert_eq!(
            menu_size(resized(MenuHandle::BottomRight, &[(0.0, 0.0), (9000.0, 9000.0)], &config)),
            Some((960, 1000))
        );
        assert_eq!(
            menu_size(resized(MenuHandle::BottomRight, &[(9000.0, 9000.0), (0.0, 0.0)], &config)),
            Some((500, 400))
        );
    }

    #[test]
    fn grabbing_a_handle_without_moving_changes_nothing() {
        let config = AppletConfig::default();

        assert!(resized(MenuHandle::Right, &[], &config).is_none());
        assert!(resized(MenuHandle::Right, &[(10.0, 10.0)], &config).is_none());
    }
}
