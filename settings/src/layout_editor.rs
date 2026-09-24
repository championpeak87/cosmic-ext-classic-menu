// SPDX-License-Identifier: GPL-3.0-only

//! Drag and drop editor for the applet menu layout.
//!
//! Renders a miniature of the menu popup. The search field can be dragged
//! to the top or bottom slot, the app list / categories panes can be
//! dragged onto each other to swap sides, and the power controls can be
//! dragged to the header, below either pane, or to the footer.
//!
//! Blocks are drawn as a sketch of the real menu (icons and placeholder
//! text lines) rather than an exact rendering. Each [`MenuLayout`] has its
//! own preview, and only offers the slots that layout supports.

use crate::fl;
use cosmic::iced::core::text::{Ellipsize, EllipsizeHeightLimit, Wrapping};
use cosmic::iced::{mouse, Alignment, Background, Border, Color, Length, Padding, Point};
use cosmic::widget::{container, icon, mouse_area, text, tooltip, Space};
use cosmic::{Element, Theme};
use cosmic_ext_classic_menu_applet::config::{
    AppletConfig, HorizontalPosition, MenuLayout, Place, PowerMenuPosition, SidePanel,
    VerticalPosition,
};
use cosmic_ext_classic_menu_applet::model::application_category::{ApplicationCategory, CategoryIcon};

const PREVIEW_WIDTH: f32 = 320.0;
const PREVIEW_HEIGHT: f32 = 360.0;
const BAR_HEIGHT: f32 = 26.0;
const POWER_HEADER_WIDTH: f32 = 96.0;
const GHOST_WIDTH: f32 = 110.0;
const GHOST_HEIGHT: f32 = 28.0;

/// Same proportions as the panes in the applet popup.
/// Pane proportions in the layout thumbnails.
const APP_LIST_PORTION: u16 = 5;
const CATEGORIES_PORTION: u16 = 3;
const RESIZE_HANDLE_WIDTH: f32 = 9.0;
/// Width of the pinned apps sidebar sketch.
const SIDEBAR_WIDTH: f32 = 22.0;

const POWER_ICONS: [&[u8]; 5] = [
    include_bytes!("../../res/icons/bundled/system-log-out-symbolic.svg"),
    include_bytes!("../../res/icons/bundled/system-suspend-symbolic.svg"),
    include_bytes!("../../res/icons/bundled/system-lock-screen-symbolic.svg"),
    include_bytes!("../../res/icons/bundled/system-reboot-symbolic.svg"),
    include_bytes!("../../res/icons/bundled/system-shutdown-symbolic.svg"),
];
const USER_ICON: &[u8] = include_bytes!("../../res/icons/bundled/user-idle-symbolic.svg");

/// Placeholder text widths for the sketched app list rows.
const APP_NAME_WIDTHS: [f32; 9] = [70.0, 54.0, 82.0, 46.0, 64.0, 76.0, 50.0, 68.0, 58.0];
/// Categories shown in the sketch, in the same order as the applet.
const CATEGORIES: [ApplicationCategory; 10] = [
    ApplicationCategory::ALL,
    ApplicationCategory::RECENTLY_USED,
    ApplicationCategory::AUDIO,
    ApplicationCategory::DEVELOPMENT,
    ApplicationCategory::GAMES,
    ApplicationCategory::GRAPHICS,
    ApplicationCategory::NETWORK,
    ApplicationCategory::OFFICE,
    ApplicationCategory::SYSTEM,
    ApplicationCategory::UTILITY,
];

/// A block of the menu that can be picked up.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Block {
    SearchField,
    AppList,
    Categories,
    /// Shown instead of [`Block::Categories`] when the side panel lists places.
    Places,
    PowerMenu,
}

/// A place in the preview where a block can be dropped.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Slot {
    SearchTop,
    SearchBottom,
    PaneLeft,
    PaneRight,
    Power(PowerMenuPosition),
}

impl Block {
    fn label(self) -> String {
        match self {
            Block::SearchField => fl!("layout-search-field"),
            Block::AppList => fl!("layout-app-list"),
            Block::Categories => fl!("layout-categories"),
            Block::Places => fl!("layout-places"),
            Block::PowerMenu => fl!("layout-power"),
        }
    }

