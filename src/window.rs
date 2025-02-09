use cosmic::app::Core;
use cosmic::cosmic_config::Config;
use cosmic::cosmic_theme::Spacing;
use cosmic::desktop::DesktopEntryData;
use cosmic::iced::{
    platform_specific::shell::commands::popup::{destroy_popup, get_popup},
    widget::{column, row},
    window::Id,
    Alignment, Length, Limits, Task,
};
use cosmic::iced_runtime::core::window;
use cosmic::widget::menu::menu_button;
use cosmic::widget::{container, Column};
use cosmic::widget::{scrollable, text};
use cosmic::{theme, Element};
use freedesktop_desktop_entry::DesktopEntry;
use std::borrow::Cow;
use std::fmt::Debug;
use std::sync::Arc;
use std::{env, process};

use crate::config::{
    load_config, update_config, AppListPosition, PowerOptionsPosition, SearchFieldPosition,
    APP_LIST_POSITION, POWER_OPTIONS_POSITION, SEARCH_FIELD_POSITION,
};
use crate::fl;
use crate::logic::{load_apps, ApplicationCategory};
use crate::power_options::{lock, log_out, restart, shutdown, suspend};

const ID: &str = "com.championpeak87.cosmic-classic-menu";
const CONFIG_VERS: u64 = 1;
const POPUP_MAX_WIDTH: f32 = 500.0;
const POPUP_MIN_WIDTH: f32 = 500.0;
const POPUP_MAX_HEIGHT: f32 = 650.0;
const POPUP_MIN_HEIGHT: f32 = 650.0;

/// Holds the applet's state
pub struct Window {
    core: Core,
    config: Config,
    popup: Option<Id>,
    search_field: String,
    available_applications: Vec<Arc<DesktopEntryData>>,
    all_applications: Vec<Arc<DesktopEntryData>>,
    popup_type: PopupType,
    selected_category: Option<ApplicationCategory>,
    power_menu_position: PowerOptionsPosition,
    app_menu_position: AppListPosition,
    search_field_position: SearchFieldPosition,
}

/// Messages to be sent to the Libcosmic Update function
#[derive(Clone, Debug)]
pub enum Message {
    TogglePopup(PopupType),
    PopupClosed(Id),
    SearchFieldInput(String),
    PowerOptionSelected(PowerAction),
    ApplicationSelected(Arc<DesktopEntryData>),
    CategorySelected(ApplicationCategory),
    LaunchTool(SystemTool),
    Zbus(Result<(), zbus::Error>),
}

#[derive(Clone, Debug)]
pub enum SystemTool {
    SystemSettings,
    SystemMonitor,
    DiskManagement,
}

