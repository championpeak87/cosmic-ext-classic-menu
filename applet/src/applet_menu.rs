use std::path::PathBuf;

use cosmic::cosmic_theme::Spacing;
use cosmic::iced::{
    Alignment, Length,
    widget::{column, row},
};
use cosmic::iced::{ContentFit, Font, Limits};
use cosmic::widget::{container, menu};
use cosmic::widget::text;
use cosmic::{Element, theme};

use crate::applet::{Applet, Message};
use crate::config::{HorizontalPosition, VerticalPosition};
use crate::fl;
use crate::model::power_action::PowerAction;
use crate::widgets::VirtualizedAppList;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextMenuAction {
    LaunchApplication(usize),
    LaunchApplicationWithAction(usize, usize),
    PinToPanel(usize, bool),
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
        }
    }
}

pub struct AppletMenu;

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

        let current_user = AppletMenu::create_logged_user_widget(applet);
        let search_field = AppletMenu::create_search_field(applet);
        let app_list = AppletMenu::create_app_list(applet);
        let categories_pane = AppletMenu::create_categories_pane(applet);
        let vertical_spacer =
            cosmic::applet::padded_control(cosmic::widget::divider::vertical::default())
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .width(Length::Shrink)
                .padding(5);

        let dual_pane = match applet.config.app_menu_position {
            HorizontalPosition::Left => {
                row![app_list, vertical_spacer, categories_pane].padding([space_xxs, 0])
            }
            HorizontalPosition::Right => {
                row![categories_pane, vertical_spacer, app_list].padding([space_xxs, 0])
            }
        };
        let menu_layout = match applet.config.search_field_position {
            VerticalPosition::Top => {
                column![current_user, search_field, dual_pane].padding([space_xxs, space_s])
            }
            VerticalPosition::Bottom => {
                column![current_user, dual_pane, search_field].padding([space_xxs, space_s])
            }
        };

        applet
            .core
            .applet
            .popup_container(menu_layout.width(Length::Fixed(600.)).height(Length::Fill))
            .limits(
                Limits::NONE
                    .max_height(AppletMenu::POPUP_MAX_HEIGHT)
                    .min_height(AppletMenu::POPUP_MIN_HEIGHT)
                    .max_width(AppletMenu::POPUP_MAX_WIDTH)
                    .min_width(AppletMenu::POPUP_MIN_WIDTH),
            )
            .into()
    }

    fn create_power_menu(_applet: &Applet) -> Element<'_, Message> {
        container(
            row![
                cosmic::widget::button::icon(cosmic::widget::icon::from_svg_bytes(
                    AppletMenu::SYSTEM_LOGOUT_SYMBOLIC_ICON,
                ).symbolic(true))
                .on_press(Message::PowerOptionSelected(PowerAction::Logout)),
                cosmic::widget::button::icon(cosmic::widget::icon::from_svg_bytes(
                    AppletMenu::SYSTEM_SUSPEND_SYMBOLIC_ICON,
                ).symbolic(true))
                .on_press(Message::PowerOptionSelected(PowerAction::Suspend)),
                cosmic::widget::button::icon(cosmic::widget::icon::from_svg_bytes(
                    AppletMenu::SYSTEM_LOCKSCREEN_SYMBOLIC_ICON,
                ).symbolic(true))
                .on_press(Message::PowerOptionSelected(PowerAction::Lock)),
                cosmic::widget::button::icon(cosmic::widget::icon::from_svg_bytes(
                    AppletMenu::SYSTEM_REBOOT_SYMBOLIC_ICON,
                ).symbolic(true))
                .on_press(Message::PowerOptionSelected(PowerAction::Reboot)),
                cosmic::widget::button::icon(cosmic::widget::icon::from_svg_bytes(
                    AppletMenu::SYSTEM_SHUTDOWN_SYMBOLIC_ICON,
                ).symbolic(true))
                .on_press(Message::PowerOptionSelected(PowerAction::Shutdown)),
            ]
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([20, 0])
        .align_x(Alignment::Center)
        .into()
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
        VirtualizedAppList::view(applet)
    }

    fn create_categories_pane(applet: &Applet) -> Element<'_, Message> {
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
                .on_press(Message::CategorySelected(category.clone()))
                .class(if applet.selected_category == Some(category.clone()) {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::AppletMenu
                })
                .width(Length::Fill)
                .into()
            })
            .collect();

        let horizontal_divider =
            cosmic::applet::padded_control(cosmic::widget::divider::horizontal::default())
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .padding(5)
                .into();
        if !categories_pane.is_empty() {
            categories_pane.insert(2, horizontal_divider);
        }

        // add power menu to the bottom of the categories pane
        categories_pane.push(cosmic::widget::Space::new(Length::Fill, Length::Fill).into());
        categories_pane.push(AppletMenu::create_power_menu(&applet));

        cosmic::widget::column::with_children(categories_pane)
            .height(Length::Fill)
            .width(Length::FillPortion(3))
            .into()
    }

    pub fn create_logged_user_widget(applet: &Applet) -> Element<'_, Message> {
        if applet.config.user_widget == crate::config::UserWidgetStyle::None {
            return cosmic::widget::Space::new(0, 0).into();
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
                crate::config::UserWidgetStyle::None => cosmic::widget::Space::new(0, 0).into(),
            };

            row![
                profile_picture_widget,
                cosmic::widget::Space::new(5, Length::Shrink),
                nametag_widget
            ]
            .align_y(Alignment::Center)
            .padding([10., 0.])
            .into()
        } else {
            cosmic::widget::Space::new(0, 0).into()
        }
    }
}
