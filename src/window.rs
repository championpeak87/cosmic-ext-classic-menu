use cosmic::app::Core;
use cosmic::cosmic_config::Config;
use cosmic::cosmic_theme::Spacing;
use cosmic::desktop::DesktopEntryData;
use cosmic::iced::ContentFit;
use cosmic::iced::{
    platform_specific::shell::commands::popup::{destroy_popup, get_popup},
    widget::{column, row},
    window::Id,
    Alignment, Length, Limits, Task,
};
use cosmic::iced_runtime::core::window;
use cosmic::widget::container;
use cosmic::widget::{scrollable, text};
use cosmic::{theme, Element};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::HashMap;
use std::sync::Arc;
use std::{env, process};

use crate::config::{
    load_config, load_or_default_config, update_config, AppListPosition, PowerOptionsPosition,
    RecentApplication, RecentApplicationConfig, SearchFieldPosition, APP_LIST_POSITION,
    POWER_OPTIONS_POSITION, RECENT_APPLICATIONS, SEARCH_FIELD_POSITION,
};
use crate::fl;
use crate::logic::{get_comment, load_apps, ApplicationCategory};
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
            power_menu_position: load_or_default_config::<PowerOptionsPosition>(
                config.clone(),
                POWER_OPTIONS_POSITION,
                CONFIG_VERS,
                PowerOptionsPosition::Top,
            ),
            app_menu_position: load_or_default_config::<AppListPosition>(
                config.clone(),
                APP_LIST_POSITION,
                CONFIG_VERS,
                AppListPosition::Left,
            ),
            search_field_position: load_or_default_config::<SearchFieldPosition>(
                config.clone(),
                SEARCH_FIELD_POSITION,
                CONFIG_VERS,
                SearchFieldPosition::Top,
            ),
            config,
        };

        (window, Task::none())
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::app::Message<Self::Message>> {
        match message {
            Message::TogglePopup(popup_type) => self.toggle_popup(popup_type),
            Message::PopupClosed(id) => self.close_popup(id),
            Message::SearchFieldInput(input) => self.update_search_field(&input),
            Message::PowerOptionSelected(action) => self.perform_power_action(action),
            Message::ApplicationSelected(app) => self.launch_application(app),
            Message::CategorySelected(category) => self.select_category(category),
            Message::LaunchTool(tool) => self.launch_tool(tool),
            Message::Zbus(result) => self.handle_zbus_result(result),
        }
    }

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

    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        match self.popup_type {
            PopupType::MainMenu => self.view_main_menu(),
            PopupType::ContextMenu => self.view_context_menu(),
        }
    }
}

impl Window {
    fn toggle_popup(&mut self, popup_type: PopupType) -> Task<cosmic::app::Message<Message>> {
        self.popup_type = popup_type;
        if let Some(p) = self.popup.take() {
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
        }
    }

    fn close_popup(&mut self, id: Id) -> Task<cosmic::app::Message<Message>> {
        self.search_field.clear();
        self.selected_category = Some(ApplicationCategory::All);
        self.available_applications = self.all_applications.clone();

        if self.popup.as_ref() == Some(&id) {
            self.popup = None;
        }

        Task::none()
    }

    fn update_search_field(&mut self, input: &str) -> Task<cosmic::app::Message<Message>> {
        self.selected_category = None;
        let matcher = SkimMatcherV2::default();

        if input.is_empty() {
            self.available_applications = self.all_applications.clone();
            self.selected_category = Some(ApplicationCategory::All);
        } else {
            self.available_applications = self
                .all_applications
                .iter()
                .filter(|app| matcher.fuzzy_match(&app.name, input).is_some())
                .cloned()
                .collect();
        }
        self.search_field = input.to_string();

        Task::none()
    }

    fn perform_power_action(&mut self, action: PowerAction) -> Task<cosmic::app::Message<Message>> {
        match action {
            PowerAction::Logout => {
                if let Err(_) = process::Command::new("cosmic-osd").arg("log-out").spawn() {
                    return action.perform();
                }
            }
            PowerAction::Reboot => {
                if let Err(_) = process::Command::new("cosmic-osd").arg("restart").spawn() {
                    return action.perform();
                }
            }
            PowerAction::Shutdown => {
                if let Err(_) = process::Command::new("cosmic-osd").arg("shutdown").spawn() {
                    return action.perform();
                }
            }
            _ => return action.perform(),
        };

        if let Some(p) = self.popup.take() {
            return destroy_popup(p);
        }

        Task::none()
    }