    fn accepts(self, slot: Slot) -> bool {
        match self {
            Block::SearchField => matches!(slot, Slot::SearchTop | Slot::SearchBottom),
            Block::AppList | Block::Categories | Block::Places => {
                matches!(slot, Slot::PaneLeft | Slot::PaneRight)
            }
            Block::PowerMenu => matches!(slot, Slot::Power(_)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    LayoutSelected(MenuLayout),
    DragStart(Block),
    DragMove(Point),
    DragEnter(Slot),
    DragLeave(Slot),
    /// The divider between the panes was grabbed.
    ResizeStart,
    /// The cursor moved to this x position over the panes while resizing.
    ResizeMove(f32),
    /// Left mouse button released anywhere in the window.
    DragEnd,
}

/// Layout change resulting from picking a layout or completing a drop.
#[derive(Debug, Clone, Copy)]
pub enum Change {
    MenuLayout(MenuLayout),
    AppMenuPosition(HorizontalPosition),
    SearchFieldPosition(VerticalPosition),
    PowerMenuPosition(PowerMenuPosition),
    SidePanelWidth(u16),
}

#[derive(Debug, Default)]
pub struct LayoutEditor {
    dragging: Option<Block>,
    hovered: Option<Slot>,
    cursor: Option<Point>,
    /// Side panel width while the divider between the panes is dragged.
    resizing: Option<u16>,
}

impl LayoutEditor {
    pub fn is_dragging(&self) -> bool {
        self.dragging.is_some() || self.resizing.is_some()
    }

    pub fn update(&mut self, message: Message, config: &AppletConfig) -> Option<Change> {
        match message {
            Message::LayoutSelected(layout) => {
                (layout != config.menu_layout).then_some(Change::MenuLayout(layout))
            }
            Message::DragStart(block) => {
                self.dragging = Some(block);
                self.cursor = None;
                None
            }
            Message::DragMove(point) => {
                if self.dragging.is_some() {
                    self.cursor = Some(point);
                }
                None
            }
            Message::DragEnter(slot) => {
                self.hovered = Some(slot);
                None
            }
            Message::DragLeave(slot) => {
                // Enter/leave of neighbouring slots may arrive in either order.
                if self.hovered == Some(slot) {
                    self.hovered = None;
                }
                None
            }
            Message::ResizeStart => {
                self.resizing = Some(config.pane_portions().1);
                None
            }
            Message::ResizeMove(x) => {
                if self.resizing.is_some() {
                    let left_share = x / panes_width(config.menu_layout);
                    let side_panel_share = match config.app_menu_position {
                        HorizontalPosition::Left => 1.0 - left_share,
                        HorizontalPosition::Right => left_share,
                    };
                    let range = AppletConfig::SIDE_PANEL_WIDTH_RANGE;
                    self.resizing = Some(
                        ((side_panel_share * 100.0).round().max(0.0) as u16)
                            .clamp(*range.start(), *range.end()),
                    );
                }
                None
            }
            Message::DragEnd => {
                if let Some(width) = self.resizing.take() {
                    return (width != config.side_panel_width)
                        .then_some(Change::SidePanelWidth(width));
                }

                let block = self.dragging.take()?;
                self.cursor = None;
                let slot = self.hovered.filter(|slot| block.accepts(*slot))?;

                match (block, slot) {
                    (_, Slot::Power(position)) => {
                        (Some(position) != config.effective_power_menu_position())
                            .then_some(Change::PowerMenuPosition(position))
                    }
                    (Block::SearchField, slot) => {
                        let position = if slot == Slot::SearchTop {
                            VerticalPosition::Top
                        } else {
                            VerticalPosition::Bottom
                        };
                        (position != config.search_field_position)
                            .then_some(Change::SearchFieldPosition(position))
                    }
                    (block, slot) => {
                        // `HorizontalPosition` describes where the app list sits.
                        let app_list_left = (block == Block::AppList) == (slot == Slot::PaneLeft);
                        let position = if app_list_left {
                            HorizontalPosition::Left
                        } else {
                            HorizontalPosition::Right
                        };
                        (position != config.app_menu_position)
                            .then_some(Change::AppMenuPosition(position))
                    }
                }
            }
        }
    }

    pub fn view(&self, config: &AppletConfig) -> Element<'_, Message> {
        let spacing = cosmic::theme::active().cosmic().spacing;

        let menu = match config.menu_layout {
            MenuLayout::Classic => self.view_classic(config),
            MenuLayout::Retro => self.view_retro(config),
            MenuLayout::Compact => self.view_compact(config),
            MenuLayout::Cinnamon => self.view_cinnamon(config),
            MenuLayout::Modern => self.view_modern(config),
        };

        let frame = container(menu)
            .padding(spacing.space_xxs)
            .width(Length::Fixed(PREVIEW_WIDTH))
            .height(Length::Fixed(PREVIEW_HEIGHT))
            .class(cosmic::theme::Container::custom(frame_style));

        let mut preview = cosmic::iced::widget::stack![frame];
        if let (Some(block), Some(cursor)) = (self.dragging, self.cursor) {
            preview = preview.push(ghost(block, cursor));
        }

        let preview = mouse_area(preview)
            .on_move(Message::DragMove)
            .interaction(if self.is_dragging() {
                mouse::Interaction::Grabbing
            } else {
                mouse::Interaction::None
            });

        cosmic::iced::widget::column![
            layout_picker(config.menu_layout),
            preview,
            text::caption(fl!("layout-hint")),
        ]
        .spacing(spacing.space_s)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .into()
    }

    fn view_classic(&self, config: &AppletConfig) -> Element<'_, Message> {
        let header = cosmic::iced::widget::row![user_bar(Length::Fill)]
            .push_maybe(self.power_slot(
                PowerMenuPosition::Header,
                config,
                Length::Fixed(POWER_HEADER_WIDTH),
            ))
            .spacing(space_xxs());

        self.with_search(
            config,
            header.into(),
            self.panes(config, false),
            self.power_slot(PowerMenuPosition::Footer, config, Length::Fill),
        )
    }

    fn view_retro(&self, config: &AppletConfig) -> Element<'_, Message> {
        let header = container(
            cosmic::iced::widget::row![user_bar(Length::Fill)]
                .push_maybe(self.power_slot(
                    PowerMenuPosition::Header,
                    config,
                    Length::Fixed(POWER_HEADER_WIDTH),
                ))
                .spacing(space_xxs()),
        )
        .padding(2)
        .class(cosmic::theme::Container::custom(banner_style));

        let footer = container(
            cosmic::iced::widget::row![Space::new().width(Length::Fill)]
                .push_maybe(self.power_slot(
                    PowerMenuPosition::Footer,
                    config,
                    Length::Fixed(POWER_HEADER_WIDTH),
                ))
                .height(Length::Fixed(BAR_HEIGHT)),
        )
        .padding(2)
        .class(cosmic::theme::Container::custom(banner_style));

        self.with_search(
            config,
            header.into(),
            self.panes(config, true),
            Some(footer.into()),
        )
    }

