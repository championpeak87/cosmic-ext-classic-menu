use std::path::PathBuf;

use cosmic::cosmic_theme::Spacing;
use cosmic::iced::{
    Alignment, Length,
    widget::{column, row},
};
use cosmic::iced::{ContentFit, Font, Limits};
use cosmic::widget::text;
use cosmic::widget::{container, menu};
use cosmic::{Element, theme};

use crate::applet::{Applet, Message};
use crate::config::{HorizontalPosition, VerticalPosition};
use crate::fl;
use crate::model::appearance;
use crate::model::application_category::{ApplicationCategory, CategoryIcon};
use crate::model::application_entry::ApplicationEntry;
use crate::model::power_action::PowerAction;
use crate::model::system_tool::SystemTool;
use crate::widgets::VirtualizedAppList;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextMenuAction {
    LaunchApplication(usize),
    LaunchApplicationWithAction(usize, usize),
    PinToPanel(usize, bool),
    ToggleFavorite(usize),
    ChooseCategory(usize),
    ResetCategory(usize),
    HideApp(usize),
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
            ContextMenuAction::ChooseCategory(index) => Message::ChooseCategoryFor(*index),
            ContextMenuAction::ResetCategory(index) => Message::ResetApplicationCategory(*index),
            ContextMenuAction::HideApp(index) => Message::HideApplicationAt(*index),
        }
    }
}

/// State needed to build a single app's right-click menu.
pub struct AppMenuState<'a> {
    pub pinned_to_panel: bool,
    pub favorite: bool,
    /// Categories offered in "Move to...", in display order.
    pub move_targets: &'a [ApplicationCategory],
    /// Index into `move_targets` the app is currently moved to, if any.
    pub current_target: Option<usize>,
}

pub struct AppletMenu;

impl AppletMenu {
    pub const POPUP_MAX_WIDTH: f32 = 1200.0;
    pub const POPUP_MIN_WIDTH: f32 = 500.0;
    pub const POPUP_MAX_HEIGHT: f32 = 1200.0;
    pub const POPUP_MIN_HEIGHT: f32 = 400.0;

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
    const SYSTEM_HIBERNATE_SYMBOLIC_ICON: &[u8] =
        include_bytes!("../../res/icons/bundled/system-hibernate-symbolic.svg");
    const USER_IDLE_SYMBOLIC: &[u8] =
        include_bytes!("../../res/icons/bundled/user-idle-symbolic.svg");
    const SETTINGS_SYMBOLIC: &[u8] =
        include_bytes!("../../res/icons/bundled/preferences-system-symbolic.svg");

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
            .popup_container(
                menu_layout
                    .width(Length::Fixed(
                        appearance::clamp_popup_width(applet.config.popup_width) as f32,
                    ))
                    .height(Length::Fixed(
                        appearance::clamp_popup_height(applet.config.popup_height) as f32,
                    )),
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

    /// Right-click menu of an app entry: launch, pin to panel, favorite,
    /// move to another category, hide, then the app's own desktop actions.
    pub fn build_app_context_menu(
        app: &ApplicationEntry,
        app_index: usize,
        state: AppMenuState,
    ) -> Vec<menu::Tree<Message>> {
        let mut buttons: Vec<menu::Item<ContextMenuAction, _>> = vec![
            menu::Item::Button(
                fl!("launch"),
                None,
                ContextMenuAction::LaunchApplication(app_index),
            ),
            menu::Item::CheckBox(
                fl!("pin-to-panel"),
                None,
                state.pinned_to_panel,
                ContextMenuAction::PinToPanel(app_index, state.pinned_to_panel),
            ),
            menu::Item::CheckBox(
                fl!("add-to-favorites"),
                None,
                state.favorite,
                ContextMenuAction::ToggleFavorite(app_index),
            ),
        ];

        if !state.move_targets.is_empty() {
            // The category list is shown inside the main popup, not as a
            // submenu: a submenu popup makes libcosmic destroy popups out of
            // order, which crashes cosmic-comp.
            buttons.push(menu::Item::Divider);
            buttons.push(menu::Item::Button(
                fl!("move-to"),
                None,
                ContextMenuAction::ChooseCategory(app_index),
            ));
            buttons.push(if state.current_target.is_some() {
                menu::Item::Button(
                    fl!("restore-category"),
                    None,
                    ContextMenuAction::ResetCategory(app_index),
                )
            } else {
                menu::Item::ButtonDisabled(
                    fl!("restore-category"),
                    None,
                    ContextMenuAction::ResetCategory(app_index),
                )
            });
        }

        buttons.push(menu::Item::Button(
            fl!("hide-from-menu"),
            None,
            ContextMenuAction::HideApp(app_index),
        ));

        let actions: Vec<menu::Item<ContextMenuAction, _>> = app
            .desktop_actions
            .iter()
            .enumerate()
            .map(|(action_index, action)| {
                menu::Item::Button(
                    action.name.to_string(),
                    None,
                    ContextMenuAction::LaunchApplicationWithAction(app_index, action_index),
                )
            })
            .collect();
        if !actions.is_empty() {
            buttons.push(menu::Item::Divider);
            buttons.extend(actions);
        }

        menu::items(&std::collections::HashMap::new(), buttons)
    }

    fn category_icon(category: &ApplicationCategory) -> Element<'static, Message> {
        match &category.icon {
            CategoryIcon::Bundled(bytes) => cosmic::widget::icon::from_svg_bytes(*bytes)
                .symbolic(true)
                .icon()
                .into(),
            CategoryIcon::Named(name) => cosmic::widget::icon::from_name(name.as_str())
                .symbolic(true)
                .icon()
                .into(),
        }
    }

