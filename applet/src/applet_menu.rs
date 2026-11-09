use std::path::PathBuf;
use std::sync::Arc;

use cosmic::cosmic_theme::Spacing;
use cosmic::iced::{
    Alignment, Length,
    widget::{column, row},
};
use cosmic::iced::core::text::{Ellipsize, EllipsizeHeightLimit, Wrapping};
use cosmic::iced::{Background, ContentFit, Font, Limits};
use cosmic::widget::text;
use cosmic::iced::clipboard::dnd::DndAction;
use cosmic::iced::widget::scrollable::{Direction, Scrollbar};
use cosmic::widget::{DndDestination, container, menu};
use cosmic::{Element, theme};

use crate::applet::{Applet, FavoritesDropTarget, FavoritesRegion, Message};
use crate::config::{
    AppView, FeaturedApplications, HorizontalPosition, MenuLayout, PowerMenuPosition, SidePanel,
    VerticalPosition,
};
use crate::fl;
use crate::model::application_category::{ApplicationCategory, CategoryIcon};
use crate::model::application_entry::ApplicationEntry;
use crate::model::dragged_app::DraggedApp;
use crate::model::place::Place;
use crate::model::power_action::PowerAction;
use crate::widgets::{VirtualizedAppGrid, VirtualizedAppList};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextMenuAction {
    LaunchApplication(usize),
    LaunchApplicationWithAction(usize, usize),
    PinToPanel(usize, bool),
    ToggleFavorite(usize),
}

impl menu::Action for ContextMenuAction {
    type Message = Message;
    fn message(&self) -> Self::Message {
        match self {
            ContextMenuAction::LaunchApplication(index) => Message::LaunchApplicationAt(*index),
            ContextMenuAction::LaunchApplicationWithAction(app_index, action_index) => {
                Message::LaunchApplicationWithActionAt(*app_index, *action_index)
            }
            ContextMenuAction::PinToPanel(index, favorites) => {
                Message::PinToAppTrayIndex(*index, *favorites)
            }
            ContextMenuAction::ToggleFavorite(index) => Message::ToggleFavoriteAt(*index),
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
    /// Centered row with vertical padding, at the bottom of a pane.
    Padded,
    /// Plain row, sharing a row with other widgets.
    Compact,
    /// Plain row on a colored banner.
    Banner,
    /// Column, in a sidebar.
    Vertical,
}

/// Colored bar at the top and bottom of the retro layout.
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

/// The menu on its transparent surface, solid as it is not blurred.
fn menu_surface_style(theme: &cosmic::Theme) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        text_color: Some(cosmic.background(false).on.into()),
        icon_color: Some(cosmic.background(false).on.into()),
        background: Some(Background::Color(cosmic.background(false).base.into())),
        border: cosmic::iced::Border {
            radius: cosmic.corner_radii.radius_m.into(),
            width: 1.0,
            color: cosmic.background(false).divider.into(),
        },
        ..Default::default()
    }
}