    fn view_compact(&self, config: &AppletConfig) -> Element<'_, Message> {
        let header = cosmic::iced::widget::row![user_bar(Length::Fixed(72.0))]
            .push(fill_or_space(self.search_slot(Slot::SearchTop, config)))
            .push_maybe(self.power_slot(
                PowerMenuPosition::Header,
                config,
                Length::Fixed(POWER_HEADER_WIDTH),
            ))
            .spacing(space_xxs());

        let footer = cosmic::iced::widget::row![]
            .push(fill_or_space(self.search_slot(Slot::SearchBottom, config)))
            .push_maybe(self.power_slot(
                PowerMenuPosition::Footer,
                config,
                Length::Fixed(POWER_HEADER_WIDTH),
            ))
            .spacing(space_xxs());

        cosmic::iced::widget::column![header, self.panes(config, false), footer]
            .spacing(space_xxs())
            .into()
    }

    fn view_cinnamon(&self, config: &AppletConfig) -> Element<'_, Message> {
        let body = cosmic::iced::widget::row![
            fixed_part(
                container(sidebar_sketch()).width(Length::Fixed(SIDEBAR_WIDTH)).into(),
                fl!("layout-pinned")
            ),
            cosmic::widget::divider::vertical::default(),
            self.panes(config, false),
        ]
        .spacing(space_xxs());

        self.with_search(config, user_bar(Length::Fill), body.into(), None)
    }

    fn view_modern(&self, config: &AppletConfig) -> Element<'_, Message> {
        let header = self
            .power_slot(PowerMenuPosition::Header, config, Length::Fixed(POWER_HEADER_WIDTH))
            .map(|power| cosmic::iced::widget::row![Space::new().width(Length::Fill), power]);

        let footer = cosmic::iced::widget::row![user_bar(Length::Fill)]
            .push_maybe(self.power_slot(
                PowerMenuPosition::Footer,
                config,
                Length::Fixed(POWER_HEADER_WIDTH),
            ))
            .spacing(space_xxs());

        cosmic::iced::widget::column![]
            .push_maybe(header)
            .push_maybe(self.search_slot(Slot::SearchTop, config))
            .push(fixed_part(app_grid_sketch(), fl!("layout-app-list")))
            .push(fixed_part(recent_sketch(), fl!("layout-recent")))
            .push_maybe(self.search_slot(Slot::SearchBottom, config))
            .push(cosmic::widget::divider::horizontal::light())
            .push(footer)
            .spacing(space_xxs())
            .into()
    }

    /// Stacks `header`, `body` and `footer`, with search slots above and
    /// below the body.
    fn with_search<'a>(
        &'a self,
        config: &AppletConfig,
        header: Element<'a, Message>,
        body: Element<'a, Message>,
        footer: Option<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        cosmic::iced::widget::column![header]
            .push_maybe(self.search_slot(Slot::SearchTop, config))
            .push(container(body).height(Length::Fill))
            .push_maybe(self.search_slot(Slot::SearchBottom, config))
            .push_maybe(footer)
            .spacing(space_xxs())
            .into()
    }

    fn search_slot(&self, slot: Slot, config: &AppletConfig) -> Option<Element<'_, Message>> {
        let position = if slot == Slot::SearchTop {
            VerticalPosition::Top
        } else {
            VerticalPosition::Bottom
        };
        self.slot(
            slot,
            (config.search_field_position == position).then_some(Block::SearchField),
            Length::Fill,
            Length::Fixed(BAR_HEIGHT),
        )
    }

    /// App list and categories side by side, each with the power controls
    /// slot below it where the layout supports one.
    fn panes(&self, config: &AppletConfig, tinted: bool) -> Element<'_, Message> {
        let side_panel = match config.side_panel {
            SidePanel::Categories => Block::Categories,
            SidePanel::Places => Block::Places,
        };
        let (left, right) = match config.app_menu_position {
            HorizontalPosition::Left => (Block::AppList, side_panel),
            HorizontalPosition::Right => (side_panel, Block::AppList),
        };

        let panes = cosmic::iced::widget::row![
            self.pane(Slot::PaneLeft, left, config, tinted),
            self.resize_handle(),
            self.pane(Slot::PaneRight, right, config, tinted),
        ]
        .spacing(space_xxs())
        // Fixed, so cursor positions translate into a split.
        .width(Length::Fixed(panes_width(config.menu_layout)))
        .height(Length::Fill);

        let mut area = mouse_area(panes);
        if self.resizing.is_some() {
            area = area.on_move(|point| Message::ResizeMove(point.x));
        }
        area.into()
    }

    /// The divider between the panes, dragged to change their widths.
    fn resize_handle(&self) -> Element<'_, Message> {
        let resizing = self.resizing.is_some();
        let line = container(Space::new())
            .width(Length::Fixed(if resizing { 2.0 } else { 1.0 }))
            .height(Length::Fill)
            .class(cosmic::theme::Container::custom(move |theme: &Theme| {
                let cosmic = theme.cosmic();
                container::Style {
                    background: Some(Background::Color(if resizing {
                        cosmic.accent_color().into()
                    } else {
                        cosmic.bg_divider().into()
                    })),
                    ..Default::default()
                }
            }));

        // Wider than the line, so it is easy to grab.
        let handle = mouse_area(
            container(line)
                .center_x(Length::Fixed(RESIZE_HANDLE_WIDTH))
                .height(Length::Fill),
        )
        .on_press(Message::ResizeStart)
        .interaction(mouse::Interaction::ResizingHorizontally);

        if self.is_dragging() {
            handle.into()
        } else {
            tooltip(
                handle,
                text::caption(fl!("layout-resize-panes")),
                tooltip::Position::Bottom,
            )
            .into()
        }
    }

    /// A pane together with the power controls slot below it.
    fn pane(
        &self,
        slot: Slot,
        block: Block,
        config: &AppletConfig,
        tinted: bool,
    ) -> Element<'_, Message> {
        let (app_list_portion, side_panel_portion) = match self.resizing {
            Some(width) => (100 - width, width),
            None => config.pane_portions(),
        };
        let (power, portion) = if block == Block::AppList {
            (PowerMenuPosition::AppList, app_list_portion)
        } else {
            (PowerMenuPosition::Categories, side_panel_portion)
        };

        let pane = cosmic::iced::widget::column![]
            .push_maybe(self.slot(slot, Some(block), Length::Fill, Length::Fill))
            .push_maybe(self.power_slot(power, config, Length::Fill))
            .spacing(space_xxs())
            .width(Length::FillPortion(portion))
            .height(Length::Fill);

        if tinted && block != Block::AppList {
            container(pane)
                .padding(2)
                .width(Length::FillPortion(portion))
                .height(Length::Fill)
                .class(cosmic::theme::Container::custom(tinted_style))
                .into()
        } else {
            pane.into()
        }
    }

    /// Power controls slot, if the layout supports this position.
    fn power_slot(
        &self,
        position: PowerMenuPosition,
        config: &AppletConfig,
        width: Length,
    ) -> Option<Element<'_, Message>> {
        if !config.menu_layout.power_menu_positions().contains(&position) {
            return None;
        }

        self.slot(
            Slot::Power(position),
            (config.effective_power_menu_position() == Some(position))
                .then_some(Block::PowerMenu),
            width,
            Length::Fixed(BAR_HEIGHT),
        )
    }

    /// A drop slot, optionally holding a block.
    ///
    /// Empty slots are only shown while dragging a block that fits them, so
    /// the preview otherwise looks like the real menu.
    fn slot(
        &self,
        slot: Slot,
        block: Option<Block>,
        width: Length,
        height: Length,
    ) -> Option<Element<'_, Message>> {
        let picked_up = block.is_some() && block == self.dragging;
        let drop_target = self.dragging.is_some_and(|dragging| dragging.accepts(slot));
        let highlighted = drop_target && self.hovered == Some(slot);
        if block.is_none() && !drop_target {
            return None;
        }

        let content: Element<'_, Message> = match block {
            Some(block) if !picked_up => sketch(block),
            _ => Space::new().into(),
        };

        let content = container(content)
            .width(width)
            .height(height)
            .clip(true)
            .class(cosmic::theme::Container::custom(move |theme| {
                slot_style(theme, block.is_some() && !picked_up, drop_target, highlighted)
            }));

        // Name the block on hover, as the sketch alone may be ambiguous.
        let content: Element<'_, Message> = match block {
            Some(block) if !self.is_dragging() => tooltip(
                content,
                text::caption(block.label()),
                tooltip::Position::Bottom,
            )
            .into(),
            _ => content.into(),
        };

        let mut area = mouse_area(content)
            .on_enter(Message::DragEnter(slot))
            .on_exit(Message::DragLeave(slot));
        if let Some(block) = block {
            area = area.on_press(Message::DragStart(block));
            if !self.is_dragging() {
                area = area.interaction(mouse::Interaction::Grab);
            }
        }

        Some(area.into())
    }
}

