use std::path::PathBuf;
use std::sync::Arc;

use cosmic::cosmic_theme::Spacing;
use cosmic::iced::{
    Alignment, Length,
    widget::{column, row},
};
use cosmic::iced::core::text::{Ellipsize, EllipsizeHeightLimit, Wrapping};
use cosmic::iced::{Background, ContentFit, Font, Limits, Padding};
use cosmic::widget::text;
use cosmic::iced::clipboard::dnd::DndAction;
use cosmic::iced::widget::scrollable::{Direction, Scrollbar};
use cosmic::widget::{DndDestination, container, menu};
use cosmic::{Element, theme};

use crate::applet::{Applet, FavoritesDropTarget, FavoritesRegion, HomeSections, LeadingApp, Message};
use crate::config::{AppView, FeaturedApplications, ItemSize};
use crate::layout::{
    AppListOptions, Block, Column, FeaturedOptions, PowerOptions, Row, SidePanelContent,
    SidePanelOptions, UserName, UserOptions,
};
use crate::fl;
use crate::model::application_category::{ApplicationCategory, CategoryIcon};

/// All apps, first of the categories.
static FEATURED_ALL: ApplicationCategory = ApplicationCategory::ALL;
use crate::model::application_entry::ApplicationEntry;
use crate::model::dragged_app::DraggedApp;
use crate::model::place::Place;
use crate::widgets::{VirtualizedAppGrid, VirtualizedAppList};

/// What an entry of the context menu does to the app it was opened on,
/// [`crate::applet::Applet::context_menu_target`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextMenuAction {
    LaunchApplication,
    /// Launches the app with its desktop action at this index.
    LaunchApplicationWithAction(usize),
    /// Pins the app to the dock, or unpins it if it is pinned.
    PinToPanel(bool),
    ToggleFavorite,
}

impl menu::Action for ContextMenuAction {
    type Message = Message;
    fn message(&self) -> Self::Message {
        match self {
            ContextMenuAction::LaunchApplication => Message::LaunchContextMenuTarget(None),
            ContextMenuAction::LaunchApplicationWithAction(action_index) => {
                Message::LaunchContextMenuTarget(Some(*action_index))
            }
            ContextMenuAction::PinToPanel(pinned) => Message::PinContextMenuTarget(*pinned),
            ContextMenuAction::ToggleFavorite => Message::ToggleContextMenuTargetFavorite,
        }
    }
}

/// How the favorites around a [`AppletMenu::favorite_slot`] are laid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FavoriteSlotAxis {
    Vertical,
    Horizontal,
}

pub struct AppletMenu;

/// How the power controls are laid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PowerMenuStyle {
    /// Centered row with vertical padding, at the bottom of a pane or in a
    /// row of its own.
    Padded,
    /// Plain row, sharing a row with other blocks.
    Compact,
    /// Column, in a sidebar.
    Vertical,
}

/// Where a block is in the layout, which tells how it is laid out.
#[derive(Clone, Copy, Debug)]
struct BlockSlot {
    /// Whether its row shares the menu's height.
    tall_row: bool,
    /// Whether its row is a colored bar.
    banner: bool,
    /// Whether its column shares the row's width.
    wide_column: bool,
    /// Whether it is the only block of its row.
    alone: bool,
    /// Whether its column is the first of the row.
    leading: bool,
}