    fn launch_application(
        &mut self,
        app: Arc<DesktopEntryData>,
    ) -> Task<cosmic::app::Message<Message>> {
        let app_exec = app.exec.clone().unwrap();
        let env_vars: Vec<(String, String)> = env::vars().collect();
        let app_id = Some(app.id.clone());

        tokio::spawn(async move {
            cosmic::desktop::spawn_desktop_exec(app_exec, env_vars, app_id.as_deref()).await;
        });

        self.update_recent_applications(app);

        if let Some(p) = self.popup.take() {
            return destroy_popup(p);
        }
        Task::none()
    }

    fn update_recent_applications(&mut self, app: Arc<DesktopEntryData>) {
        let recent_app_key = String::from(RECENT_APPLICATIONS);
        let mut recent_applications: HashMap<String, RecentApplication> =
            match load_config::<RecentApplicationConfig>(&recent_app_key, CONFIG_VERS).0 {
                Some(x) => x
                    .recent_applications
                    .into_iter()
                    .map(|app| (app.app_id.clone(), app))
                    .collect(),
                None => HashMap::new(),
            };

        let recent_application = recent_applications
            .entry(app.id.clone())
            .or_insert_with(|| RecentApplication {
                app_id: app.id.clone(),
                launch_count: 0,
            });

        if recent_application.launch_count < u32::MAX {
            recent_application.launch_count += 1;
        }

        let recent_applications_values: Vec<RecentApplication> =
            recent_applications.values().cloned().collect();
        let recent_application_config = RecentApplicationConfig {
            recent_applications: recent_applications_values,
        };
        update_config(
            self.config.clone(),
            RECENT_APPLICATIONS,
            recent_application_config,
        );
    }

    fn select_category(
        &mut self,
        category: ApplicationCategory,
    ) -> Task<cosmic::app::Message<Message>> {
        self.search_field.clear();
        self.selected_category = Some(category.clone());

        if category == ApplicationCategory::All {
            self.available_applications = self.all_applications.clone();
        } else if category == ApplicationCategory::RecentlyUsed {
            self.available_applications = self.get_recent_applications();
        } else {
            self.available_applications = self
                .all_applications
                .iter()
                .filter(|app| {
                    app.categories
                        .contains(&category.get_mime_name().to_string())
                })
                .cloned()
                .collect();
        }

        Task::none()
    }

    fn get_recent_applications(&self) -> Vec<Arc<DesktopEntryData>> {
        let all_applications_entries: HashMap<String, &Arc<DesktopEntryData>> = self
            .all_applications
            .iter()
            .map(|app| (app.id.clone(), app))
            .collect();

        match load_config::<RecentApplicationConfig>(RECENT_APPLICATIONS, CONFIG_VERS).0 {
            Some(mut x) => {
                x.recent_applications
                    .sort_by(|a, b| b.launch_count.cmp(&a.launch_count));
                x.recent_applications
                    .iter()
                    .filter_map(|app| {
                        all_applications_entries
                            .get(&app.app_id)
                            .cloned()
                            .map(Arc::clone)
                    })
                    .collect()
            }
            None => Vec::new(),
        }
    }

    fn launch_tool(&mut self, tool: SystemTool) -> Task<cosmic::app::Message<Message>> {
        tool.perform();
        if let Some(p) = self.popup.take() {
            return destroy_popup(p);
        }
        Task::none()
    }

    fn handle_zbus_result(
        &self,
        result: Result<(), zbus::Error>,
    ) -> Task<cosmic::app::Message<Message>> {
        if let Err(e) = result {
            eprintln!("cosmic-classic-menu ERROR: '{}'", e);
        }

        Task::none()
    }

    fn view_main_menu(&self) -> Element<Message> {
        let Spacing {
            space_xxs, space_s, ..
        } = theme::active().cosmic().spacing;

        let power_menu = self.create_power_menu();
        let search_field = self.create_search_field();
        let app_list = self.create_app_list();
        let categories_pane = self.create_categories_pane();
        let vertical_spacer =
            cosmic::applet::padded_control(cosmic::widget::divider::vertical::default())
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .width(Length::Shrink)
                .padding(5);

        let dual_pane = match self.app_menu_position {
            AppListPosition::Left => {
                row![app_list, vertical_spacer, categories_pane].padding([space_xxs, 0])
            }
            AppListPosition::Right => {
                row![categories_pane, vertical_spacer, app_list].padding([space_xxs, 0])
            }
        };
        let menu_layout = match self.power_menu_position {
            PowerOptionsPosition::Top => match self.search_field_position {
                SearchFieldPosition::Top => {
                    column![search_field, dual_pane, power_menu].padding([space_xxs, space_s])
                }
                SearchFieldPosition::Bottom => {
                    column![dual_pane, search_field, power_menu].padding([space_xxs, space_s])
                }
            },
            PowerOptionsPosition::Bottom => match self.search_field_position {
                SearchFieldPosition::Top => {
                    column![search_field, dual_pane, power_menu].padding([space_xxs, space_s])
                }
                SearchFieldPosition::Bottom => {
                    column![dual_pane, search_field, power_menu].padding([space_xxs, space_s])
                }
            },
        };

        self.core
            .applet
            .popup_container(menu_layout)
            .max_height(POPUP_MAX_HEIGHT)
            .min_height(POPUP_MAX_HEIGHT)
            .into()
    }