/// A rough drawing of what the block looks like in the applet popup.
fn sketch(block: Block) -> Element<'static, Message> {
    let spacing = cosmic::theme::active().cosmic().spacing;

    match block {
        Block::SearchField => container(
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

        Block::AppList => {
            let rows = APP_NAME_WIDTHS.iter().enumerate().map(|(index, &width)| {
                cosmic::iced::widget::row![
                    app_tile(index),
                    cosmic::iced::widget::column![
                        text_line(Length::Fixed(width), 5.0),
                        text_line(Length::Fixed(width * 1.4), 3.0),
                    ]
                    .spacing(3),
                ]
                .spacing(spacing.space_xxs)
                .align_y(Alignment::Center)
                .height(Length::Fixed(22.0))
                .into()
            });
            let rows: Vec<Element<'static, Message>> = rows.collect();

            cosmic::widget::column::with_children(rows)
                .padding([spacing.space_xxs, spacing.space_xs])
                .into()
        }

        Block::Categories => {
            let mut rows: Vec<Element<'static, Message>> = CATEGORIES
                .iter()
                .enumerate()
                .map(|(index, category)| {
                    let selected = index == 0;
                    container(
                        cosmic::iced::widget::row![
                            category_icon(&category.icon).size(10),
                            text_line(Length::Fill, 5.0),
                        ]
                        .spacing(spacing.space_xxs)
                        .align_y(Alignment::Center),
                    )
                    .center_y(Length::Fixed(18.0))
                    .width(Length::Fill)
                    .padding([0, spacing.space_xxs])
                    .class(cosmic::theme::Container::custom(move |theme| {
                        category_style(theme, selected)
                    }))
                    .into()
                })
                .collect();
            // The applet separates the permanent categories from the rest.
            rows.insert(2, cosmic::widget::divider::horizontal::light().into());

            cosmic::widget::column::with_children(rows)
                .spacing(2)
                .padding(spacing.space_xxs)
                .into()
        }

        Block::Places => {
            let rows = Place::DEFAULT.iter().map(|place| -> Element<'static, Message> {
                if *place == Place::Separator {
                    return container(cosmic::widget::divider::horizontal::light())
                        .center_y(Length::Fixed(8.0))
                        .into();
                }
                container(
                    cosmic::iced::widget::row![
                        icon::from_name(place.icon_name()).size(10),
                        text_line(Length::Fill, 5.0),
                    ]
                    .spacing(spacing.space_xxs)
                    .align_y(Alignment::Center),
                )
                .center_y(Length::Fixed(18.0))
                .width(Length::Fill)
                .padding([0, spacing.space_xxs])
                .into()
            });

            cosmic::widget::column::with_children(rows)
                .spacing(2)
                .padding(spacing.space_xxs)
                .into()
        }

        Block::PowerMenu => container(
            cosmic::widget::row::with_children(
                POWER_ICONS
                    .iter()
                    .map(|bytes| {
                        icon::from_svg_bytes(*bytes)
                            .symbolic(true)
                            .icon()
                            .size(12)
                            .into()
                    })
                    .collect::<Vec<Element<'static, Message>>>(),
            )
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .center(Length::Fill)
        .into(),
    }
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

/// Width of the row holding both panes and the divider between them.
fn panes_width(layout: MenuLayout) -> f32 {
    let spacing = f32::from(space_xxs());
    // Inside the frame's padding.
    let width = PREVIEW_WIDTH - 2.0 * spacing;
    if layout == MenuLayout::Cinnamon {
        // Next to the sidebar and its divider.
        width - SIDEBAR_WIDTH - 1.0 - 2.0 * spacing
    } else {
        width
    }
}

fn space_xxs() -> u16 {
    cosmic::theme::active().cosmic().space_xxs()
}

fn user_bar(width: Length) -> Element<'static, Message> {
    let user = container(
        cosmic::iced::widget::row![
            icon::from_svg_bytes(USER_ICON).symbolic(true).icon().size(16),
            text_line(Length::Fixed(40.0), 6.0),
        ]
        .spacing(space_xxs())
        .align_y(Alignment::Center),
    )
    .center_y(Length::Fixed(BAR_HEIGHT))
    .padding([0, space_xxs()])
    .width(width)
    .class(cosmic::theme::Container::custom(fixed_style));

    tooltip(
        user,
        text::caption(fl!("layout-user")),
        tooltip::Position::Bottom,
    )
    .into()
}

