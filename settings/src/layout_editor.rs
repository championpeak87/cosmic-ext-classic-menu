// SPDX-License-Identifier: GPL-3.0-only

//! Drag and drop editor for the applet menu layout.
//!
//! Renders a miniature of the menu popup. The search field can be dragged
//! to the top or bottom slot, the app list / categories panes can be
//! dragged onto each other to swap sides, and the power controls can be
//! dragged to the header, below either pane, or to the footer.
//!
//! Blocks are drawn as a sketch of the real menu (icons and placeholder
//! text lines) rather than an exact rendering.

use crate::fl;
use cosmic::iced::{mouse, Alignment, Background, Border, Color, Length, Padding, Point};
use cosmic::widget::{container, icon, mouse_area, text, tooltip, Space};
use cosmic::{Element, Theme};
use cosmic_ext_classic_menu_applet::config::{
    AppletConfig, HorizontalPosition, PowerMenuPosition, VerticalPosition,
};
use cosmic_ext_classic_menu_applet::model::application_category::ApplicationCategory;

const PREVIEW_WIDTH: f32 = 320.0;
const PREVIEW_HEIGHT: f32 = 360.0;
const BAR_HEIGHT: f32 = 26.0;
const POWER_HEADER_WIDTH: f32 = 96.0;
const GHOST_WIDTH: f32 = 110.0;
const GHOST_HEIGHT: f32 = 28.0;

/// Same proportions as the panes in the applet popup.
const APP_LIST_PORTION: u16 = 5;
const CATEGORIES_PORTION: u16 = 3;

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
            Block::PowerMenu => fl!("layout-power"),
        }
    }

    fn accepts(self, slot: Slot) -> bool {
        match self {
            Block::SearchField => matches!(slot, Slot::SearchTop | Slot::SearchBottom),
            Block::AppList | Block::Categories => {
                matches!(slot, Slot::PaneLeft | Slot::PaneRight)
            }
            Block::PowerMenu => matches!(slot, Slot::Power(_)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    DragStart(Block),
    DragMove(Point),
    DragEnter(Slot),
    DragLeave(Slot),
    /// Left mouse button released anywhere in the window.
    DragEnd,
}

/// Layout change resulting from a completed drop.
#[derive(Debug, Clone, Copy)]
pub enum Change {
    AppMenuPosition(HorizontalPosition),
    SearchFieldPosition(VerticalPosition),
    PowerMenuPosition(PowerMenuPosition),
}

#[derive(Debug, Default)]
pub struct LayoutEditor {
    dragging: Option<Block>,
    hovered: Option<Slot>,
    cursor: Option<Point>,
}

impl LayoutEditor {
    pub fn is_dragging(&self) -> bool {
        self.dragging.is_some()
    }

    pub fn update(&mut self, message: Message, config: &AppletConfig) -> Option<Change> {
        match message {
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
            Message::DragEnd => {
                let block = self.dragging.take()?;
                self.cursor = None;
                let slot = self.hovered.filter(|slot| block.accepts(*slot))?;

                match (block, slot) {
                    (_, Slot::Power(position)) => (position != config.power_menu_position)
                        .then_some(Change::PowerMenuPosition(position)),
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

        let (left, right) = match config.app_menu_position {
            HorizontalPosition::Left => (Block::AppList, Block::Categories),
            HorizontalPosition::Right => (Block::Categories, Block::AppList),
        };
        let panes = cosmic::iced::widget::row![
            self.pane(Slot::PaneLeft, left, config),
            cosmic::widget::divider::vertical::default(),
            self.pane(Slot::PaneRight, right, config),
        ]
        .spacing(spacing.space_xxs)
        .height(Length::Fill);

        let search_top = self.slot(
            Slot::SearchTop,
            (config.search_field_position == VerticalPosition::Top).then_some(Block::SearchField),
            Length::Fill,
            Length::Fixed(BAR_HEIGHT),
        );
        let search_bottom = self.slot(
            Slot::SearchBottom,
            (config.search_field_position == VerticalPosition::Bottom)
                .then_some(Block::SearchField),
            Length::Fill,
            Length::Fixed(BAR_HEIGHT),
        );

        let user = container(
            cosmic::iced::widget::row![
                icon::from_svg_bytes(USER_ICON).symbolic(true).icon().size(16),
                text_line(Length::Fixed(56.0), 6.0),
            ]
            .spacing(spacing.space_xxs)
            .align_y(Alignment::Center),
        )
        .center_y(Length::Fixed(BAR_HEIGHT))
        .padding([0, spacing.space_xxs])
        .width(Length::Fill)
        .class(cosmic::theme::Container::custom(fixed_style));
        let user = tooltip(
            user,
            text::caption(fl!("layout-user")),
            tooltip::Position::Bottom,
        );
        let header = cosmic::iced::widget::row![user]
            .push_maybe(self.power_slot(
                PowerMenuPosition::Header,
                config,
                Length::Fixed(POWER_HEADER_WIDTH),
            ))
            .spacing(spacing.space_xxs);

        let footer = self.power_slot(PowerMenuPosition::Footer, config, Length::Fill);

        let menu = cosmic::iced::widget::column![header]
            .push_maybe(search_top)
            .push(panes)
            .push_maybe(search_bottom)
            .push_maybe(footer)
            .spacing(spacing.space_xxs);

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
            preview,
            text::caption(fl!("layout-hint")),
        ]
        .spacing(spacing.space_xs)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .into()
    }

    /// A pane together with the power controls slot below it.
    fn pane(&self, slot: Slot, block: Block, config: &AppletConfig) -> Element<'_, Message> {
        let (power, portion) = if block == Block::AppList {
            (PowerMenuPosition::AppList, APP_LIST_PORTION)
        } else {
            (PowerMenuPosition::Categories, CATEGORIES_PORTION)
        };

        cosmic::iced::widget::column![]
            .push_maybe(self.slot(slot, Some(block), Length::Fill, Length::Fill))
            .push_maybe(self.power_slot(power, config, Length::Fill))
            .spacing(cosmic::theme::active().cosmic().space_xxs())
        .width(Length::FillPortion(portion))
        .height(Length::Fill)
        .into()
    }

    fn power_slot(
        &self,
        position: PowerMenuPosition,
        config: &AppletConfig,
        width: Length,
    ) -> Option<Element<'_, Message>> {
        self.slot(
            Slot::Power(position),
            (config.power_menu_position == position).then_some(Block::PowerMenu),
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
                            icon::from_svg_bytes(category.icon_svg_bytes)
                                .symbolic(true)
                                .icon()
                                .size(10),
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