/// Background of the categories pane in the retro layout.
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

    const SYSTEM_LOCKSCREEN_SYMBOLIC_ICON: &[u8] =
        include_bytes!("../../res/icons/bundled/system-lock-screen-symbolic.svg");
    const SYSTEM_LOGOUT_SYMBOLIC_ICON: &[u8] =
        include_bytes!("../../res/icons/bundled/system-log-out-symbolic.svg");
    const SYSTEM_REBOOT_SYMBOLIC_ICON: &[u8] =
        include_bytes!("../../res/icons/bundled/system-reboot-symbolic.svg");
    const SYSTEM_SHUTDOWN_SYMBOLIC_ICON: &[u8] =
        include_bytes!("../../res/icons/bundled/system-shutdown-symbolic.svg");
    const SYSTEM_SUSPEND_SYMBOLIC_ICON: &[u8] =
        include_bytes!("../../res/icons/bundled/system-suspend-symbolic.svg");
    const USER_IDLE_SYMBOLIC: &[u8] =
        include_bytes!("../../res/icons/bundled/user-idle-symbolic.svg");

    pub fn view_main_menu_list(applet: &Applet) -> Element<'_, Message> {
        let Spacing { space_xs, .. } = theme::active().cosmic().spacing;

        let menu_layout = match applet.config.menu_layout {
            MenuLayout::Classic => AppletMenu::view_classic(applet),
            // The start page sets Whistler apart, see `create_app_view`.
            MenuLayout::Retro | MenuLayout::Whistler => AppletMenu::view_retro(applet),
            MenuLayout::Compact => AppletMenu::view_compact(applet),
            MenuLayout::Cinnamon => AppletMenu::view_cinnamon(applet),
            MenuLayout::Modern => AppletMenu::view_modern(applet),
        };

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

    /// User on top, app list and categories side by side.
    fn view_classic(applet: &Applet) -> Element<'_, Message> {
        let header = row![AppletMenu::create_logged_user_widget(applet)]
            .push(cosmic::widget::Space::new().width(Length::Fill))
            .push_maybe(AppletMenu::power_menu_at(
                applet,
                PowerMenuPosition::Header,
                PowerMenuStyle::Compact,
            ))
            .align_y(Alignment::Center);
        let footer = AppletMenu::power_menu_at(
            applet,
            PowerMenuPosition::Footer,
            PowerMenuStyle::Padded,
        );

        AppletMenu::with_search(
            applet,
            header.into(),
            AppletMenu::create_dual_pane(applet, false),
            footer,
        )
    }

    /// Colored header and footer bars around two panes.
    fn view_retro(applet: &Applet) -> Element<'_, Message> {
        let Spacing {
            space_xxs, space_s, ..
        } = theme::active().cosmic().spacing;

        let header = container(
            row![AppletMenu::create_logged_user_widget(applet)]
                .push(cosmic::widget::Space::new().width(Length::Fill))
                .push_maybe(AppletMenu::power_menu_at(
                    applet,
                    PowerMenuPosition::Header,
                    PowerMenuStyle::Banner,
                ))
                .align_y(Alignment::Center),
        )
        .padding([0, space_s])
        .width(Length::Fill)
        .class(cosmic::theme::Container::custom(banner_style));
        // Keeps the bars off the search field. The body keeps its own gap.
        let search_on_top = applet.config.search_field_position == VerticalPosition::Top;
        let header = container(header).padding(cosmic::iced::Padding {
            bottom: if search_on_top { f32::from(space_xxs) } else { 0. },
            ..Default::default()
        });

        // The footer bar is shown even without power controls, as in the
        // original.
        let footer = container(
            row![cosmic::widget::Space::new().width(Length::Fill)]
                .push_maybe(AppletMenu::power_menu_at(
                    applet,
                    PowerMenuPosition::Footer,
                    PowerMenuStyle::Banner,
                ))
                .align_y(Alignment::Center),
        )
        .padding([space_xxs, space_s])
        .width(Length::Fill)
        .class(cosmic::theme::Container::custom(banner_style));
        let footer = container(footer).padding(cosmic::iced::Padding {
            top: if search_on_top { 0. } else { f32::from(space_xxs) },
            ..Default::default()
        });

        AppletMenu::with_search(
            applet,
            header.into(),
            AppletMenu::create_dual_pane(applet, true),
            Some(footer.into()),
        )
    }

    /// User and search share the header, power controls in the footer
    fn view_compact(applet: &Applet) -> Element<'_, Message> {
        let Spacing { space_xs, .. } = theme::active().cosmic().spacing;
        let search_on_top = applet.config.search_field_position == VerticalPosition::Top;

        let header = row![AppletMenu::create_logged_user_widget(applet)]
            .push(if search_on_top {
                AppletMenu::create_search_field(applet)
            } else {
                cosmic::widget::Space::new().width(Length::Fill).into()
            })
            .push_maybe(AppletMenu::power_menu_at(
                applet,
                PowerMenuPosition::Header,
                PowerMenuStyle::Compact,
            ))
            .spacing(space_xs)
            .align_y(Alignment::Center);

        let footer_power = AppletMenu::power_menu_at(
            applet,
            PowerMenuPosition::Footer,
            PowerMenuStyle::Compact,
        );
        let footer: Option<Element<'_, Message>> = match (search_on_top, footer_power) {
            (true, None) => None,
            (true, Some(power)) => Some(
                row![cosmic::widget::Space::new().width(Length::Fill), power]
                    .align_y(Alignment::Center)
                    .into(),
            ),
            (false, power) => Some(
                row![AppletMenu::create_search_field(applet)]
                    .push_maybe(power)
                    .spacing(space_xs)
                    .align_y(Alignment::Center)
                    .into(),
            ),
        };

        column![header, AppletMenu::create_dual_pane(applet, false)]
            .push_maybe(footer)
            .spacing(space_xs)
            .into()
    }

    /// Sidebar with pinned apps and power controls next to the categories
    /// and app list (Cinnamon like).
    fn view_cinnamon(applet: &Applet) -> Element<'_, Message> {
        let Spacing { space_xxs, .. } = theme::active().cosmic().spacing;
        let bar_position = applet.config.favorites_bar_position;

        // Keep the sidebar useful until the user adds favorites.
        let favorites = if applet.favorite_applications.is_empty() {
            &applet.recent_applications[..applet.recent_applications.len().min(6)]
        } else {
            &applet.favorite_applications[..]
        };
        // Favorites can be dragged, e.g. into the dock.
        let favorite_items = favorites.iter().map(|app| {
            let button = cosmic::widget::tooltip(
                cosmic::widget::button::custom(VirtualizedAppList::create_icon_widget(
                    app,
                    AppletMenu::SIDEBAR_ICON_SIZE,
                ))
                    .on_press(Message::ApplicationSelected(app.clone()))
                    .class(cosmic::theme::Button::AppletMenu)
                    .padding(space_xxs),
                text(&app.name),
                // Towards the rest of the menu.
                match bar_position {
                    HorizontalPosition::Left => cosmic::widget::tooltip::Position::Right,
                    HorizontalPosition::Right => cosmic::widget::tooltip::Position::Left,
                },
            );
            let item = container(VirtualizedAppList::drag_source(applet, app, button))
                .width(Length::Fill)
                .align_x(Alignment::Center);
            AppletMenu::favorite_slot(
                applet,
                app,
                FavoritesRegion::Bar,
                FavoriteSlotAxis::Vertical,
                (Length::Fill, Length::Shrink),
                item,
            )
        });

        let favorite_items = cosmic::widget::column::with_children(favorite_items)
            .spacing(space_xxs)
            .align_x(Alignment::Center);

        // Recently used apps stand in for missing favorites and can't be
        // reordered, so the whole sidebar adds to the favorites. Otherwise
        // each favorite is a drop target and the space below them is the
        // end, as drop targets must not be nested.
        let drop_area: Element<'_, Message> = if applet.favorite_applications.is_empty() {
            AppletMenu::favorites_drop_target(
                container(favorite_items)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .class(cosmic::theme::Container::custom(AppletMenu::drop_target_style(
                        applet.favorites_drop_target == Some(FavoritesDropTarget::Add),
                    ))),
                FavoritesDropTarget::Add,
            )
        } else {
            let end = container(cosmic::widget::Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .class(cosmic::theme::Container::custom(AppletMenu::drop_target_style(
                    applet.favorites_drop_target == Some(FavoritesDropTarget::End),
                )));
            // Many favorites would otherwise get squeezed. The bar is hidden
            // as it would cover the icons in such a narrow column.
            let favorite_items = cosmic::widget::scrollable(favorite_items)
                .width(Length::Fill)
                .direction(Direction::Vertical(
                    Scrollbar::new().width(0).scroller_width(0).margin(0),
                ));
            column![
                favorite_items,
                AppletMenu::favorites_drop_target(end, FavoritesDropTarget::End),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        };

        // A fixed width, as the drop target fills it and would otherwise
        // make the sidebar take a share of the panes' width.
        let sidebar = column![drop_area]
            .push(AppletMenu::create_power_menu(applet, PowerMenuStyle::Vertical))
            .spacing(space_xxs)
            .align_x(Alignment::Center)
            .width(Length::Fixed(f32::from(
                AppletMenu::SIDEBAR_ICON_SIZE + 2 * space_xxs,
            )))
            .height(Length::Fill);

        let panes = AppletMenu::create_dual_pane(applet, false);
        let body = match bar_position {
            HorizontalPosition::Left => row![sidebar, AppletMenu::vertical_divider(), panes],
            HorizontalPosition::Right => row![panes, AppletMenu::vertical_divider(), sidebar],
        };

        AppletMenu::with_search(
            applet,
            AppletMenu::create_logged_user_widget(applet),
            body.into(),
            None,
        )
    }

    /// App grid with recently used or favorite apps below and user and power
    /// controls in the footer.
    fn view_modern(applet: &Applet) -> Element<'_, Message> {
        let Spacing {
            space_xxs, space_xs, ..
        } = theme::active().cosmic().spacing;

        let mut body = column![
            text::heading(fl!("all-applications")),
            AppletMenu::create_app_view(applet),
        ]
        .spacing(space_xxs);

        // Recommendations would only get in the way of search results.
        if applet.search_field.is_empty() {
            body = match applet.config.effective_featured_applications() {
                FeaturedApplications::RecentlyUsed if !applet.recent_applications.is_empty() => {
                    body.push(cosmic::widget::divider::horizontal::default())
                        .push(text::heading(fl!("recently-used")))
                        .push(AppletMenu::featured_row(applet, &applet.recent_applications, false))
                }
                FeaturedApplications::RecentlyUsed => body,
                FeaturedApplications::Favorites => {
                    let favorites: Element<'_, Message> =
                        if applet.favorite_applications.is_empty() {
                            text::caption(fl!("favorites-empty")).into()
                        } else {
                            AppletMenu::featured_row(applet, &applet.favorite_applications, true)
                        };
                    let favorites = container(favorites)
                        .width(Length::Fill)
                        .padding(2)
                        .class(cosmic::theme::Container::custom(
                            AppletMenu::drop_target_style(
                                applet.favorites_drop_target == Some(FavoritesDropTarget::Add),
                            ),
                        ));
                    // Filled, each favorite is a drop target of its own.
                    let favorites: Element<'_, Message> =
                        if applet.favorite_applications.is_empty() {
                            AppletMenu::favorites_drop_target(favorites, FavoritesDropTarget::Add)
                        } else {
                            favorites.into()
                        };

                    body.push(cosmic::widget::divider::horizontal::default())
                        .push(text::heading(fl!("favorites")))
                        .push(favorites)
                }
            };
        }

        let header: Option<Element<'_, Message>> = AppletMenu::power_menu_at(
            applet,
            PowerMenuPosition::Header,
            PowerMenuStyle::Compact,
        )
        .map(|power| {
            row![cosmic::widget::Space::new().width(Length::Fill), power]
                .align_y(Alignment::Center)
                .into()
        });

        let footer = row![
            AppletMenu::create_logged_user_widget(applet),
            cosmic::widget::Space::new().width(Length::Fill),
        ]
        .push_maybe(AppletMenu::power_menu_at(
            applet,
            PowerMenuPosition::Footer,
            PowerMenuStyle::Compact,
        ))
        .align_y(Alignment::Center);

        let search_field = AppletMenu::create_search_field(applet);
        let (search_top, search_bottom) = match applet.config.search_field_position {
            VerticalPosition::Top => (Some(search_field), None),
            VerticalPosition::Bottom => (None, Some(search_field)),
        };

        column![]
            .push_maybe(header)
            .push_maybe(search_top)
            .push(body.height(Length::Fill))
            .push_maybe(search_bottom)
            .push(cosmic::widget::divider::horizontal::default())
            .push(footer)
            .spacing(space_xs)
            .into()
    }

    /// Up to four apps in a row, below the app grid of the modern layout.
    /// A row of `apps`, which can be reordered by dragging if they are the
    /// `favorites`.
    fn featured_row<'a>(
        applet: &'a Applet,
        apps: &'a [Arc<ApplicationEntry>],
        favorites: bool,
    ) -> Element<'a, Message> {
        let Spacing {
            space_xxs, space_xs, ..
        } = theme::active().cosmic().spacing;

        let buttons = apps
            .iter()
            .take(4)
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
                .class(cosmic::theme::Button::AppletMenu);

                if favorites {
                    AppletMenu::favorite_slot(
                        applet,
                        app,
                        FavoritesRegion::Bar,
                        FavoriteSlotAxis::Horizontal,
                        (Length::FillPortion(1), Length::Shrink),
                        VirtualizedAppList::drag_source(applet, app, button.width(Length::Fill)),
                    )
                } else {
                    button.width(Length::FillPortion(1)).into()
                }
            })
            .collect::<Vec<Element<'_, Message>>>();

        row(buttons).spacing(space_xxs).into()
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

    /// Stacks `header`, `body` and `footer`, with the search field above or
    /// below the body as configured.
    fn with_search<'a>(
        applet: &'a Applet,
        header: Element<'a, Message>,
        body: Element<'a, Message>,
        footer: Option<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        let Spacing {
            space_xxs, space_xs, ..
        } = theme::active().cosmic().spacing;

        let search_field = AppletMenu::create_search_field(applet);
        let search_field = match AppletMenu::power_menu_at(
            applet,
            PowerMenuPosition::Search,
            PowerMenuStyle::Compact,
        ) {
            Some(power) => row![search_field, power]
                .spacing(space_xs)
                .align_y(Alignment::Center)
                .into(),
            None => search_field,
        };
        let body = container(body).padding([space_xxs, 0]).height(Length::Fill);
        let column = match applet.config.search_field_position {
            VerticalPosition::Top => column![header, search_field, body],
            VerticalPosition::Bottom => column![header, body, search_field],
        };

        column.push_maybe(footer).into()
    }

    /// App list and categories side by side. With `tinted`, the categories
    /// pane gets its own background.
    fn create_dual_pane(applet: &Applet, tinted: bool) -> Element<'_, Message> {
        let side_panel_portion = applet.config.pane_portions().1;
        let app_list = AppletMenu::create_app_list(applet);
        let categories_pane = AppletMenu::create_categories_pane(applet);
        let categories_pane: Element<'_, Message> = if tinted {
            container(categories_pane)
                .padding(theme::active().cosmic().space_xxs())
                .width(Length::FillPortion(side_panel_portion))
                .height(Length::Fill)
                .class(cosmic::theme::Container::custom(tinted_pane_style))
                .into()
        } else {
            categories_pane
        };

        match applet.config.app_menu_position {
            HorizontalPosition::Left => {
                row![app_list, AppletMenu::vertical_divider(), categories_pane].into()
            }
            HorizontalPosition::Right => {
                row![categories_pane, AppletMenu::vertical_divider(), app_list].into()
            }
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

    /// Power controls, if the configured position is `position`.
    fn power_menu_at(
        applet: &Applet,
        position: PowerMenuPosition,
        style: PowerMenuStyle,
    ) -> Option<Element<'_, Message>> {
        (applet.config.effective_power_menu_position() == Some(position))
            .then(|| AppletMenu::create_power_menu(applet, style))
    }

    fn create_power_menu(_applet: &Applet, style: PowerMenuStyle) -> Element<'_, Message> {
        let buttons = [
            (AppletMenu::SYSTEM_LOGOUT_SYMBOLIC_ICON, PowerAction::Logout),
            (AppletMenu::SYSTEM_SUSPEND_SYMBOLIC_ICON, PowerAction::Suspend),
            (AppletMenu::SYSTEM_LOCKSCREEN_SYMBOLIC_ICON, PowerAction::Lock),
            (AppletMenu::SYSTEM_REBOOT_SYMBOLIC_ICON, PowerAction::Reboot),
            (AppletMenu::SYSTEM_SHUTDOWN_SYMBOLIC_ICON, PowerAction::Shutdown),
        ]
        .into_iter()
        .map(|(icon, action)| {
            let button = cosmic::widget::button::icon(
                cosmic::widget::icon::from_svg_bytes(icon).symbolic(true),
            )
            .on_press(Message::PowerOptionSelected(action));
            if style == PowerMenuStyle::Banner {
                button.class(banner_button_class()).into()
            } else {
                button.into()
            }
        })
        .collect::<Vec<Element<'_, Message>>>();

        match style {
            PowerMenuStyle::Padded => container(row(buttons).align_y(Alignment::Center))
                .width(Length::Fill)
                .padding([20, 0])
                .align_x(Alignment::Center)
                .into(),
            PowerMenuStyle::Compact | PowerMenuStyle::Banner => {
                row(buttons).align_y(Alignment::Center).into()
            }
            PowerMenuStyle::Vertical => column(buttons).align_x(Alignment::Center).into(),
        }
    }

    fn create_search_field(applet: &Applet) -> Element<'_, Message> {
        let Spacing {
            space_xxs, space_s, ..
        } = theme::active().cosmic().spacing;

        cosmic::widget::search_input(fl!("search-placeholder"), &applet.search_field)
            .on_input(Message::SearchFieldInput)
            .on_clear(Message::SearchCleared)
            .width(Length::Fill)
            .always_active()
            .padding([space_xxs, space_s])
            .into()
    }

    /// The apps, in the configured view.
    fn create_app_view(applet: &Applet) -> Element<'_, Message> {
        if let Some(favorites) = applet.start_page_favorites {
            return AppletMenu::create_start_page(applet, favorites);
        }

        let view = match applet.config.effective_app_view() {
            AppView::Column => VirtualizedAppList::view(applet),
            AppView::Grid => VirtualizedAppGrid::view(applet),
        };
        // Search results are left by clearing the search. The button takes
        // the place of the one leading here, at the bottom.
        if applet.config.menu_layout.has_start_page() && applet.search_field.is_empty() {
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

    /// The favorites, then the recently used apps, which lead the
    /// `available_applications` on the start page, and a button to all apps.
    fn create_start_page(applet: &Applet, favorites: usize) -> Element<'_, Message> {
        let mut rows = applet
            .available_applications
            .iter()
            .enumerate()
            .map(|(index, app)| VirtualizedAppList::create_app_button(applet, index, app));
        let favorite_rows: Vec<Element<'_, Message>> = rows.by_ref().take(favorites).collect();
        let recent_rows: Vec<Element<'_, Message>> = rows.collect();

        // Apps dropped on the favorites are added to them.
        let favorites: Element<'_, Message> = if favorite_rows.is_empty() {
            container(text::caption(fl!("favorites-empty")))
                .padding(theme::active().cosmic().space_xs())
                .width(Length::Fill)
                .into()
        } else {
            column(favorite_rows).into()
        };
        // A tinted background sets the favorites apart from the recently
        // used apps.
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
        let favorites: Element<'_, Message> = if applet.favorite_applications.is_empty() {
            AppletMenu::favorites_drop_target(favorites, drop_target)
        } else {
            favorites.into()
        };
        let mut list = column![favorites];
        if !recent_rows.is_empty() {
            list = list
                .push(AppletMenu::horizontal_divider())
                .extend(recent_rows);
        }

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
        .class(cosmic::theme::Button::AppletMenu)
        // Lines up with the apps in the list.
        .padding([space_xxs, space_s])
        .width(Length::Fill)
    }

    fn create_app_list(applet: &Applet) -> Element<'_, Message> {
        let app_list = AppletMenu::create_app_view(applet);

        column![app_list]
            .push_maybe(AppletMenu::power_menu_at(
                applet,
                PowerMenuPosition::AppList,
                PowerMenuStyle::Padded,
            ))
            .height(Length::Fill)
            .width(Length::FillPortion(applet.config.pane_portions().0))
            .into()
    }

    fn create_place_buttons(applet: &Applet) -> Vec<Element<'_, Message>> {
        let Spacing { space_m, .. } = cosmic::theme::active().cosmic().spacing;

        let mut buttons = Vec::new();
        // Places that are not shown can leave separators at the ends or next
        // to each other, so they are only added in front of a place.
        let mut separator_pending = false;
        for place in applet.config.places.iter().filter(|place| place.is_available()) {
            if *place == Place::Separator {
                separator_pending = !buttons.is_empty();
                continue;
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
                .class(cosmic::theme::Button::AppletMenu)
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

    fn create_category_buttons(applet: &Applet) -> Vec<Element<'_, Message>> {
        let Spacing { space_m, .. } = cosmic::theme::active().cosmic().spacing;

        let mut categories_pane: Vec<Element<Message>> = applet
            .available_categories
            .iter()
            .map(|category| {
                // The recently used category may be swapped for favorites.
                let category = if *category == ApplicationCategory::RECENTLY_USED {
                    applet.config.effective_featured_applications().category()
                } else {
                    category.clone()
                };
                let is_favorites = category == ApplicationCategory::FAVORITES;
                // The start page is not one of the categories, whichever
                // was selected before it.
                let selected = applet.start_page_favorites.is_none()
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
                .class(if highlighted {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::AppletMenu
                })
                .width(Length::Fill)
                .on_press(Message::CategorySelected(category));

                if is_favorites {
                    AppletMenu::favorites_drop_target(button, FavoritesDropTarget::Add)
                } else {
                    button.into()
                }
            })
            .collect();

        if !categories_pane.is_empty() {
            categories_pane.insert(2, AppletMenu::horizontal_divider());
        }
        if applet.config.menu_layout.has_start_page() {
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
        .class(if applet.start_page_favorites.is_some() {
            cosmic::theme::Button::Suggested
        } else {
            cosmic::theme::Button::AppletMenu
        })
        .width(Length::Fill)
        .on_press(Message::ShowStartPage)
        .into()
    }

    /// The pane next to the app list, showing categories or places.
    fn create_categories_pane(applet: &Applet) -> Element<'_, Message> {
        let mut categories_pane = match applet.config.side_panel {
            SidePanel::Categories => AppletMenu::create_category_buttons(applet),
            SidePanel::Places => AppletMenu::create_place_buttons(applet),
        };

        // add power menu to the bottom of the categories pane
        if let Some(power_menu) = AppletMenu::power_menu_at(
            applet,
            PowerMenuPosition::Categories,
            PowerMenuStyle::Padded,
        ) {
            categories_pane.push(
                cosmic::widget::Space::new()
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into(),
            );
            categories_pane.push(power_menu);
        }

        cosmic::widget::column::with_children(categories_pane)
            .height(Length::Fill)
            .width(Length::FillPortion(applet.config.pane_portions().1))
            .into()
    }

    pub fn create_logged_user_widget(applet: &Applet) -> Element<'_, Message> {
        if applet.config.user_widget == crate::config::UserWidgetStyle::None {
            return cosmic::widget::Space::new().width(0).height(0).into();
        }

        if let Some(user) = &applet.current_user {
            let profile_picture_widget: Element<Message> =
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
                };

            let nametag_widget: Element<Message> = match &applet.config.user_widget {
                crate::config::UserWidgetStyle::UsernamePrefered => text(&user.username)
                    .font(Font {
                        weight: cosmic::iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .size(16)
                    .into(),
                crate::config::UserWidgetStyle::RealNamePrefered => {
                    if !&user.user_realname.is_empty() {
                        column![
                            text(&user.user_realname)
                                .font(Font {
                                    weight: cosmic::iced::font::Weight::Bold,
                                    ..Default::default()
                                })
                                .size(16),
                            text(&user.username).size(10),
                        ]
                        .into()
                    } else {
                        text(&user.username)
                            .font(Font {
                                weight: cosmic::iced::font::Weight::Bold,
                                ..Default::default()
                            })
                            .size(16)
                            .into()
                    }
                }
                crate::config::UserWidgetStyle::None => {
                    cosmic::widget::Space::new().width(0).height(0).into()
                }
            };

            row![
                profile_picture_widget,
                cosmic::widget::Space::new().width(5).height(Length::Shrink),
                nametag_widget
            ]
            .align_y(Alignment::Center)
            .padding([10., 0.])
            .into()
        } else {
            cosmic::widget::Space::new().width(0).height(0).into()
        }
    }
}