/// `slot` if shown, otherwise a spacer taking its room in a row.
fn fill_or_space(slot: Option<Element<'_, Message>>) -> Element<'_, Message> {
    slot.unwrap_or_else(|| Space::new().width(Length::Fill).into())
}

/// A part of the preview that the layout keeps in place.
fn fixed_part(content: Element<'static, Message>, label: String) -> Element<'static, Message> {
    tooltip(content, text::caption(label), tooltip::Position::Bottom).into()
}

/// Pinned apps above vertical power controls.
fn sidebar_sketch() -> Element<'static, Message> {
    let pinned = (0..4).map(|index| app_tile(index));
    let power = POWER_ICONS.iter().map(|bytes| {
        icon::from_svg_bytes(*bytes)
            .symbolic(true)
            .icon()
            .size(12)
            .into()
    });

    cosmic::widget::column::with_children(pinned)
        .push(Space::new().height(Length::Fill))
        .extend(power)
        .spacing(6)
        .padding([space_xxs(), 2])
        .align_x(Alignment::Center)
        .height(Length::Fill)
        .into()
}

/// A grid of app tiles with names below.
fn app_grid_sketch() -> Element<'static, Message> {
    let rows = (0..6).map(|row| -> Element<'static, Message> {
        cosmic::widget::row::with_children((0..5).map(|column| {
            cosmic::iced::widget::column![
                app_tile(row * 5 + column),
                text_line(Length::Fixed(28.0), 3.0),
            ]
            .spacing(3)
            .align_x(Alignment::Center)
            .width(Length::FillPortion(1))
            .into()
        }))
        .into()
    });

    cosmic::iced::widget::column![text_line(Length::Fixed(60.0), 5.0)]
        .extend(rows)
        .spacing(8)
        .padding(space_xxs())
        .height(Length::Fill)
        .into()
}

