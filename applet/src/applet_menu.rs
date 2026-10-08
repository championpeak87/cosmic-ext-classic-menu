use std::path::PathBuf;

use cosmic::cosmic_theme::Spacing;
use cosmic::iced::{
    Alignment, Length,
    widget::{column, row},
};
use cosmic::iced::core::text::{Ellipsize, EllipsizeHeightLimit, Wrapping};
use cosmic::iced::{Background, ContentFit, Font, Limits};
use cosmic::widget::text;
use cosmic::widget::{container, menu};
use cosmic::{Element, theme};

use crate::applet::{Applet, Message};
use crate::config::{HorizontalPosition, MenuLayout, PowerMenuPosition, SidePanel, VerticalPosition};
use crate::fl;
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
    pub const POPUP_MAX_WIDTH: f32 = 700.0;
    pub const POPUP_MIN_WIDTH: f32 = 500.0;
    pub const POPUP_MAX_HEIGHT: f32 = 700.0;
    pub const POPUP_MIN_HEIGHT: f32 = 300.0;

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
        let Spacing {
            space_xxs, space_s, ..
        } = theme::active().cosmic().spacing;

        let menu_layout = match applet.config.menu_layout {
            MenuLayout::Classic => AppletMenu::view_classic(applet),
            MenuLayout::Retro => AppletMenu::view_retro(applet),
            MenuLayout::Compact => AppletMenu::view_compact(applet),
            MenuLayout::Cinnamon => AppletMenu::view_cinnamon(applet),
            MenuLayout::Modern => AppletMenu::view_modern(applet),
        };

        applet
            .core
            .applet
            .popup_container(
                container(menu_layout)
                    .padding([space_xxs, space_s])
                    .width(Length::Fixed(600.))
                    .height(Length::Fixed(AppletMenu::POPUP_MAX_HEIGHT)),
            )
            .limits(
                Limits::NONE
                    .max_height(AppletMenu::POPUP_MAX_HEIGHT)
                    .min_height(AppletMenu::POPUP_MIN_HEIGHT)
                    .max_width(AppletMenu::POPUP_MAX_WIDTH)
                    .min_width(AppletMenu::POPUP_MIN_WIDTH),
            )
            .into()
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

        let favorites = applet.favorite_applications.iter().map(|app| -> Element<'_, Message> {
            cosmic::widget::tooltip(
                cosmic::widget::button::custom(VirtualizedAppList::create_icon_widget(app, 28))
                    .on_press(Message::ApplicationSelected(app.clone()))
                    .class(cosmic::theme::Button::AppletMenu)
                    .padding(space_xxs),
                text(&app.name),
                cosmic::widget::tooltip::Position::Right,
            )
            .into()
        });

        let sidebar = cosmic::widget::column::with_children(favorites)
            .push(cosmic::widget::Space::new().height(Length::Fill))
            .push(AppletMenu::create_power_menu(applet, PowerMenuStyle::Vertical))
            .spacing(space_xxs)
            .align_x(Alignment::Center)
            .height(Length::Fill);

        let body = row![
            sidebar,
            AppletMenu::vertical_divider(),
            AppletMenu::create_dual_pane(applet, false),
        ];

        AppletMenu::with_search(
            applet,
            AppletMenu::create_logged_user_widget(applet),
            body.into(),
            None,
        )
    }

    /// App grid with recently used apps below and user and power controls
    /// in the footer.
    fn view_modern(applet: &Applet) -> Element<'_, Message> {
        let Spacing {
            space_xxs, space_xs, ..
        } = theme::active().cosmic().spacing;

        let mut body = column![
            text::heading(fl!("all-applications")),
            VirtualizedAppGrid::view(applet),
        ]
        .spacing(space_xxs);

        // Recommendations would only get in the way of search results.
        if applet.search_field.is_empty() && !applet.recent_applications.is_empty() {
            let recent = applet
                .recent_applications
                .iter()
                .take(4)
                .map(|app| {
                    cosmic::widget::button::custom(
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
                    .class(cosmic::theme::Button::AppletMenu)
                    .width(Length::FillPortion(1))
                    .into()
                })
                .collect::<Vec<Element<'_, Message>>>();

            body = body
                .push(cosmic::widget::divider::horizontal::default())
                .push(text::heading(fl!("recently-used")))
                .push(row(recent).spacing(space_xxs));
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

    /// Stacks `header`, `body` and `footer`, with the search field above or
    /// below the body as configured.
    fn with_search<'a>(
        applet: &'a Applet,
        header: Element<'a, Message>,
        body: Element<'a, Message>,
        footer: Option<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        let Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

        let search_field = AppletMenu::create_search_field(applet);
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

    fn create_app_list(applet: &Applet) -> Element<'_, Message> {
        let app_list = VirtualizedAppList::view(applet);

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

    fn create_category_buttons(applet: &Applet) -> Vec<Element<'_, Message>> {
        let Spacing { space_m, .. } = cosmic::theme::active().cosmic().spacing;

        let mut categories_pane: Vec<Element<Message>> = applet
            .available_categories
            .iter()
            .map(|category| {
                cosmic::widget::button::custom(
                    row![
                        container(
                            cosmic::widget::icon::from_svg_bytes(category.icon_svg_bytes)
                                .symbolic(true)
                                .icon()
                        )
                        .padding([0, space_m]),
                        text(category.get_display_name()),
                    ]
                    .align_y(Alignment::Center),
                )
                .on_press(Message::CategorySelected(*category))
                .class(if applet.selected_category.as_ref() == Some(category) {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::AppletMenu
                })
                .width(Length::Fill)
                .into()
            })
            .collect();

        if !categories_pane.is_empty() {
            categories_pane.insert(2, AppletMenu::horizontal_divider());
        }

        categories_pane
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