/// Colored bar, e.g. at the top and bottom of the retro layout.
fn banner_style(theme: &cosmic::Theme) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        text_color: Some(cosmic.on_accent_color().into()),
        icon_color: Some(cosmic.on_accent_color().into()),
        background: Some(Background::Color(cosmic.accent_color().into())),
        border: cosmic::iced::Border {
            radius: cosmic.radius_s().into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Menu entry, `highlighted` when it is selected, with the theme's small
/// corner radius.
///
/// libcosmic's `AppletMenu` style is always square and `Suggested` takes the
/// theme's large button radius, so neither follows the menu item corners.
pub fn menu_item_class(highlighted: bool) -> cosmic::theme::Button {
    use cosmic::widget::button::{Catalog, Style};

    fn base(highlighted: bool) -> cosmic::theme::Button {
        if highlighted {
            cosmic::theme::Button::Suggested
        } else {
            cosmic::theme::Button::AppletMenu
        }
    }
    fn rounded(mut style: Style, theme: &cosmic::Theme) -> Style {
        style.border_radius = theme.cosmic().radius_s().into();
        style
    }

    cosmic::theme::Button::Custom {
        active: Box::new(move |focused, theme| {
            rounded(theme.active(focused, false, &base(highlighted)), theme)
        }),
        disabled: Box::new(move |theme| rounded(theme.disabled(&base(highlighted)), theme)),
        hovered: Box::new(move |focused, theme| {
            rounded(theme.hovered(focused, false, &base(highlighted)), theme)
        }),
        pressed: Box::new(move |focused, theme| {
            rounded(theme.pressed(focused, false, &base(highlighted)), theme)
        }),
    }
}

/// Icon button readable on top of [`banner_style`].
fn banner_button_class() -> cosmic::theme::Button {
    fn on_accent(mut style: cosmic::widget::button::Style, theme: &cosmic::Theme) -> cosmic::widget::button::Style {
        let on_accent = theme.cosmic().on_accent_color().into();
        style.icon_color = Some(on_accent);
        style.text_color = Some(on_accent);
        style.background = None;
        style
    }
    use cosmic::widget::button::Catalog;

    cosmic::theme::Button::Custom {
        active: Box::new(|focused, theme| {
            on_accent(theme.active(focused, false, &cosmic::theme::Button::Icon), theme)
        }),
        disabled: Box::new(|theme| {
            on_accent(theme.disabled(&cosmic::theme::Button::Icon), theme)
        }),
        hovered: Box::new(|focused, theme| {
            let mut style = on_accent(theme.hovered(focused, false, &cosmic::theme::Button::Icon), theme);
            style.background = Some(Background::Color(cosmic::iced::Color::from_rgba(1.0, 1.0, 1.0, 0.15)));
            style
        }),
        pressed: Box::new(|focused, theme| {
            let mut style = on_accent(theme.pressed(focused, false, &cosmic::theme::Button::Icon), theme);
            style.background = Some(Background::Color(cosmic::iced::Color::from_rgba(1.0, 1.0, 1.0, 0.25)));
            style
        }),
    }
}

/// The menu on its transparent surface.
///
/// Translucent when the theme frosts applets, but not blurred: cosmic-panel
/// does not pass a blur of the menu on to the compositor, and the surface's
/// own blur would cover all of it rather than just the menu.
fn menu_surface_style(theme: &cosmic::Theme) -> container::Style {
    let cosmic = theme.cosmic();
    let background = cosmic.background(cosmic.frosted_applets);
    container::Style {
        text_color: Some(background.on.into()),
        icon_color: Some(background.on.into()),
        background: Some(Background::Color(background.base.into())),
        border: cosmic::iced::Border {
            radius: cosmic.corner_radii.radius_m.into(),
            width: 1.0,
            color: background.divider.into(),
        },
        ..Default::default()
    }
}

/// Background of a tinted side panel, as in the retro layout.
fn tinted_pane_style(theme: &cosmic::Theme) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        background: Some(Background::Color(cosmic.primary_container_color().into())),
        border: cosmic::iced::Border {
            radius: cosmic.radius_s().into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

impl AppletMenu {
    /// Gap between the menu and the edges of its surface.
    const SURFACE_PADDING: u16 = 4;
    /// Size of the favorites' icons in the sidebar.
    const SIDEBAR_ICON_SIZE: u16 = 28;

    const USER_IDLE_SYMBOLIC: &[u8] =
        include_bytes!("../../res/icons/bundled/user-idle-symbolic.svg");

    pub fn view_main_menu_list(applet: &Applet) -> Element<'_, Message> {
        let Spacing { space_xs, .. } = theme::active().cosmic().spacing;

        let menu_layout = AppletMenu::view_layout(applet);

        let (width, height) = applet.config.menu_size();
        let (width, height) = (f32::from(width), f32::from(height));

        let content = container(menu_layout)
            .padding(space_xs)
            .width(Length::Fixed(width))
            .height(Length::Fixed(height));

        if !applet.menu_is_layer_surface {
            return applet
                .core
                .applet
                .popup_container(content)
                .limits(
                    Limits::NONE
                        .max_height(height)
                        .min_height(height)
                        .max_width(width)
                        .min_width(width),
                )
                .into();
        }

        let menu = container(content).class(cosmic::theme::Container::custom(menu_surface_style));

        let placement = applet.menu_placement;
        let surface = cosmic::widget::mouse_area(
            container(cosmic::widget::mouse_area(menu).on_press(Message::MenuPressed))
                .padding(AppletMenu::SURFACE_PADDING)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(placement.align_x)
                .align_y(placement.align_y),
        )
        .on_press(Message::CloseMenu);

        let mut layers = cosmic::iced::widget::Stack::new()
            .push(surface)
            .width(Length::Fill)
            .height(Length::Fill);
        // Follows the pointer over the surface, which fills the screen.
        if let Some(drag) = applet.app_drag.as_ref()
            && let Some(position) = drag.position
        {
            let half = VirtualizedAppList::DRAG_PREVIEW_SIZE / 2.;
            layers = layers.push(
                container(VirtualizedAppList::drag_preview::<Message>(drag.icon.as_ref()))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(cosmic::iced::Padding {
                        top: (position.y - half).max(0.),
                        left: (position.x - half).max(0.),
                        ..Default::default()
                    }),
            );
        }
        layers.into()
    }

    /// The rows of the layout, see [`crate::layout`].
    fn view_layout(applet: &Applet) -> Element<'_, Message> {
        let space_xxs = theme::active().cosmic().space_xxs();
        let rows: Vec<Element<'_, Message>> = applet
            .config
            .layout
            .rows
            .iter()
            .filter_map(|row| AppletMenu::view_row(applet, row))
            .collect();

        column(rows)
            .spacing(space_xxs)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// The columns of `row` side by side, if any of them shows something.
    fn view_row<'a>(applet: &'a Applet, layout_row: &'a Row) -> Option<Element<'a, Message>> {
        let Spacing {
            space_xxs, space_xs, space_s, ..
        } = theme::active().cosmic().spacing;
        let tall_row = layout_row.fills_height();
        let alone = layout_row.columns.len() == 1 && layout_row.columns[0].blocks.len() == 1;

        let columns: Vec<Element<'a, Message>> = layout_row
            .columns
            .iter()
            .enumerate()
            .filter_map(|(index, column)| {
                let slot = BlockSlot {
                    tall_row,
                    banner: layout_row.banner,
                    wide_column: column.fills_width(),
                    alone,
                    leading: index == 0,
                };
                AppletMenu::view_column(applet, column, slot)
            })
            .collect();
        if columns.is_empty() {
            return None;
        }

        let content: Element<'a, Message> = if tall_row {
            // Panes side by side, as in the original menu.
            let mut items = Vec::with_capacity(2 * columns.len());
            for (index, column) in columns.into_iter().enumerate() {
                if index > 0 {
                    items.push(AppletMenu::vertical_divider());
                }
                items.push(column);
            }
            row(items).width(Length::Fill).height(Length::Fill).into()
        } else {
            row(columns)
                .spacing(space_xs)
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .into()
        };

        if !layout_row.banner {
            return Some(content);
        }
        Some(
            container(content)
                .padding([space_xxs, space_s])
                .width(Length::Fill)
                .height(if tall_row { Length::Fill } else { Length::Shrink })
                .class(cosmic::theme::Container::custom(banner_style))
                .into(),
        )
    }

    /// The blocks of `column` stacked, if any of them shows something.
    fn view_column<'a>(
        applet: &'a Applet,
        layout_column: &'a Column,
        slot: BlockSlot,
    ) -> Option<Element<'a, Message>> {
        let blocks: Vec<Element<'a, Message>> = layout_column
            .blocks
            .iter()
            .enumerate()
            .filter_map(|(index, block)| {
                let element = AppletMenu::view_block(applet, block, slot)?;
                let padding = layout_column.padding(index);
                if padding.is_zero() {
                    return Some(element);
                }
                Some(
                    container(element)
                        .padding(Padding {
                            top: f32::from(padding.top),
                            right: f32::from(padding.right),
                            bottom: f32::from(padding.bottom),
                            left: f32::from(padding.left),
                        })
                        .into(),
                )
            })
            .collect();
        if blocks.is_empty() {
            return None;
        }

        Some(
            column(blocks)
                .spacing(theme::active().cosmic().space_xxs())
                .width(if slot.wide_column {
                    Length::FillPortion(layout_column.portion.max(1))
                } else {
                    Length::Shrink
                })
                .height(if slot.tall_row {
                    Length::Fill
                } else {
                    Length::Shrink
                })
                .align_x(if slot.wide_column {
                    Alignment::Start
                } else {
                    Alignment::Center
                })
                .into(),
        )
    }

    /// `block`, unless it hides at the moment, e.g. while searching.
    fn view_block<'a>(
        applet: &'a Applet,
        block: &'a Block,
        slot: BlockSlot,
    ) -> Option<Element<'a, Message>> {
        let searching = !applet.search_field.is_empty();

        Some(match block {
            Block::User(options) => AppletMenu::create_logged_user_widget(applet, options),
            Block::Search(options) => AppletMenu::create_search_field(applet, options.size),
            Block::AppList(options) => AppletMenu::create_app_list(applet, options),
            Block::SidePanel(options) => {
                if options.hide_while_searching && searching {
                    return None;
                }
                AppletMenu::create_side_panel(applet, options)
            }
            Block::Power(options) => {
                let style = if options.vertical {
                    PowerMenuStyle::Vertical
                } else if slot.alone || (slot.tall_row && slot.wide_column) {
                    PowerMenuStyle::Padded
                } else {
                    PowerMenuStyle::Compact
                };
                AppletMenu::create_power_menu(
                    options,
                    style,
                    slot.banner,
                    applet.can_hibernate,
                    slot.leading,
                )
            }
            Block::Featured(options) => {
                if options.hide_while_searching && searching {
                    return None;
                }
                if options.vertical {
                    AppletMenu::create_featured_bar(applet, options, slot.leading)
                } else {
                    AppletMenu::create_featured_rows(applet, options)?
                }
            }
            Block::Spacer => {
                if slot.tall_row {
                    cosmic::widget::Space::new().height(Length::Fill).into()
                } else {
                    cosmic::widget::Space::new().width(Length::Fill).into()
                }
            }
            Block::Divider => AppletMenu::horizontal_divider(),
        })
    }

    /// The apps of a bar, falling back to the most used apps while there
    /// are no favorites in a column, so it stays useful.
    fn featured_apps<'a>(
        applet: &'a Applet,
        options: &FeaturedOptions,
    ) -> &'a [Arc<ApplicationEntry>] {
        let apps = match options.source {
            FeaturedApplications::Favorites
                if options.vertical && applet.favorite_applications.is_empty() =>
            {
                &applet.recent_applications[..applet.recent_applications.len().min(6)]
            }
            FeaturedApplications::Favorites => &applet.favorite_applications[..],
            FeaturedApplications::RecentlyUsed => &applet.recent_applications[..],
        };
        &apps[..apps.len().min(options.count())]
    }

    /// A column of icons of favorite or recently used apps (Cinnamon like).
    /// `leading` tells whether it is at the left of the menu, so the names
    /// show towards the rest of it.
    fn create_featured_bar<'a>(
        applet: &'a Applet,
        options: &FeaturedOptions,
        leading: bool,
    ) -> Element<'a, Message> {
        let space_xxs = theme::active().cosmic().space_xxs();
        let apps = AppletMenu::featured_apps(applet, options);
        // Shown in their order, they can be reordered by dragging.
        let favorites = options.source == FeaturedApplications::Favorites
            && !applet.favorite_applications.is_empty();

        // Apps can be dragged, e.g. into the dock.
        let items = apps.iter().map(|app| {
            let button = cosmic::widget::tooltip(
                cosmic::widget::button::custom(VirtualizedAppList::create_icon_widget(
                    app,
                    AppletMenu::SIDEBAR_ICON_SIZE,
                ))
                .on_press(Message::ApplicationSelected(app.clone()))
                .class(menu_item_class(false))
                .padding(space_xxs),
                text(&app.name),
                if leading {
                    cosmic::widget::tooltip::Position::Right
                } else {
                    cosmic::widget::tooltip::Position::Left
                },
            );
            let button = cosmic::widget::mouse_area(button)
                .on_right_press(Message::ContextMenuTarget(app.clone()));
            let item = container(VirtualizedAppList::drag_source(applet, app, button))
                .width(Length::Fill)
                .align_x(Alignment::Center);
            if favorites {
                AppletMenu::favorite_slot(
                    applet,
                    app,
                    FavoritesRegion::Bar,
                    FavoriteSlotAxis::Vertical,
                    (Length::Fill, Length::Shrink),
                    item,
                )
            } else {
                item.into()
            }
        });
        let items = cosmic::widget::column::with_children(items)
            .spacing(space_xxs)
            .align_x(Alignment::Center);

        // Recently used apps can't be reordered, so with them the whole bar
        // adds to the favorites. Otherwise each favorite is a drop target
        // and the space below them is the end, as drop targets must not be
        // nested.
        let content: Element<'a, Message> = match (options.source, favorites) {
            (FeaturedApplications::RecentlyUsed, _) => cosmic::widget::scrollable(items)
                .width(Length::Fill)
                .height(Length::Fill)
                .direction(Direction::Vertical(
                    Scrollbar::new().width(0).scroller_width(0).margin(0),
                ))
                .into(),
            (FeaturedApplications::Favorites, false) => AppletMenu::favorites_drop_target(
                container(items)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .class(cosmic::theme::Container::custom(AppletMenu::drop_target_style(
                        applet.favorites_drop_target == Some(FavoritesDropTarget::Add),
                    ))),
                FavoritesDropTarget::Add,
            ),
            (FeaturedApplications::Favorites, true) => {
                let end = container(cosmic::widget::Space::new())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .class(cosmic::theme::Container::custom(AppletMenu::drop_target_style(
                        applet.favorites_drop_target == Some(FavoritesDropTarget::End),
                    )));
                // Many favorites would otherwise get squeezed. The bar is
                // hidden as it would cover the icons in such a narrow column.
                let items = cosmic::widget::scrollable(items)
                    .width(Length::Fill)
                    .direction(Direction::Vertical(
                        Scrollbar::new().width(0).scroller_width(0).margin(0),
                    ));
                column![
                    items,
                    AppletMenu::favorites_drop_target(end, FavoritesDropTarget::End),
                ]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            }
        };

        // A fixed width, as the drop target fills it and would otherwise
        // make the bar take a share of the row's width.
        VirtualizedAppList::with_context_menu(
            applet,
            container(content)
                .width(Length::Fixed(f32::from(
                    AppletMenu::SIDEBAR_ICON_SIZE + 2 * space_xxs,
                )))
                .height(Length::Fill),
        )
    }

    /// Apps per row in a row of favorite or recently used apps.
    const FEATURED_ROW_LENGTH: usize = 4;

    /// Rows of favorite or recently used apps, under a heading, as below
    /// the app grid of the modern layout. Recently used apps are left out
    /// until there are any.
    fn create_featured_rows<'a>(
        applet: &'a Applet,
        options: &FeaturedOptions,
    ) -> Option<Element<'a, Message>> {
        let space_xxs = theme::active().cosmic().space_xxs();
        let apps = AppletMenu::featured_apps(applet, options);
        let favorites = options.source == FeaturedApplications::Favorites;
        if !favorites && apps.is_empty() {
            return None;
        }

        let content: Element<'a, Message> = if favorites {
            let rows: Element<'a, Message> = if apps.is_empty() {
                text::caption(fl!("favorites-empty")).into()
            } else {
                AppletMenu::featured_grid(applet, apps, true)
            };
            let rows = container(rows).width(Length::Fill).padding(2).class(
                cosmic::theme::Container::custom(AppletMenu::drop_target_style(
                    applet.favorites_drop_target == Some(FavoritesDropTarget::Add),
                )),
            );
            // Filled, each favorite is a drop target of its own.
            if apps.is_empty() {
                AppletMenu::favorites_drop_target(rows, FavoritesDropTarget::Add)
            } else {
                rows.into()
            }
        } else {
            AppletMenu::featured_grid(applet, apps, false)
        };
        let content = VirtualizedAppList::with_context_menu(applet, content);

        if !options.show_heading {
            return Some(content);
        }
        let heading = if favorites {
            fl!("favorites")
        } else {
            fl!("recently-used")
        };
        Some(
            column![
                cosmic::widget::divider::horizontal::default(),
                text::heading(heading),
                content,
            ]
            .spacing(space_xxs)
            .width(Length::Fill)
            .into(),
        )
    }

    /// `apps` in rows of [`AppletMenu::FEATURED_ROW_LENGTH`], which can be
    /// reordered by dragging if they are the `favorites`.
    fn featured_grid<'a>(
        applet: &'a Applet,
        apps: &'a [Arc<ApplicationEntry>],
        favorites: bool,
    ) -> Element<'a, Message> {
        let Spacing {
            space_xxs, space_xs, ..
        } = theme::active().cosmic().spacing;

        let rows = apps.chunks(Self::FEATURED_ROW_LENGTH).map(|chunk| {
            let mut buttons = chunk
                .iter()
                .map(|app| {
                    let button = cosmic::widget::button::custom(
                        row![
                            VirtualizedAppList::create_icon_widget(app, 24),
                            text(&app.name)
                                .wrapping(Wrapping::None)
                                .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1))),
                        ]
                        .spacing(space_xs)
                        .align_y(Alignment::Center),
                    )
                    .on_press(Message::ApplicationSelected(app.clone()))
                    .class(menu_item_class(false));
                    let right_click = |button| {
                        cosmic::widget::mouse_area(button)
                            .on_right_press(Message::ContextMenuTarget(app.clone()))
                    };

                    if favorites {
                        AppletMenu::favorite_slot(
                            applet,
                            app,
                            FavoritesRegion::Bar,
                            FavoriteSlotAxis::Horizontal,
                            (Length::FillPortion(1), Length::Shrink),
                            VirtualizedAppList::drag_source(
                                applet,
                                app,
                                right_click(button.width(Length::Fill)),
                            ),
                        )
                    } else {
                        container(right_click(button.width(Length::Fill)))
                            .width(Length::FillPortion(1))
                            .into()
                    }
                })
                .collect::<Vec<Element<'_, Message>>>();
            // Keep the apps of a partial row as wide as the others.
            while buttons.len() < Self::FEATURED_ROW_LENGTH {
                buttons.push(cosmic::widget::Space::new().width(Length::FillPortion(1)).into());
            }
            row(buttons).spacing(space_xxs).into()
        });

        cosmic::widget::column::with_children(rows.collect::<Vec<Element<'a, Message>>>())
            .spacing(space_xxs)
            .into()
    }

    /// Outlines a drop target while an app is dragged over it.
    fn drop_target_style(hovered: bool) -> impl Fn(&cosmic::Theme) -> container::Style {
        move |theme| {
            let cosmic = theme.cosmic();
            let mut style = container::Style::default();
            if hovered {
                style.border = cosmic::iced::Border {
                    color: cosmic.accent_color().into(),
                    width: 2.0,
                    radius: cosmic.radius_s().into(),
                };
            }
            style
        }
    }

    fn vertical_divider() -> Element<'static, Message> {
        cosmic::applet::padded_control(cosmic::widget::divider::vertical::default())
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .width(Length::Shrink)
            .padding(5)
            .into()
    }

    fn create_power_menu(
        options: &PowerOptions,
        style: PowerMenuStyle,
        banner: bool,
        can_hibernate: bool,
        leading: bool,
    ) -> Element<'static, Message> {
        let buttons = options
            .actions
            .iter()
            .filter(|action| action.is_available(can_hibernate))
            .map(|&action| {
                let button = cosmic::widget::button::icon(
                    cosmic::widget::icon::from_svg_bytes(action.icon()).symbolic(true),
                )
                .on_press(Message::PowerOptionSelected(action));
                let button: Element<'static, Message> = if banner {
                    button.class(banner_button_class()).into()
                } else {
                    button.into()
                };
                if !options.show_tooltips {
                    return button;
                }
                // Stacked buttons are in a column at a side of the menu, so
                // their names show beside them, towards the rest of it.
                // Otherwise above, as the buttons are mostly at the bottom,
                // where names below would be cut off by the menu's edge.
                let position = match style {
                    PowerMenuStyle::Vertical if leading => cosmic::widget::tooltip::Position::Right,
                    PowerMenuStyle::Vertical => cosmic::widget::tooltip::Position::Left,
                    _ => cosmic::widget::tooltip::Position::Top,
                };
                cosmic::widget::tooltip(button, text::caption(action.name()), position).into()
            })
            .collect::<Vec<Element<'_, Message>>>();

        match style {
            PowerMenuStyle::Padded => container(row(buttons).align_y(Alignment::Center))
                .width(Length::Fill)
                .padding([20, 0])
                .align_x(Alignment::Center)
                .into(),
            PowerMenuStyle::Compact => row(buttons).align_y(Alignment::Center).into(),
            PowerMenuStyle::Vertical => column(buttons).align_x(Alignment::Center).into(),
        }
    }

    fn create_search_field(applet: &Applet, size: ItemSize) -> Element<'_, Message> {
        let Spacing {
            space_xxs, space_xs, space_s, ..
        } = theme::active().cosmic().spacing;
        let (padding, text_size) = match size {
            ItemSize::Small => (2, 12),
            ItemSize::Medium => (space_xxs, 14),
            ItemSize::Large => (space_xs, 17),
        };

        cosmic::widget::search_input(fl!("search-placeholder"), &applet.search_field)
            .on_input(Message::SearchFieldInput)
            .on_clear(Message::SearchCleared)
            .width(Length::Fill)
            .always_active()
            .padding([padding, space_s])
            .size(text_size)
            .into()
    }

    /// The apps, in the configured view.
    fn create_app_view<'a>(applet: &'a Applet, options: &AppListOptions) -> Element<'a, Message> {
        if let Some(home) = applet.home.filter(|home| home.start_page) {
            return AppletMenu::create_start_page(applet, home);
        }

        let view = match options.view {
            AppView::Column => VirtualizedAppList::view(applet),
            AppView::Grid => VirtualizedAppGrid::view(applet),
        };
        // Search results are left by clearing the search. The button takes
        // the place of the one leading here, at the bottom.
        if options.has_start_page() && applet.search_field.is_empty() {
            column![
                view,
                AppletMenu::horizontal_divider(),
                AppletMenu::page_button(fl!("back"), "go-previous-symbolic", true)
                    .on_press(Message::ShowStartPage),
            ]
            .height(Length::Fill)
            .into()
        } else {
            view
        }
    }

    /// Height of the headings of the search results.
    const SEARCH_HEADING_HEIGHT: f32 = 28.0;
    /// Height of the dividers between the rows of a list.
    const LIST_DIVIDER_HEIGHT: f32 = 1.0;

    /// Where the parts of [`AppletMenu::search_header`] are, if shown: tops
    /// of the results of pop-launcher, in their order, and the height of the
    /// whole, after which the apps found follow. The apps' lists use it to
    /// tell which apps are in view, and the arrow keys to scroll to them.
    pub(crate) fn search_header_layout(applet: &Applet) -> Option<SearchHeaderLayout> {
        let shown = applet.shown_launcher_results();
        if shown.is_empty() {
            return None;
        }
        let spacing = f32::from(theme::active().cosmic().space_xxs());
        let item_height = f32::from(VirtualizedAppList::item_metrics(applet.config.app_list().item_size).1);

        let mut result_tops = Vec::with_capacity(shown.len());
        let mut y = 0.;
        for (_, results) in crate::logic::launcher::grouped_refs(&shown) {
            y += Self::SEARCH_HEADING_HEIGHT + spacing;
            for _ in &results {
                result_tops.push(y);
                y += item_height + Self::LIST_DIVIDER_HEIGHT;
            }
            y += spacing - Self::LIST_DIVIDER_HEIGHT;
        }
        if !applet.available_applications.is_empty() {
            y += Self::SEARCH_HEADING_HEIGHT + spacing;
        }
        Some(SearchHeaderLayout {
            result_tops,
            height: y,
        })
    }

    /// Results of pop-launcher's plugins for the search, such as the
    /// calculator, files, open windows or web searches, each kind under a
    /// heading of its own, and the heading of the apps found below. Shown at
    /// the top of the apps' list, scrolling with it, and laid out as
    /// [`AppletMenu::search_header_layout`] tells.
    pub(crate) fn search_header<'a>(applet: &'a Applet) -> Option<Element<'a, Message>> {
        let shown = applet.shown_launcher_results();
        if shown.is_empty() {
            return None;
        }
        let Spacing { space_xxs, space_s, .. } = theme::active().cosmic().spacing;
        let (icon_size, item_height) =
            VirtualizedAppList::item_metrics(applet.config.app_list().item_size);
        let highlighted = applet.default_launcher_result().map(|result| result.id);

        // As the apps' rows look, see `VirtualizedAppList::create_app_button`.
        let row = |result: &'a crate::logic::launcher::SearchResult| -> Element<'a, Message> {
            let description = (!result.description.is_empty()).then(|| {
                text(&result.description)
                    .size(VirtualizedAppList::DESCRIPTION_SIZE)
                    .wrapping(Wrapping::None)
                    .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)))
            });
            cosmic::widget::button::custom(
                row![
                    AppletMenu::launcher_icon(result, icon_size),
                    cosmic::widget::Space::new().width(5).height(Length::Fill),
                    column![
                        text(&result.name)
                            .wrapping(Wrapping::None)
                            .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1))),
                    ]
                    .push_maybe(description),
                ]
                .align_y(Alignment::Center),
            )
            .on_press(Message::LauncherResultActivated(result.id))
            .class(menu_item_class(highlighted == Some(result.id)))
            .padding([0, space_s])
            .width(Length::Fill)
            .height(item_height)
            .into()
        };
        let heading = |title: String| -> Element<'a, Message> {
            container(text::heading(title))
                .height(Length::Fixed(Self::SEARCH_HEADING_HEIGHT))
                .align_y(Alignment::Center)
                .into()
        };

        let mut header = column![].spacing(space_xxs);
        for (group, results) in crate::logic::launcher::grouped_refs(&shown) {
            let list = results.into_iter().fold(
                cosmic::widget::list_column().list_item_padding(0),
                |list, result| list.add(row(result)),
            );
            header = header
                .push(heading(AppletMenu::result_group_name(group)))
                .push(list);
        }
        if !applet.available_applications.is_empty() {
            header = header.push(heading(fl!("search-applications")));
        }
        // Spaced from the apps as its parts are from each other.
        Some(container(header).padding(Padding { bottom: f32::from(space_xxs), ..Padding::ZERO }).into())
    }

    /// Heading of a kind of results of pop-launcher.
    fn result_group_name(group: crate::logic::launcher::ResultGroup) -> String {
        use crate::logic::launcher::ResultGroup;
        match group {
            ResultGroup::Calculator => fl!("search-calculator"),
            ResultGroup::Windows => fl!("search-windows"),
            ResultGroup::Files => fl!("search-files"),
            ResultGroup::Web => fl!("search-web"),
            ResultGroup::Commands => fl!("search-commands"),
            ResultGroup::Sound => fl!("search-sound"),
            ResultGroup::Other => fl!("search-other"),
        }
    }

    /// Icon of a result of pop-launcher, by name, path or MIME type.
    fn launcher_icon(
        result: &crate::logic::launcher::SearchResult,
        size: u16,
    ) -> Element<'static, Message> {
        use crate::logic::launcher::IconSource;
        use cosmic::widget::icon;

        let fallback = || {
            Some(icon::IconFallback::Names(vec![
                "application-default".into(),
                "application-x-executable".into(),
            ]))
        };
        let handle = match result.icon.as_ref().or(result.category_icon.as_ref()) {
            Some(IconSource::Name(name)) if name.starts_with('/') => {
                icon::from_path(PathBuf::from(name))
            }
            Some(IconSource::Name(name)) => {
                icon::from_name(name.clone()).size(size).fallback(fallback()).handle()
            }
            Some(IconSource::Mime(mime)) => icon::from_name(mime.replace('/', "-"))
                .size(size)
                .fallback(fallback())
                .handle(),
            None => icon::from_name("system-search-symbolic").size(size).handle(),
        };
        icon(handle).size(size).into()
    }

    /// The favorites, then the recently used apps, which lead the
    /// `available_applications` on the start page, and a button to all apps.
    fn create_start_page(applet: &Applet, home: HomeSections) -> Element<'_, Message> {
        let favorites = home.favorites;
        let mut rows = applet
            .available_applications
            .iter()
            .enumerate()
            .map(|(index, app)| VirtualizedAppList::create_app_button(applet, index, app));
        let favorite_rows: Vec<Element<'_, Message>> = rows.by_ref().take(favorites).collect();
        let recent_rows: Vec<Element<'_, Message>> = rows.collect();

        // Without favorites, their section only shows when the page would
        // otherwise be empty, as a place to drop apps on, telling how to
        // add them. With favorites turned off, or recently used apps to
        // show, it would only take room.
        let placeholder = favorite_rows.is_empty()
            && recent_rows.is_empty()
            && applet.config.app_list().favorites;
        let favorites: Option<Element<'_, Message>> = if !favorite_rows.is_empty() {
            Some(column(favorite_rows).into())
        } else if placeholder {
            Some(
                container(text::caption(fl!("favorites-empty")))
                    .padding(theme::active().cosmic().space_xs())
                    .width(Length::Fill)
                    .into(),
            )
        } else {
            None
        };

        let mut list = column![];
        if let Some(favorites) = favorites {
            // A tinted background sets the favorites apart from the
            // recently used apps.
            // Dropped apps go first, as only the first favorites are shown.
            let drop_target = FavoritesDropTarget::First;
            let drop_hovered = applet.favorites_drop_target == Some(drop_target);
            let favorites = container(favorites).width(Length::Fill).class(
                cosmic::theme::Container::custom(move |theme| {
                    let mut style = AppletMenu::drop_target_style(drop_hovered)(theme);
                    let cosmic = theme.cosmic();
                    let mut tint: cosmic::iced::Color = cosmic.accent_color().into();
                    tint.a = 0.12;
                    style.background = Some(Background::Color(tint));
                    style.border.radius = cosmic.radius_s().into();
                    style
                }),
            );
            // Filled, each favorite is a drop target of its own.
            list = list.push(if placeholder {
                AppletMenu::favorites_drop_target(favorites, drop_target)
            } else {
                favorites.into()
            });
            if !recent_rows.is_empty() {
                list = list.push(AppletMenu::horizontal_divider());
            }
        }
        list = list.extend(recent_rows);

        let list = cosmic::widget::scrollable(list)
            .height(Length::Fill)
            .width(Length::Fill)
            .id(applet.scrollable_id.clone())
            .on_scroll(Message::ScrollUpdated);

        column![
            VirtualizedAppList::with_context_menu(applet, list),
            AppletMenu::horizontal_divider(),
            AppletMenu::page_button(fl!("all-applications"), "go-next-symbolic", false)
                .on_press(Message::ShowAllApplications),
        ]
        .height(Length::Fill)
        .into()
    }

    /// Full width button moving between the start page and all apps, with
    /// an arrow before (`leading`) or after its label.
    fn page_button(
        label: String,
        arrow: &'static str,
        leading: bool,
    ) -> cosmic::widget::Button<'static, Message> {
        let Spacing {
            space_xxs,
            space_xs,
            space_s,
            ..
        } = theme::active().cosmic().spacing;
        let arrow = cosmic::widget::icon::from_name(arrow).size(16).icon();
        let label = text::body(label).font(cosmic::font::bold());

        let content = if leading {
            row![arrow, label]
        } else {
            row![label, cosmic::widget::Space::new().width(Length::Fill), arrow]
        };
        cosmic::widget::button::custom(
            content.spacing(space_xs).align_y(Alignment::Center),
        )
        .class(menu_item_class(false))
        // Lines up with the apps in the list.
        .padding([space_xxs, space_s])
        .width(Length::Fill)
    }

    /// The app list, under a heading if it has one. The start page is not
    /// all apps, so it has none.
    fn create_app_list<'a>(applet: &'a Applet, options: &AppListOptions) -> Element<'a, Message> {
        let heading = (options.show_heading && !applet.showing_start_page())
            .then(|| text::heading(fl!("all-applications")));

        column![]
            .push_maybe(heading)
            .push(AppletMenu::create_app_view(applet, options))
            .spacing(theme::active().cosmic().space_xxs())
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }

    /// Height of [`AppletMenu::leading_separator`].
    pub(crate) const LEADING_SEPARATOR_HEIGHT: f32 = 13.0;

    /// The line between the apps leading all apps and all apps, with room
    /// around it, as between the sections of the start page.
    pub(crate) fn leading_separator() -> Element<'static, Message> {
        container(AppletMenu::horizontal_divider())
            .width(Length::Fill)
            .height(Length::Fixed(Self::LEADING_SEPARATOR_HEIGHT))
            .align_y(Alignment::Center)
            .into()
    }

    /// Tints the apps leading the list of all apps, see
    /// [`Applet::leading_app`]. [`AppletMenu::leading_separator`] sets them
    /// apart from all apps.
    pub(crate) fn leading_app<'a>(
        applet: &Applet,
        index: usize,
        content: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        let Some(leading) = applet.leading_app(index) else {
            return content.into();
        };
        let alpha = match leading {
            LeadingApp::Favorite => 0.12,
            LeadingApp::Recent => 0.05,
        };
        container(content)
            .width(Length::Fill)
            .class(cosmic::theme::Container::custom(move |theme| {
                let cosmic = theme.cosmic();
                let mut tint: cosmic::iced::Color = cosmic.accent_color().into();
                tint.a = alpha;
                container::Style {
                    background: Some(Background::Color(tint)),
                    ..Default::default()
                }
            }))
            .into()
    }

    fn create_place_buttons<'a>(options: &'a SidePanelOptions) -> Vec<Element<'a, Message>> {
        let Spacing { space_m, .. } = cosmic::theme::active().cosmic().spacing;

        let mut buttons = Vec::new();
        // Places that are not shown can leave separators at the ends or next
        // to each other, so they are only added in front of a place.
        let mut separator_pending = false;
        let mut places_shown = false;
        for place in options.places.iter().filter(|place| place.is_available()) {
            match place {
                Place::Separator => {
                    separator_pending = places_shown;
                    continue;
                }
                Place::Spacer => {
                    buttons.push(cosmic::widget::Space::new().height(Length::Fill).into());
                    continue;
                }
                _ => places_shown = true,
            }
            if std::mem::take(&mut separator_pending) {
                buttons.push(AppletMenu::horizontal_divider());
            }

            buttons.push(
                cosmic::widget::button::custom(
                    row![
                        container(cosmic::widget::icon::from_name(place.icon_name()).size(16))
                            .padding([0, space_m]),
                        text(place.display_name()),
                    ]
                    .align_y(Alignment::Center),
                )
                .on_press(Message::OpenPlace(*place))
                .class(menu_item_class(false))
                .width(Length::Fill)
                .into(),
            );
        }
        buttons
    }

    fn horizontal_divider() -> Element<'static, Message> {
        cosmic::applet::padded_control(cosmic::widget::divider::horizontal::default())
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .padding(5)
            .into()
    }

    /// Makes the favorite `app` shown in `content` a place to drop apps on,
    /// with a line showing where the dropped app ends up.
    ///
    /// The line is drawn over `content`, and always there if only invisibly,
    /// so that showing it moves nothing around and interrupts no drag.
    pub(crate) fn favorite_slot<'a>(
        applet: &Applet,
        app: &ApplicationEntry,
        region: FavoritesRegion,
        axis: FavoriteSlotAxis,
        (width, height): (Length, Length),
        content: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        const LINE_THICKNESS: f32 = 3.0;

        let favorites = &applet.config.favorite_applications;
        let Some(index) = favorites.iter().position(|id| *id == app.id) else {
            return content.into();
        };
        let target = FavoritesDropTarget::Onto { index, region };

        let dragged_from = applet
            .app_drag
            .as_ref()
            .and_then(|drag| favorites.iter().position(|id| *id == drag.app_id));
        // Dropping a favorite on itself leaves it where it is.
        let shown = applet.favorites_drop_target == Some(target) && dragged_from != Some(index);
        // Dragged down the list, it ends up after this favorite.
        let edge = if dragged_from.is_some_and(|from| from < index) {
            Alignment::End
        } else {
            Alignment::Start
        };

        let line = match axis {
            FavoriteSlotAxis::Vertical => container(cosmic::widget::Space::new())
                .width(Length::Fill)
                .height(Length::Fixed(LINE_THICKNESS)),
            FavoriteSlotAxis::Horizontal => container(cosmic::widget::Space::new())
                .width(Length::Fixed(LINE_THICKNESS))
                .height(Length::Fill),
        }
        .class(cosmic::theme::Container::custom(move |theme| {
            let cosmic = theme.cosmic();
            container::Style {
                background: shown.then(|| Background::Color(cosmic.accent_color().into())),
                border: cosmic::iced::Border {
                    radius: cosmic.radius_xs().into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }));
        let line = container(line).width(Length::Fill).height(Length::Fill);
        let line = match axis {
            FavoriteSlotAxis::Vertical => line.align_y(edge),
            FavoriteSlotAxis::Horizontal => line.align_x(edge),
        };

        let slot = cosmic::iced::widget::stack![content.into(), line]
            .width(width)
            .height(height);
        AppletMenu::favorites_drop_target(slot, target)
    }

    /// Places apps dragged from the menu and dropped on `content` in the
    /// favorites, as `target` says.
    ///
    /// Drop targets must not be nested, as the outer one would get all the
    /// drag events.
    fn favorites_drop_target<'a>(
        content: impl Into<Element<'a, Message>>,
        target: FavoritesDropTarget,
    ) -> Element<'a, Message> {
        DndDestination::for_data::<DraggedApp>(content, move |app, _| {
            Message::FavoriteDropped(app, target)
        })
        .drag_id(target.drag_id())
        .action(DndAction::Copy | DndAction::Move)
        .preferred_action(DndAction::Copy)
        .on_enter(move |_, _, _| Message::FavoritesDragEntered(target))
        .on_leave(move || Message::FavoritesDragLeft(target))
        .into()
    }

    fn create_category_icon(category_icon: &CategoryIcon) -> Element<'static, Message> {
        use cosmic::widget::icon;

        match category_icon {
            CategoryIcon::Bundled(bytes) => icon::from_svg_bytes(bytes.clone()).symbolic(true).icon(),
            CategoryIcon::Named(name) => icon::from_name(name.as_str())
                .size(16)
                .fallback(Some(icon::IconFallback::Names(vec!["folder-symbolic".into()])))
                .icon(),
            CategoryIcon::Path(path) => icon::icon(icon::from_path(path.clone())).size(16),
        }
        .into()
    }

    /// The categories the panel lists: all apps, the recently used apps
    /// and favorites if shown, then the others.
    pub fn listed_categories<'a>(
        categories: &'a [ApplicationCategory],
        options: &SidePanelOptions,
    ) -> Vec<&'a ApplicationCategory> {
        static FEATURED: [ApplicationCategory; 2] =
            [ApplicationCategory::RECENTLY_USED, ApplicationCategory::FAVORITES];
        let featured = [options.show_recently_used, options.show_favorites]
            .into_iter()
            .zip(&FEATURED)
            .filter_map(|(shown, category)| shown.then_some(category));
        let others = categories.iter().filter(|category| {
            **category != ApplicationCategory::ALL
                && !FEATURED.contains(category)
        });
        std::iter::once(&FEATURED_ALL)
            .chain(featured)
            .chain(others)
            .collect()
    }

    fn create_category_buttons<'a>(
        applet: &'a Applet,
        options: &SidePanelOptions,
    ) -> Vec<Element<'a, Message>> {
        let Spacing { space_m, .. } = cosmic::theme::active().cosmic().spacing;
        // Until they are loaded, there are none to list.
        if applet.available_categories.is_empty() {
            return Vec::new();
        }
        let listed = AppletMenu::listed_categories(&applet.available_categories, options);
        let featured = listed
            .iter()
            .take_while(|category| {
                [ApplicationCategory::ALL, ApplicationCategory::RECENTLY_USED, ApplicationCategory::FAVORITES]
                    .contains(category)
            })
            .count();

        let mut categories_pane: Vec<Element<Message>> = listed
            .into_iter()
            .map(|category| {
                let category = category.clone();
                let is_favorites = category == ApplicationCategory::FAVORITES;
                // The start page is not one of the categories, whichever
                // was selected before it.
                let selected = !applet.showing_start_page()
                    && applet.selected_category.as_ref() == Some(&category);
                let highlighted = selected
                    || (is_favorites
                        && applet.favorites_drop_target == Some(FavoritesDropTarget::Add));

                let button = cosmic::widget::button::custom(
                    row![
                        container(AppletMenu::create_category_icon(&category.icon))
                            .padding([0, space_m]),
                        text(category.get_display_name()),
                    ]
                    .align_y(Alignment::Center),
                )
                .class(menu_item_class(highlighted))
                .width(Length::Fill)
                .on_press(Message::CategorySelected(category));

                if is_favorites {
                    AppletMenu::favorites_drop_target(button, FavoritesDropTarget::Add)
                } else {
                    button.into()
                }
            })
            .collect();

        // Between the permanent categories and the others.
        if featured < categories_pane.len() {
            categories_pane.insert(featured, AppletMenu::horizontal_divider());
        }
        if applet.config.app_list().has_start_page() {
            categories_pane.insert(0, AppletMenu::create_home_button(applet));
        }

        categories_pane
    }

    /// Entry above the categories leading to the start page.
    fn create_home_button(applet: &Applet) -> Element<'_, Message> {
        let Spacing { space_m, .. } = cosmic::theme::active().cosmic().spacing;

        cosmic::widget::button::custom(
            row![
                container(cosmic::widget::icon::from_name(Place::Home.icon_name()).size(16))
                    .padding([0, space_m]),
                text(fl!("home-page")),
            ]
            .align_y(Alignment::Center),
        )
        .class(menu_item_class(applet.showing_start_page()))
        .width(Length::Fill)
        .on_press(Message::ShowStartPage)
        .into()
    }

    /// A pane showing categories or places.
    fn create_side_panel<'a>(
        applet: &'a Applet,
        options: &'a SidePanelOptions,
    ) -> Element<'a, Message> {
        let buttons = match options.content {
            SidePanelContent::Categories => AppletMenu::create_category_buttons(applet, options),
            SidePanelContent::Places => AppletMenu::create_place_buttons(options),
        };
        let column = cosmic::widget::column::with_children(buttons);
        // Spacers take the room left in the panel, which a scrollable has
        // no end to. Places are few enough to do without one; many
        // categories, e.g. custom ones, would otherwise get cut off.
        let panel: Element<'a, Message> = if options.content == SidePanelContent::Places
            && options.places.contains(&Place::Spacer)
        {
            column.height(Length::Fill).width(Length::Fill).into()
        } else {
            cosmic::widget::scrollable(column)
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
        };

        if options.tinted {
            container(panel)
                .padding(theme::active().cosmic().space_xxs())
                .width(Length::Fill)
                .height(Length::Fill)
                .class(cosmic::theme::Container::custom(tinted_pane_style))
                .into()
        } else {
            container(panel)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
    }

    pub fn create_logged_user_widget<'a>(
        applet: &'a Applet,
        options: &UserOptions,
    ) -> Element<'a, Message> {
        let Some(user) = &applet.current_user else {
            return cosmic::widget::Space::new().width(0).height(0).into();
        };
        let bold = |name: &'a str| {
            text(name)
                .font(Font {
                    weight: cosmic::iced::font::Weight::Bold,
                    ..Default::default()
                })
                .size(16)
        };

        let profile_picture_widget: Option<Element<Message>> = options.show_avatar.then(|| {
            if PathBuf::from(&user.profile_picture).exists() {
                cosmic::widget::image(&user.profile_picture)
                    .width(Length::Fixed(40.))
                    .height(Length::Fixed(40.))
                    .content_fit(ContentFit::ScaleDown)
                    .border_radius([5.; 4])
                    .into()
            } else {
                cosmic::widget::icon::from_svg_bytes(AppletMenu::USER_IDLE_SYMBOLIC)
                    .symbolic(true)
                    .icon()
                    .size(40)
                    .into()
            }
        });

        let nametag_widget: Element<Message> = match options.name {
            UserName::FullName if !user.user_realname.is_empty() => column![
                bold(&user.user_realname),
                text(&user.username).size(10),
            ]
            .into(),
            UserName::Username | UserName::FullName => bold(&user.username).into(),
        };

        row![]
            .push_maybe(profile_picture_widget)
            .push(nametag_widget)
            .spacing(5)
            .align_y(Alignment::Center)
            .padding([10., 0.])
            .into()
    }
}

/// Where the parts of the search results above the apps are, see
/// [`AppletMenu::search_header_layout`].
pub(crate) struct SearchHeaderLayout {
    /// Tops of the shown results of pop-launcher, in their order.
    pub result_tops: Vec<f32>,
    /// Height of the whole, including the apps' heading.
    pub height: f32,
}