/// Recently used apps in two columns.
fn recent_sketch() -> Element<'static, Message> {
    let item = |index: usize| -> Element<'static, Message> {
        cosmic::iced::widget::row![app_tile(index), text_line(Length::Fixed(48.0), 4.0)]
            .spacing(space_xxs())
            .align_y(Alignment::Center)
            .width(Length::FillPortion(1))
            .into()
    };

    cosmic::iced::widget::column![
        text_line(Length::Fixed(60.0), 5.0),
        cosmic::iced::widget::row![item(5), item(6)],
        cosmic::iced::widget::row![item(7), item(0)],
    ]
    .spacing(6)
    .padding(space_xxs())
    .into()
}

/// Cards to switch between layouts, each with a miniature of its shape.
fn layout_picker(selected: MenuLayout) -> Element<'static, Message> {
    let cards = MenuLayout::ALL.into_iter().map(|layout| -> Element<'static, Message> {
        let (name, description) = layout_name(layout);
        // Long translations get cut off on the card, so show them in full on hover.
        let full_text = format!("{name} · {description}");
        let card = cosmic::widget::button::custom(
            cosmic::iced::widget::column![
                thumbnail(layout),
                text::body(name)
                    .wrapping(Wrapping::None)
                    .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1))),
                text::caption(description)
                    .wrapping(Wrapping::None)
                    .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1))),
            ]
            .spacing(space_xxs())
            .align_x(Alignment::Center)
            .width(Length::Fill),
        )
        .class(cosmic::theme::Button::Image)
        .selected(layout == selected)
        .padding(space_xxs())
        .width(Length::FillPortion(1))
        .on_press(Message::LayoutSelected(layout));

        tooltip(card, text::caption(full_text), tooltip::Position::Bottom).into()
    });

    cosmic::widget::row::with_children(cards)
        .spacing(cosmic::theme::active().cosmic().space_xs())
        .width(Length::Fill)
        .into()
}

