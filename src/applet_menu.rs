use std::path::PathBuf;

use cosmic::cosmic_theme::Spacing;
use cosmic::desktop::IconSourceExt;
use cosmic::iced::{
    widget::{column, row},
    Alignment, Length,
};
use cosmic::iced::{ContentFit, Font, Limits};
use cosmic::widget::{container, ListColumn};
use cosmic::widget::{scrollable, text};
use cosmic::{theme, Element};

use crate::applet::{CosmicClassicMenu, Message, PowerAction};
use crate::config::{HorizontalPosition, VerticalPosition};
use crate::fl;
use crate::logic::apps::ApplicationCategory;

pub struct AppletMenu;

impl AppletMenu {
    const POPUP_MAX_WIDTH: f32 = 625.0;
    const POPUP_MIN_WIDTH: f32 = 625.0;
    const POPUP_MAX_HEIGHT: f32 = 700.0;
    const POPUP_MIN_HEIGHT: f32 = 700.0;

    pub fn view_main_menu_list(applet: &CosmicClassicMenu) -> Element<Message> {
        let Spacing {
            space_xxs, space_s, ..
        } = theme::active().cosmic().spacing;

        let current_user = AppletMenu::create_logged_user_widget(&applet);
        let search_field = AppletMenu::create_search_field(&applet);
        let app_list = AppletMenu::create_app_list(&applet);
        let categories_pane = AppletMenu::create_categories_pane(&applet);
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
            .popup_container(menu_layout)
            .limits(
                Limits::NONE
                    .max_height(AppletMenu::POPUP_MAX_HEIGHT)
                    .min_height(AppletMenu::POPUP_MIN_HEIGHT)
                    .max_width(AppletMenu::POPUP_MAX_WIDTH)
                    .min_width(AppletMenu::POPUP_MIN_WIDTH),
            )
            .into()
    }

    fn create_power_menu(_applet: &CosmicClassicMenu) -> Element<Message> {
        container(
            row![
                cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                    "system-log-out-symbolic"
                ))
                .on_press(Message::PowerOptionSelected(PowerAction::Logout)),
                cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                    "system-suspend-symbolic"
                ))
                .on_press(Message::PowerOptionSelected(PowerAction::Suspend)),
                cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                    "system-lock-screen-symbolic"
                ))
                .on_press(Message::PowerOptionSelected(PowerAction::Lock)),
                cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                    "system-restart-symbolic"
                ))
                .on_press(Message::PowerOptionSelected(PowerAction::Reboot)),
                cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                    "system-shutdown-symbolic"
                ))
                .on_press(Message::PowerOptionSelected(PowerAction::Shutdown)),
            ]
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([20, 0])
        .align_x(Alignment::Center)
        .into()
    }

    fn create_search_field(applet: &CosmicClassicMenu) -> Element<Message> {
        let Spacing {
            space_xxs, space_s, ..
        } = theme::active().cosmic().spacing;

        cosmic::widget::search_input(fl!("search-placeholder"), &applet.search_field)
            .on_input(Message::SearchFieldInput)
            .always_active()
            .width(Length::Fill)
            .padding([space_xxs, space_s])
            .into()
    }

    fn create_app_list(applet: &CosmicClassicMenu) -> Element<Message> {
        let Spacing {
            space_l, space_xl, ..
        } = theme::active().cosmic().spacing;

        let app_list: ListColumn<Message> = applet.available_applications.iter().fold(
            cosmic::widget::list_column().padding([0., 0.]),
            |list, app| {
                let button = cosmic::widget::button::custom(
                    container(row![
                        app.icon
                            .as_cosmic_icon()
                            .width(Length::Fixed(space_l.into()))
                            .height(Length::Fixed(space_l.into()))
                            .content_fit(ContentFit::ScaleDown),
                        cosmic::widget::Space::new(5, Length::Fill),
                        column![
                            text(&app.name),
                            text(app.comment.as_deref().unwrap_or_default())
                                .size(8.0),
                        ]
                        .padding([0, 0]),
                    ])
                    .align_y(Alignment::Center),
                )
                .on_press(Message::ApplicationSelected(app.clone()))
                .class(cosmic::theme::Button::MenuItem)
                .width(Length::Fill)
                .height(space_xl);

                list.add(button)
            },
        );

        scrollable(app_list)
            .height(Length::Fill)
            .width(Length::FillPortion(5))
            .into()
    }

    fn create_categories_pane(applet: &CosmicClassicMenu) -> Element<Message> {
        let Spacing { space_m, .. } = cosmic::theme::active().cosmic().spacing;

        let categories: [ApplicationCategory; 13] = [
            ApplicationCategory::ALL,
            ApplicationCategory::RECENTLY_USED,
            ApplicationCategory::AUDIO,
            ApplicationCategory::VIDEO,
            ApplicationCategory::DEVELOPMENT,
            ApplicationCategory::GAMES,
            ApplicationCategory::GRAPHICS,
            ApplicationCategory::NETWORK,
            ApplicationCategory::OFFICE,
            ApplicationCategory::SCIENCE,
            ApplicationCategory::SETTINGS,
            ApplicationCategory::SYSTEM,
            ApplicationCategory::UTILITY,
        ];

        let mut categories_pane: Vec<Element<Message>> = categories
            .iter()
            .map(|category| {
                cosmic::widget::button::custom(
                    row![
                        container(cosmic::widget::icon::from_name(category.icon_name))
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
        categories_pane.insert(2, horizontal_divider);

        // add power menu to the bottom of the categories pane
        categories_pane.push(cosmic::widget::Space::new(Length::Fill, Length::Fill).into());
        categories_pane.push(AppletMenu::create_power_menu(&applet));

        cosmic::widget::column::with_children(categories_pane)
            .height(Length::Fill)
            .width(Length::FillPortion(3))
            .into()
    }

    pub fn create_logged_user_widget(applet: &CosmicClassicMenu) -> Element<Message> {
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
                    cosmic::widget::icon::from_name("user-idle-symbolic")
                        .size(40)
                        .symbolic(true)
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