    fn create_power_menu(&self) -> Element<Message> {
        let Spacing {
            space_xxs,
            space_s,
            space_l,
            ..
        } = theme::active().cosmic().spacing;

        container(
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
        .align_x(Alignment::End)
        .into()
    }

    fn create_search_field(&self) -> Element<Message> {
        let Spacing {
            space_xxs, space_s, ..
        } = theme::active().cosmic().spacing;

        cosmic::widget::search_input(fl!("search-placeholder"), &self.search_field)
            .on_input(Message::SearchFieldInput)
            .always_active()
            .padding([space_xxs, space_s])
            .into()
    }

    fn create_app_list(&self) -> Element<Message> {
        let Spacing {
            space_xl,

            space_xxl,
            space_s,
            ..
        } = theme::active().cosmic().spacing;

        let app_list: cosmic::widget::Column<Message> = self
            .available_applications
            .iter()
            .map(|app| {
                cosmic::widget::button::custom(
                    container(row![
                        app.icon
                            .as_cosmic_icon()
                            .width(Length::Fixed(space_xl.into()))
                            .height(Length::Fixed(space_xl.into()))
                            .content_fit(ContentFit::ScaleDown),
                        column![
                            text(&app.name),
                            text(get_comment(&app).unwrap_or_default()).size(8.0),
                        ]
                        .padding([0, space_s]),
                    ])
                    .align_y(Alignment::Center),
                )
                .on_press(Message::ApplicationSelected(app.clone()))
                .class(cosmic::theme::Button::MenuItem)
                .width(Length::Fill)
                .height(space_xxl)
                .into()
            })
            .collect();

        scrollable(app_list)
            .height(Length::Fill)
            .width(Length::FillPortion(2))
            .into()
    }

    fn create_categories_pane(&self) -> Element<Message> {
        let Spacing { space_m, .. } = cosmic::theme::active().cosmic().spacing;

        let categories: [ApplicationCategory; 13] = [
            ApplicationCategory::All,
            ApplicationCategory::RecentlyUsed,
            ApplicationCategory::Audio,
            ApplicationCategory::Video,
            ApplicationCategory::Development,
            ApplicationCategory::Games,
            ApplicationCategory::Graphics,
            ApplicationCategory::Network,
            ApplicationCategory::Office,
            ApplicationCategory::Science,
            ApplicationCategory::Settings,
            ApplicationCategory::System,
            ApplicationCategory::Utility,
        ];

        let mut categories_pane: Vec<Element<Message>> = categories
            .iter()
            .map(|category| {
                cosmic::widget::button::custom(
                    row![
                        container(cosmic::widget::icon::from_name(category.get_icon_name()))
                            .padding([0, space_m]),
                        text(category.get_display_name()),
                    ]
                    .align_y(Alignment::Center),
                )
                .on_press(Message::CategorySelected(category.clone()))
                .class(if self.selected_category == Some(category.clone()) {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::MenuItem
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

        cosmic::widget::column::with_children(categories_pane)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }

    fn view_context_menu(&self) -> Element<Message> {
        let context_menu = column![
            cosmic::applet::menu_button(
                row![cosmic::widget::text::body(fl!("settings-label")),].align_y(Alignment::Center)
            )
            .class(cosmic::theme::Button::AppletMenu)
            .on_press(Message::LaunchTool(SystemTool::SystemSettings)),
            cosmic::applet::menu_button(
                row![cosmic::widget::text::body(fl!("system-monitor-label")),]
                    .align_y(Alignment::Center)
            )
            .class(cosmic::theme::Button::AppletMenu)
            .on_press(Message::LaunchTool(SystemTool::SystemMonitor)),
            cosmic::applet::menu_button(
                row![cosmic::widget::text::body(fl!("disks-label")),].align_y(Alignment::Center)
            )
            .class(cosmic::theme::Button::AppletMenu)
            .on_press(Message::LaunchTool(SystemTool::DiskManagement))
        ];

        self.core
            .applet
            .popup_container(context_menu)
            .max_width(POPUP_MAX_WIDTH)
            .min_width(POPUP_MIN_WIDTH)
            .into()
    }
}