fn layout_name(layout: MenuLayout) -> (String, String) {
    match layout {
        MenuLayout::Classic => (fl!("layout-classic"), fl!("layout-classic-description")),
        MenuLayout::Retro => (fl!("layout-retro"), fl!("layout-retro-description")),
        MenuLayout::Compact => (fl!("layout-compact"), fl!("layout-compact-description")),
        MenuLayout::Cinnamon => (fl!("layout-sidebar"), fl!("layout-sidebar-description")),
        MenuLayout::Modern => (fl!("layout-modern"), fl!("layout-modern-description")),
    }
}

/// Tiny wireframe of a layout's shape.
fn thumbnail(layout: MenuLayout) -> Element<'static, Message> {
    let bar = |height: f32| shape(Length::Fill, Length::Fixed(height), ShapeKind::Line);
    let pane = |portion: u16, kind: ShapeKind| {
        shape(Length::FillPortion(portion), Length::Fill, kind)
    };
    let panes = |categories_first: bool, tinted: bool| -> Element<'static, Message> {
        let categories = pane(
            CATEGORIES_PORTION,
            if tinted { ShapeKind::Tinted } else { ShapeKind::Pane },
        );
        let apps = pane(APP_LIST_PORTION, ShapeKind::Pane);
        let row = if categories_first {
            cosmic::iced::widget::row![categories, apps]
        } else {
            cosmic::iced::widget::row![apps, categories]
        };
        row.spacing(2).height(Length::Fill).into()
    };

    let content: Element<'static, Message> = match layout {
        MenuLayout::Classic => cosmic::iced::widget::column![bar(5.0), bar(5.0), panes(false, false)]
            .spacing(2)
            .into(),
        MenuLayout::Retro => cosmic::iced::widget::column![
            shape(Length::Fill, Length::Fixed(8.0), ShapeKind::Accent),
            panes(false, true),
            shape(Length::Fill, Length::Fixed(6.0), ShapeKind::Accent),
        ]
        .spacing(2)
        .into(),
        MenuLayout::Compact => cosmic::iced::widget::column![
            cosmic::iced::widget::row![
                shape(Length::Fixed(5.0), Length::Fixed(5.0), ShapeKind::Line),
                bar(5.0),
            ]
            .spacing(2),
            panes(true, false),
            cosmic::iced::widget::row![
                Space::new().width(Length::Fill),
                shape(Length::Fixed(20.0), Length::Fixed(4.0), ShapeKind::Line),
            ],
        ]
        .spacing(2)
        .into(),
        MenuLayout::Cinnamon => cosmic::iced::widget::column![
            bar(5.0),
            cosmic::iced::widget::row![
                shape(Length::Fixed(6.0), Length::Fill, ShapeKind::Pane),
                pane(CATEGORIES_PORTION, ShapeKind::Pane),
                pane(APP_LIST_PORTION, ShapeKind::Pane),
            ]
            .spacing(2)
            .height(Length::Fill),
        ]
        .spacing(2)
        .into(),
        MenuLayout::Modern => {
            let tiles = (0..3).map(|_| -> Element<'static, Message> {
                cosmic::widget::row::with_children((0..5).map(|_| {
                    shape(Length::Fixed(6.0), Length::Fixed(6.0), ShapeKind::Accent)
                }))
                .spacing(4)
                .into()
            });
            cosmic::iced::widget::column![bar(5.0)]
                .extend(tiles)
                .push(Space::new().height(Length::Fill))
                .push(bar(4.0))
                .spacing(3)
                .align_x(Alignment::Center)
                .into()
        }
    };

    container(content)
        .padding(3)
        .width(Length::Fixed(72.0))
        .height(Length::Fixed(56.0))
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