impl SystemTool {
    fn perform(&self) {
        match self {
            SystemTool::SystemSettings => {
                if let Err(_) = std::process::Command::new("cosmic-settings").spawn() {
                    eprintln!("COSMIC Settings cannot be launched!");
                }
            }
            SystemTool::SystemMonitor => {
                if let Err(_) = std::process::Command::new("gnome-system-monitor").spawn() {
                    eprintln!("GNOME System Monitor cannot be launched!");
                }
            }
            SystemTool::DiskManagement => {
                if let Err(_) = std::process::Command::new("gnome-disks").spawn() {
                    eprintln!("GNOME Disks cannot be launched!");
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum PowerAction {
    Shutdown,
    Logout,
    Lock,
    Reboot,
    Suspend,
}

impl PowerAction {
    fn perform(self) -> cosmic::iced::Task<cosmic::app::Message<Message>> {
        let msg = |m| cosmic::app::message::app(Message::Zbus(m));
        match self {
            PowerAction::Lock => cosmic::iced::Task::perform(lock(), msg),
            PowerAction::Logout => cosmic::iced::Task::perform(log_out(), msg),
            PowerAction::Reboot => cosmic::iced::Task::perform(restart(), msg),
            PowerAction::Shutdown => cosmic::iced::Task::perform(shutdown(), msg),
            PowerAction::Suspend => cosmic::iced::Task::perform(suspend(), msg),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PopupType {
    MainMenu,
    ContextMenu,
}

impl cosmic::Application for Window {
    type Executor = cosmic::executor::multi::Executor;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Window, Task<cosmic::app::Message<Message>>) {
        // Set the start up state of the application using the above variables
        let all_apps = load_apps();
        let config = Config::new(ID, CONFIG_VERS).unwrap();

        let window = Window {
            core,
            popup: None,
            search_field: String::new(),
            available_applications: all_apps.clone(),
            all_applications: all_apps,
            popup_type: PopupType::MainMenu,
            selected_category: Some(ApplicationCategory::All),
            power_menu_position: match load_config::<PowerOptionsPosition>(
                POWER_OPTIONS_POSITION,
                CONFIG_VERS,
            )
            .0
            {
                Some(x) => x,
                None => {
                    // create default config
                    update_config(
                        config.clone(),
                        POWER_OPTIONS_POSITION,
                        PowerOptionsPosition::Top,
                    );

                    PowerOptionsPosition::Top
                }
            },
            app_menu_position: match load_config::<AppListPosition>(APP_LIST_POSITION, CONFIG_VERS)
                .0
            {
                Some(x) => x,
                None => {
                    // create default config
                    update_config(config.clone(), APP_LIST_POSITION, AppListPosition::Left);

                    AppListPosition::Left
                }
            },
            search_field_position: match load_config::<SearchFieldPosition>(
                SEARCH_FIELD_POSITION,
                CONFIG_VERS,
            )
            .0
            {
                Some(x) => x,
                None => {
                    // create default config
                    update_config(
                        config.clone(),
                        SEARCH_FIELD_POSITION,
                        SearchFieldPosition::Top,
                    );

                    SearchFieldPosition::Top
                }
            },
            config: config,
        };

        dbg!(&window.app_menu_position);

        // Return the state and no Task
        (window, Task::none())
    }

    // The function that is called when the applet is closed
    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    // Libcosmic's update function
    fn update(&mut self, message: Self::Message) -> Task<cosmic::app::Message<Self::Message>> {
        match message {
            Message::TogglePopup(popup_type) => {
                self.popup_type = popup_type;
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);

                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );

                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(POPUP_MAX_WIDTH)
                        .min_width(POPUP_MIN_WIDTH)
                        .min_height(POPUP_MIN_HEIGHT)
                        .max_height(POPUP_MAX_HEIGHT);

                    get_popup(popup_settings)
                };
            }
            Message::PopupClosed(id) => {
                // delete search field
                self.search_field = "".to_string();
                self.selected_category = Some(ApplicationCategory::All);
                self.available_applications = self.all_applications.clone();

                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::SearchFieldInput(input) => {
                self.selected_category = None;
                if input.is_empty() {
                    self.available_applications = self.all_applications.clone();
                    self.selected_category = Some(ApplicationCategory::All);
                } else {
                    self.available_applications = self
                        .all_applications
                        .iter()
                        .filter(|x| {
                            x.name
                                .to_lowercase()
                                .contains(input.to_lowercase().as_str())
                        })
                        .cloned()
                        .collect();
                }
                self.search_field = input;
            }
            Message::PowerOptionSelected(action) => {
                match action {
                    PowerAction::Logout => {
                        if let Err(_err) =
                            process::Command::new("cosmic-osd").arg("log-out").spawn()
                        {
                            return PowerAction::Logout.perform();
                        }
                    }
                    PowerAction::Reboot => {
                        if let Err(_err) =
                            process::Command::new("cosmic-osd").arg("restart").spawn()
                        {
                            return PowerAction::Reboot.perform();
                        }
                    }
                    PowerAction::Shutdown => {
                        if let Err(_err) =
                            process::Command::new("cosmic-osd").arg("shutdown").spawn()
                        {
                            return PowerAction::Shutdown.perform();
                        }
                    }
                    a => return a.perform(),
                };
            }
            Message::ApplicationSelected(app) => {
                let app_exec: String = app.exec.to_owned().unwrap();
                let env_vars: Vec<(String, String)> = env::vars().collect();
                let app_id: Option<String> = Some(app.id.clone());

                tokio::spawn(async move {
                    cosmic::desktop::spawn_desktop_exec(app_exec, env_vars, app_id.as_deref())
                        .await;
                });

                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                }
            }
            Message::CategorySelected(category) => {
                // delete search field
                self.search_field = "".to_string();
                self.selected_category = Some(category.clone());

                let category_mime_name: String = category.get_mime_name().to_string();

                if category == ApplicationCategory::All {
                    self.available_applications = self.all_applications.clone();
                } else if category == ApplicationCategory::RecentlyUsed {
                    // TODO: Implement recently used apps
                    self.available_applications = Vec::with_capacity(0);
                } else {
                    self.available_applications = self
                        .all_applications
                        .iter()
                        .filter(|&app| app.categories.contains(&category_mime_name))
                        .cloned()
                        .collect();
                }
            }
            Message::LaunchTool(tool) => {
                tool.perform();

                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                }
            }
            Message::Zbus(result) => {
                if let Err(e) = result {
                    eprintln!("cosmic-classic-menu ERROR: '{}'", e);
                }
            }
        }
        Task::none()
    }

    // Libcosmic's view function
    fn view(&self) -> Element<Self::Message> {
        self.core
            .applet
            .autosize_window(
                cosmic::widget::mouse_area(
                    cosmic::widget::button::custom(
                        row![
                            cosmic::widget::icon::from_name(
                                "com.championpeak87.cosmic-classic-menu"
                            ),
                            text(fl!("menu-label")),
                        ]
                        .align_y(Alignment::Center),
                    )
                    .on_press(Message::TogglePopup(PopupType::MainMenu))
                    .class(cosmic::theme::Button::AppletIcon),
                )
                .on_right_press(Message::TogglePopup(PopupType::ContextMenu)),
            )
            .into()
    }

    // Libcosmic's applet view_window function
    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        let Spacing {
            space_xxs,
            space_s,
            space_l,
            ..
        } = theme::active().cosmic().spacing;

        match self.popup_type {
            PopupType::MainMenu => {
                let power_menu = container(
                    row![
                        cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                            "system-log-out-symbolic"
                        ))
                        .on_press(Message::PowerOptionSelected(PowerAction::Logout))
                        .icon_size(space_l)
                        .height(space_l)
                        .width(space_l),
                        cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                            "system-suspend-symbolic"
                        ))
                        .on_press(Message::PowerOptionSelected(PowerAction::Suspend))
                        .icon_size(space_l)
                        .height(space_l)
                        .width(space_l),
                        cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                            "system-lock-screen-symbolic"
                        ))
                        .on_press(Message::PowerOptionSelected(PowerAction::Lock))
                        .icon_size(space_l)
                        .height(space_l)
                        .width(space_l),
                        cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                            "system-restart-symbolic"
                        ))
                        .on_press(Message::PowerOptionSelected(PowerAction::Reboot))
                        .icon_size(space_l)
                        .height(space_l)
                        .width(space_l),
                        cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                            "system-shutdown-symbolic"
                        ))
                        .on_press(Message::PowerOptionSelected(PowerAction::Shutdown))
                        .icon_size(space_l)
                        .height(space_l)
                        .width(space_l),
                    ]
                    .padding([space_xxs, space_s])
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([space_xxs, 0])
                .align_x(Alignment::End);

                let search_field =
                    cosmic::widget::search_input(fl!("search-placeholder"), &self.search_field)
                        .on_input(Message::SearchFieldInput)
                        .always_active()
                        .padding([space_xxs, space_s]);

                let app_list: cosmic::widget::Column<Message> = self
                    .available_applications
                    .iter()
                    .fold(cosmic::widget::column(), |col, app| {
                        let comment = match &app.path {
                            Some(path) => {
                                let locale = current_locale::current_locale().ok();
                                let desktop_entry =
                                    DesktopEntry::from_path(path, Some(locale.as_slice()));

                                if let Ok(entry) = desktop_entry {
                                    match entry.comment(locale.as_slice()).unwrap_or(Cow::from(""))
                                    {
                                        Cow::Borrowed(x) => x.to_string(),
                                        Cow::Owned(x) => x,
                                    }
                                } else {
                                    "".to_string()
                                }
                            }
                            None => "".to_string(),
                        };
                        col.push(
                            cosmic::widget::button::custom(container(
                                row![
                                    app.icon
                                        .as_cosmic_icon()
                                        .width(Length::Fixed(space_l.into()))
                                        .height(Length::Fixed(space_l.into())),
                                    column![text(app.name.clone()), text(comment).size(8.0)]
                                        .padding([space_xxs, space_s])
                                ]
                                .align_y(Alignment::Center),
                            ))
                            .width(Length::Fill)
                            .on_press(Message::ApplicationSelected(app.clone())),
                        )
                        .width(Length::Fill)
                    });

                let filtered_categories: Vec<ApplicationCategory> =
                    ApplicationCategory::into_iter()
                        .filter(|&x| {
                            self.all_applications
                                .iter()
                                .filter(|&app| {
                                    let category_mime_name: String = x.get_mime_name().to_string();
                                    app.categories.contains(&category_mime_name)
                                })
                                .count()
                                > 0
                        })
                        .collect();

                let categories_list = filtered_categories.into_iter().fold(
                    cosmic::widget::column::with_capacity(ApplicationCategory::into_iter().len()),
                    |col, category| {
                        col.push(
                            cosmic::widget::button::custom(
                                row![
                                    cosmic::applet::padded_control(
                                        cosmic::widget::icon::from_name(category.get_icon_name())
                                            .size(space_s)
                                            .symbolic(true)
                                    )
                                    .padding([0, space_xxs]),
                                    cosmic::widget::text(category.get_name())
                                ]
                                .align_y(Alignment::Center),
                            )
                            .class(
                                if let Some(selected_category_unwrapped) = self.selected_category {
                                    if selected_category_unwrapped == category {
                                        cosmic::theme::Button::Suggested
                                    } else {
                                        cosmic::theme::Button::default()
                                    }
                                } else {
                                    cosmic::theme::Button::default()
                                },
                            )
                            .on_press(Message::CategorySelected(category))
                            .width(Length::Fill),
                        )
                        .width(Length::Fill)
                    },
                );
                let categories_pane = column![
                    cosmic::widget::button::custom(
                        row![
                            cosmic::applet::padded_control(
                                cosmic::widget::icon::from_name(
                                    ApplicationCategory::All.get_icon_name()
                                )
                                .symbolic(true)
                                .size(space_s)
                            )
                            .padding([0, space_xxs]),
                            cosmic::widget::text(ApplicationCategory::All.get_name())
                        ]
                        .align_y(Alignment::Center)
                    )
                    .class(
                        if self.selected_category == Some(ApplicationCategory::All) {
                            cosmic::theme::Button::Suggested
                        } else {
                            cosmic::theme::Button::default()
                        }
                    )
                    .on_press(Message::CategorySelected(ApplicationCategory::All))
                    .width(Length::Fill),
                    cosmic::widget::button::custom(
                        row![
                            cosmic::applet::padded_control(
                                cosmic::widget::icon::from_name(
                                    ApplicationCategory::RecentlyUsed.get_icon_name()
                                )
                                .symbolic(true)
                                .size(space_s)
                            )
                            .padding([0, space_xxs]),
                            cosmic::widget::text(ApplicationCategory::RecentlyUsed.get_name())
                        ]
                        .align_y(Alignment::Center)
                    )
                    .class(
                        if self.selected_category == Some(ApplicationCategory::RecentlyUsed) {
                            cosmic::theme::Button::Suggested
                        } else {
                            cosmic::theme::Button::default()
                        }
                    )
                    .on_press(Message::CategorySelected(ApplicationCategory::RecentlyUsed))
                    .width(Length::Fill),
                    cosmic::applet::padded_control(cosmic::widget::divider::horizontal::default())
                        .padding(space_xxs),
                    categories_list
                ];

                let app_container = if self.available_applications.len() > 0 {
                    cosmic::applet::padded_control(scrollable(app_list))
                        .width(Length::FillPortion(2))
                        .height(Length::Fill)
                        .padding([0, 0, space_xxs, 0])
                } else {
                    container(
                        column![
                            cosmic::widget::icon::from_name("emblem-important-symbolic")
                                .size(space_l),
                            cosmic::widget::text(fl!("no-apps"))
                        ]
                        .align_x(Alignment::Center),
                    )
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .width(Length::FillPortion(2))
                    .height(Length::Fill)
                    .padding([0, 0, space_xxs, 0])
                };
                let categories_container =
                    cosmic::applet::padded_control(scrollable(categories_pane))
                        .width(Length::FillPortion(1))
                        .padding([0, 0, 0, space_xxs]);

                let mut dual_pane_vec: [Element<Message>; 2] = [text("").into(), text("").into()];
                dual_pane_vec[match self.app_menu_position {
                    AppListPosition::Left => 0,
                    AppListPosition::Right => 1,
                }] = app_container.into();
                dual_pane_vec[match self.app_menu_position {
                    AppListPosition::Left => 1,
                    AppListPosition::Right => 0,
                }] = categories_container.into();

                let mut menu_layout_vec: [Element<Message>; 4] = [text("").into(), text("").into(), text("").into(), text("").into()];
                menu_layout_vec[match self.power_menu_position {
                    PowerOptionsPosition::Top => 0,
                    PowerOptionsPosition::Bottom => 3,
                }] = power_menu.into();
                menu_layout_vec[match self.search_field_position {
                    SearchFieldPosition::Top => 1,
                    SearchFieldPosition::Bottom => 2,
                }] = search_field.into();
                menu_layout_vec[match self.search_field_position {
                    SearchFieldPosition::Top => 2,
                    SearchFieldPosition::Bottom => 1,
                }] = cosmic::applet::padded_control(cosmic::widget::divider::horizontal::default())
                    .padding([space_xxs, 0])
                    .width(Length::Fill)
                    .into();
                menu_layout_vec[match self.power_menu_position {
                    PowerOptionsPosition::Top => match self.search_field_position {
                        SearchFieldPosition::Top => 3,
                        SearchFieldPosition::Bottom => 2,
                    },
                    PowerOptionsPosition::Bottom => match self.search_field_position {
                        SearchFieldPosition::Top => 2,
                        SearchFieldPosition::Bottom => 0,
                    },
                }] = cosmic::widget::row::with_children(dual_pane_vec.into_iter().collect()).into();

                let menu_layout = cosmic::widget::column::with_children(menu_layout_vec.into_iter().collect())
                    .padding([space_xxs, space_s]);

                return self
                    .core
                    .applet
                    .popup_container(menu_layout)
                    .max_height(POPUP_MAX_HEIGHT)
                    .min_height(POPUP_MAX_HEIGHT)
                    .into();
            }
            PopupType::ContextMenu => {
                let content = vec![
                    menu_button(vec![
                        row![cosmic::widget::text::body(fl!("system-config")),]
                            .align_y(Alignment::Center)
                            .into(),
                    ])
                    .class(cosmic::theme::Button::AppletMenu)
                    .on_press(Message::LaunchTool(SystemTool::SystemSettings))
                    .into(),
                    menu_button(vec![row![cosmic::widget::text::body(fl!(
                        "system-monitor"
                    )),]
                    .align_y(Alignment::Center)
                    .into()])
                    .class(cosmic::theme::Button::AppletMenu)
                    .on_press(Message::LaunchTool(SystemTool::SystemMonitor))
                    .into(),
                    menu_button(vec![row![cosmic::widget::text::body(fl!("disks")),]
                        .align_y(Alignment::Center)
                        .into()])
                    .class(cosmic::theme::Button::AppletMenu)
                    .on_press(Message::LaunchTool(SystemTool::DiskManagement))
                    .into(),
                ];

                return self
                    .core
                    .applet
                    .popup_container(Column::with_children(content))
                    .into();
            }
        }
    }
}