    fn power_action_icon(action: PowerAction) -> &'static [u8] {
        match action {
            PowerAction::Logout => AppletMenu::SYSTEM_LOGOUT_SYMBOLIC_ICON,
            PowerAction::Suspend => AppletMenu::SYSTEM_SUSPEND_SYMBOLIC_ICON,
            PowerAction::Hibernate => AppletMenu::SYSTEM_HIBERNATE_SYMBOLIC_ICON,
            PowerAction::Lock => AppletMenu::SYSTEM_LOCKSCREEN_SYMBOLIC_ICON,
            PowerAction::Reboot => AppletMenu::SYSTEM_REBOOT_SYMBOLIC_ICON,
            PowerAction::Shutdown => AppletMenu::SYSTEM_SHUTDOWN_SYMBOLIC_ICON,
        }
    }

    fn create_power_menu(applet: &Applet) -> Element<'_, Message> {
        let Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

        let content: Element<Message> = if applet.pending_confirmation.is_some() {
            column![
                text(fl!("hibernate-confirm-question")),
                row![
                    cosmic::widget::button::standard(fl!("cancel"))
                        .on_press(Message::CancelPowerAction),
                    cosmic::widget::button::suggested(fl!("confirm-yes"))
                        .on_press(Message::ConfirmPowerAction),
                ]
                .spacing(space_xxs)
                .align_y(Alignment::Center),
            ]
            .spacing(space_xxs)
            .align_x(Alignment::Center)
            .into()
        } else {
            let actions = PowerAction::visible(&applet.config.power_buttons, applet.can_hibernate);
            if actions.is_empty() {
                return cosmic::widget::Space::new().width(0).height(0).into();
            }
            cosmic::widget::row::with_children(
                actions
                    .into_iter()
                    .map(|action| {
                        cosmic::widget::button::icon(
                            cosmic::widget::icon::from_svg_bytes(AppletMenu::power_action_icon(
                                action,
                            ))
                            .symbolic(true),
                        )
                        // fixed padding keeps six icons inside the pane in every density
                        .padding(8)
                        .on_press(Message::PowerOptionSelected(action))
                        .into()
                    })
                    .collect::<Vec<Element<Message>>>(),
            )
            .align_y(Alignment::Center)
            .into()
        };

        container(content)
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

    /// Replaces the categories pane while choosing where to move an app.
    fn create_move_to_pane(applet: &Applet, app_index: usize) -> Element<'_, Message> {
        let Spacing {
            space_xxs, space_m, ..
        } = cosmic::theme::active().cosmic().spacing;

        let current_target = applet.available_applications.get(app_index).and_then(|app| {
            crate::logic::categories::current_override(&app.id, &applet.config)
        });

        let mut items: Vec<Element<Message>> = vec![
            cosmic::widget::button::custom(text(fl!("cancel")))
                .on_press(Message::CancelChooseCategory)
                .class(cosmic::theme::Button::AppletMenu)
                .width(Length::Fill)
                .into(),
            text(fl!("move-to")).into(),
        ];
        items.extend(applet.move_targets.iter().enumerate().map(
            |(target_index, category)| {
                cosmic::widget::button::custom(
                    row![
                        container(AppletMenu::category_icon(category)).padding([0, space_m]),
                        text(category.get_display_name()),
                    ]
                    .align_y(Alignment::Center),
                )
                .on_press(Message::MoveApplicationToCategory(app_index, target_index))
                .class(if current_target == Some(category.key.as_ref()) {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::AppletMenu
                })
                .width(Length::Fill)
                .into()
            },
        ));

        cosmic::widget::scrollable(
            cosmic::widget::column::with_children(items).spacing(space_xxs),
        )
        .height(Length::Fill)
        .width(Length::FillPortion(3))
        .into()
    }

    fn create_categories_pane(applet: &Applet) -> Element<'_, Message> {
        if let Some(app_index) = applet.moving_app {
            return AppletMenu::create_move_to_pane(applet, app_index);
        }
        let Spacing { space_m, .. } = cosmic::theme::active().cosmic().spacing;

        let mut categories_pane: Vec<Element<Message>> = applet
            .available_categories
            .iter()
            .map(|category| {
                cosmic::widget::button::custom(
                    row![
                        container(AppletMenu::category_icon(category)).padding([0, space_m]),
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
        let permanent_count = applet
            .available_categories
            .iter()
            .filter(|c| c.is_permanent())
            .count();
        if !categories_pane.is_empty() {
            categories_pane.insert(permanent_count, horizontal_divider);
        }

        // add power menu to the bottom of the categories pane
        categories_pane.push(
            cosmic::widget::Space::new()
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        );
        categories_pane.push(AppletMenu::create_power_menu(&applet));

        cosmic::widget::column::with_children(categories_pane)
            .height(Length::Fill)
            .width(Length::FillPortion(3))
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

            let user_button = cosmic::widget::button::custom(
                row![
                    profile_picture_widget,
                    cosmic::widget::Space::new().width(5).height(Length::Shrink),
                    nametag_widget
                ]
                .align_y(Alignment::Center),
            )
            .class(cosmic::theme::Button::Text)
            .on_press(Message::LaunchTool(SystemTool::USER_SETTINGS));

            let settings_button = cosmic::widget::tooltip(
                cosmic::widget::button::icon(
                    cosmic::widget::icon::from_svg_bytes(AppletMenu::SETTINGS_SYMBOLIC)
                        .symbolic(true),
                )
                .padding(8)
                .on_press(Message::LaunchTool(SystemTool::APPLET_SETTINGS)),
                text(fl!("settings")),
                cosmic::widget::tooltip::Position::Bottom,
            );

            row![
                user_button,
                cosmic::widget::Space::new().width(Length::Fill),
                settings_button
            ]
            .align_y(Alignment::Center)
            .padding([10., 0.])
            .into()
        } else {
            cosmic::widget::Space::new().width(0).height(0).into()
        }
    }
}