/// The block following the cursor while dragging.
fn ghost(block: Block, cursor: Point) -> Element<'static, Message> {
    let x = (cursor.x - GHOST_WIDTH / 2.0).clamp(0.0, PREVIEW_WIDTH - GHOST_WIDTH);
    let y = (cursor.y - GHOST_HEIGHT / 2.0).clamp(0.0, PREVIEW_HEIGHT - GHOST_HEIGHT);

    let label = container(text::body(block.label()))
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
    let mut text_color: Color = cosmic.on_bg_color().into();
    text_color.a *= 0.6;
    container::Style {
        text_color: Some(text_color),
        border: Border {
            radius: radius(theme),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn slot_style(
    theme: &Theme,
    occupied: bool,
    drop_target: bool,
    highlighted: bool,
) -> container::Style {
    let cosmic = theme.cosmic();
    let accent: Color = cosmic.accent_color().into();

    let background = if highlighted {
        Some(Background::Color(Color { a: 0.2, ..accent }))
    } else if occupied {
        Some(Background::Color(cosmic.bg_component_color().into()))
    } else {
        None
    };

    let border = if highlighted {
        Border {
            color: accent,
            width: 2.0,
            radius: radius(theme),
        }
    } else if occupied {
        Border {
            radius: radius(theme),
            ..Default::default()
        }
    } else if drop_target {
        // Outline empty slots the dragged block can be dropped on.
        Border {
            color: cosmic.bg_divider().into(),
            width: 1.0,
            radius: radius(theme),
        }
    } else {
        Border::default()
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
