// SPDX-License-Identifier: GPL-3.0-only

use cached::Cached;
use cosmic::app::{Core, Task};
use cosmic::applet::cosmic_panel_config::PanelAnchor;
use cosmic::cctk::sctk::reexports::protocols::xdg::shell::client::xdg_positioner::{
    Anchor, Gravity,
};
use cosmic::cosmic_config::{Config, CosmicConfigEntry};
use cosmic::iced::event::listen_raw;
use cosmic::iced::keyboard::key::Named;
use cosmic::iced::{
    Alignment,
    platform_specific::shell::commands::popup::{destroy_popup, get_popup},
    widget::{column, row},
    window::Id,
};
use cosmic::iced::{Subscription, keyboard};
use cosmic::iced_widget::scrollable::{RelativeOffset, Viewport};
use cosmic::surface::Action;
use cosmic::{Application, Element};
use cosmic_app_list_config::AppListConfig;
use std::process;
use std::sync::Arc;

use crate::applet_button::AppletButton;
use crate::applet_menu::AppletMenu;
use crate::config::{AppletButtonStyle, AppletConfig, RecentApplication};
use crate::fl;
use crate::logic::apps::{Event, desktop_files, load_apps};
use crate::model::application_category::ApplicationCategory;
use crate::model::application_entry::{ApplicationEntry, DesktopAction};
use crate::model::popup_type::PopupType;
use crate::model::power_action::PowerAction;
use crate::model::system_tool::SystemTool;
use crate::model::user::User;

pub const APP_ID: &str = "com.championpeak87.cosmic-ext-classic-menu";

/// This is the struct that represents your application.
/// It is used to define the data that will be used by your application.
pub struct Applet {
    /// Application state which is managed by the COSMIC runtime.
    pub core: Core,
    /// The popup id.
    pub popup: Option<Id>,
    /// The configuration that is used to store the application settings.
    pub config: AppletConfig,
    /// The search field that is used to filter the applications.
    pub search_field: String,
    /// The list of available applications that are displayed in the menu.
    pub available_applications: Vec<Arc<ApplicationEntry>>,
    /// The list of available categories that are displayed in the menu.
    pub available_categories: Vec<ApplicationCategory>,
    /// The popup type that is used to determine which popup to display.
    pub popup_type: PopupType,
    /// The selected category that is used to filter the applications.
    pub selected_category: Option<ApplicationCategory>,
    /// Currently logged user
    pub current_user: Option<User>,
    /// Currently selected item
    pub selected_item_index: Option<usize>,
    /// Scrollable ID for keyboard navigation
    pub scrollable_id: cosmic::widget::Id,
    /// List of pinned apps
    pub app_list_config: AppListConfig,
    /// Cached context menus for applications (built once when apps are loaded)
    pub context_menus: std::collections::HashMap<String, Vec<cosmic::widget::menu::Tree<Message>>>,
    /// Scroll offset for virtualization (pixels from top)
    pub scroll_offset: f32,
}

/// This is the enum that contains all the possible variants that your application will need to transmit messages.
/// This is used to communicate between the different parts of your application.
/// If your application does not need to send messages, you can use an empty enum or `()`.
#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup(PopupType),
    PopupClosed(Id),
    SearchFieldInput(String),
    SearchCleared,
    PowerOptionSelected(PowerAction),
    ApplicationSelected(Arc<ApplicationEntry>),
    CategorySelected(ApplicationCategory),
    LaunchTool(SystemTool),
    Zbus(Result<(), zbus::Error>),
    UpdateLoggedUser(Result<User, zbus::Error>),
    FileEvent(Event),
    UpdateConfig(AppletConfig),
    UpdateAvailableApplications(Vec<Arc<ApplicationEntry>>),
    UpdateAvailableCategories(Vec<ApplicationCategory>),
    SelectPreviousApp,
    SelectNextApp,
    LaunchSelectedApplication,
    SuperKeyPressed,
    AppListConfigUpdated(AppListConfig),
    ContextMenuAction(Action),
    LaunchApplicationAt(usize),
    LaunchApplicationWithActionAt(usize, usize),
    PinToAppTrayIndex(usize, bool),
    ScrollUpdated(Viewport),
}

/// Implement the `Application` trait for your application.
/// This is where you define the behavior of your application.
///
/// The `Application` trait requires you to define the following types and constants:
/// - `Executor` is the async executor that will be used to run your application's commands.
/// - `Flags` is the data that your application needs to use before it starts.
/// - `Message` is the enum that contains all the possible variants that your application will need to transmit messages.
/// - `APP_ID` is the unique identifier of your application.
impl Application for Applet {
    type Executor = cosmic::executor::multi::Executor;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    /// This is the entry point of your application, it is where you initialize your application.
    ///
    /// Any work that needs to be done before the application starts should be done here.
    ///
    /// - `core` is used to passed on for you by libcosmic to use in the core of your own application.
    /// - `flags` is used to pass in any data that your application needs to use before it starts.
    /// - `Task` type is used to send messages to your application. `Task::none()` can be used to send no messages to your application.
    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let window = Applet {
            core,
            search_field: "".to_owned(),
            popup_type: PopupType::MainMenu,
            selected_category: Some(ApplicationCategory::ALL),
            config: AppletConfig::config(),
            current_user: None,
            selected_item_index: None,
            scrollable_id: cosmic::widget::Id::unique(),
            app_list_config: Default::default(),
            available_applications: Vec::new(),
            available_categories: Vec::new(),
            popup: None,
            context_menus: std::collections::HashMap::new(),
            scroll_offset: 0.0,
        };

        // fetch current user asynchronously
        let fetch_current_user_task =
            Task::perform(crate::model::user::get_current_user(), |result| {
                cosmic::Action::App(Message::UpdateLoggedUser(result))
            });

        let fetch_all_apps_task = Task::perform(
            tokio::task::spawn_blocking(|| crate::logic::apps::load_apps()),
            |res| cosmic::Action::App(Message::UpdateAvailableApplications(res.unwrap())),
        );

        let fetch_available_categories_task = Task::perform(
            tokio::task::spawn_blocking(|| crate::logic::apps::load_app_categories()),
            |res| cosmic::Action::App(Message::UpdateAvailableCategories(res.unwrap())),
        );

        (
            window,
            Task::batch(vec![
                fetch_current_user_task,
                fetch_all_apps_task,
                fetch_available_categories_task,
            ]),
        )
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    /// This is the main view of your application, it is the root of your widget tree.
    ///
    /// The `Element` type is used to represent the visual elements of your application,
    /// it has a `Message` associated with it, which dictates what type of message it can send.
    ///
    /// To get a better sense of which widgets are available, check out the `widget` module.
    fn view(&self) -> Element<'_, Message> {
        let applet_button_style = &self.config.applet_button_style;
        let panel_type = &self.core.applet.panel_type;
        let size = &self.core.applet.size;

        match applet_button_style {
            AppletButtonStyle::IconOnly => AppletButton::view_icon_only(&self),
            AppletButtonStyle::LabelOnly => AppletButton::view_label_only(&self),
            AppletButtonStyle::IconAndLabel => AppletButton::view_icon_and_label(&self),
            AppletButtonStyle::Auto => match panel_type {
                cosmic::applet::PanelType::Panel => match size {
                    cosmic::applet::Size::Hardcoded(hardcoded_size) => {
                        if hardcoded_size.0
                            < cosmic::applet::cosmic_panel_config::PanelSize::M
                                .get_applet_icon_size(false) as u16
                        {
                            AppletButton::view_label_only(&self)
                        } else {
                            AppletButton::view_icon_only(&self)
                        }
                    }
                    cosmic::applet::Size::PanelSize(panel_size) => match panel_size {
                        cosmic::applet::cosmic_panel_config::PanelSize::XS
                        | cosmic::applet::cosmic_panel_config::PanelSize::S => {
                            AppletButton::view_label_only(&self)
                        }
                        cosmic::applet::cosmic_panel_config::PanelSize::M
                        | cosmic::applet::cosmic_panel_config::PanelSize::L
                        | cosmic::applet::cosmic_panel_config::PanelSize::XL
                        | cosmic::applet::cosmic_panel_config::PanelSize::Custom(_) => {
                            AppletButton::view_icon_only(&self)
                        }
                    },
                },
                cosmic::applet::PanelType::Dock | cosmic::applet::PanelType::Other(_) => {
                    AppletButton::view_icon_only(&self)
                }
            },
        }
    }

    fn view_window(&self, _id: Id) -> Element<'_, Message> {
        match self.popup_type {
            PopupType::MainMenu => self.view_main_menu(),
            PopupType::ContextMenu => self.view_context_menu(),
        }
    }

    /// Application messages are handled here. The application state can be modified based on
    /// what message was received. Tasks may be returned for asynchronous execution on a
    /// background thread managed by the application's executor.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::TogglePopup(popup_type) => self.toggle_popup(popup_type),
            Message::PopupClosed(id) => self.close_popup(id),
            Message::SearchFieldInput(input) => self.update_search_field(input),
            Message::SearchCleared => self.clear_search(),
            Message::PowerOptionSelected(action) => self.perform_power_action(action),
            Message::ApplicationSelected(app) => self.launch_application(app, None),
            Message::CategorySelected(category) => self.select_category(category),
            Message::LaunchTool(tool) => self.launch_tool(tool),
            Message::Zbus(result) => self.handle_zbus_result(result),
            Message::UpdateLoggedUser(user) => {
                self.current_user = user.ok();
                Task::none()
            }
            Message::FileEvent(event) => self.handle_event(event),
            Message::UpdateConfig(config) => {
                self.config = config;

                Task::none()
            }
            Message::UpdateAvailableApplications(items) => {
                self.available_applications = items;

                // Build and cache context menus for each application once
                self.context_menus.clear();
                for (app_index, app) in self.available_applications.iter().enumerate() {
                    let is_app_in_favorites =
                        crate::logic::apps::is_app_in_favorites(app, &self.app_list_config);

                    let mut context_menu_buttons: Vec<
                        cosmic::widget::menu::Item<crate::applet_menu::ContextMenuAction, _>,
                    > = vec![
                        cosmic::widget::menu::Item::Button(
                            crate::fl!("launch"),
                            None,
                            crate::applet_menu::ContextMenuAction::LaunchApplication(app_index),
                        ),
                        cosmic::widget::menu::Item::CheckBox(
                            crate::fl!("pin-to-panel"),
                            None,
                            is_app_in_favorites,
                            crate::applet_menu::ContextMenuAction::PinToPanel(
                                app_index,
                                is_app_in_favorites,
                            ),
                        ),
                    ];

                    let additional_options_buttons: Vec<
                        cosmic::widget::menu::Item<crate::applet_menu::ContextMenuAction, _>,
                    > =
                        app.desktop_actions
                            .iter()
                            .enumerate()
                            .map(|(action_index, action)| {
                                cosmic::widget::menu::Item::Button(
                                action.name.to_string(),
                                None,
                                crate::applet_menu::ContextMenuAction::
                                    LaunchApplicationWithAction(app_index, action_index),
                            )
                            })
                            .collect();

                    if !additional_options_buttons.is_empty() {
                        context_menu_buttons.push(cosmic::widget::menu::Item::Divider);
                        context_menu_buttons.extend(additional_options_buttons);
                    }

                    let trees = cosmic::widget::menu::items(
                        &std::collections::HashMap::new(),
                        context_menu_buttons,
                    );
                    self.context_menus.insert(app.id.clone(), trees);
                }

                Task::none()
            }
            Message::UpdateAvailableCategories(items) => {
                self.available_categories = items;

                Task::none()
            }
            Message::SelectPreviousApp => self.select_previous_app(),
            Message::SelectNextApp => self.select_next_app(),
            Message::LaunchSelectedApplication => {
                dbg!(self.selected_item_index);
                if let Some(index) = self.selected_item_index {
                    let selected_application =
                        self.available_applications.get(index).unwrap().clone();

                    return self.launch_application(selected_application, None);
                }

                Task::none()
            }
            Message::SuperKeyPressed => self.toggle_popup(PopupType::MainMenu),
            Message::LaunchApplicationAt(index) => {
                if let Some(app) = self.available_applications.get(index).cloned() {
                    return self.launch_application(app, None);
                }

                Task::none()
            }
            Message::LaunchApplicationWithActionAt(app_index, action_index) => {
                if let Some(app) = self.available_applications.get(app_index).cloned() {
                    if let Some(action) = app.desktop_actions.get(action_index).cloned() {
                        return self.launch_application(app, Some(action));
                    }
                }

                Task::none()
            }
            Message::PinToAppTrayIndex(app_index, favorites) => {
                if let Some(app) = self.available_applications.get(app_index).cloned() {
                    let pinned_id = app.id.clone();
                    if let Some(app_list_helper) =
                        Config::new(cosmic_app_list_config::APP_ID, AppListConfig::VERSION).ok()
                    {
                        if favorites {
                            // currently favorites==true indicates it is pinned; request unpin
                            self.app_list_config
                                .remove_pinned(&pinned_id, &app_list_helper);
                        } else {
                            self.app_list_config.add_pinned(pinned_id, &app_list_helper);
                        }

                        // Rebuild the cached menu for this app to reflect the new pin state
                        let new_is_favorites = !favorites;
                        let mut context_menu_buttons: Vec<
                            cosmic::widget::menu::Item<crate::applet_menu::ContextMenuAction, _>,
                        > = vec![
                            cosmic::widget::menu::Item::Button(
                                crate::fl!("launch"),
                                None,
                                crate::applet_menu::ContextMenuAction::LaunchApplication(app_index),
                            ),
                            cosmic::widget::menu::Item::CheckBox(
                                crate::fl!("pin-to-panel"),
                                None,
                                new_is_favorites,
                                crate::applet_menu::ContextMenuAction::PinToPanel(
                                    app_index,
                                    new_is_favorites,
                                ),
                            ),
                        ];

                        let additional_options_buttons: Vec<
                            cosmic::widget::menu::Item<crate::applet_menu::ContextMenuAction, _>,
                        > = app
                            .desktop_actions
                            .iter()
                            .enumerate()
                            .map(|(action_index, action)| {
                                cosmic::widget::menu::Item::Button(
                                    action.name.to_string(),
                                    None,
                                    crate::applet_menu::ContextMenuAction::
                                        LaunchApplicationWithAction(app_index, action_index),
                                )
                            })
                            .collect();

                        if !additional_options_buttons.is_empty() {
                            context_menu_buttons.push(cosmic::widget::menu::Item::Divider);
                            context_menu_buttons.extend(additional_options_buttons);
                        }

                        let trees = cosmic::widget::menu::items(
                            &std::collections::HashMap::new(),
                            context_menu_buttons,
                        );
                        self.context_menus.insert(app.id.clone(), trees);
                    }
                }

                Task::none()
            }
            Message::AppListConfigUpdated(app_list_config) => {
                self.app_list_config = app_list_config;

                Task::none()
            }
            Message::ContextMenuAction(action) => {
                return cosmic::task::message(cosmic::Action::Cosmic(
                    cosmic::app::Action::Surface(action),
                ));
            }
            Message::ScrollUpdated(viewport) => {
                self.scroll_offset = viewport.absolute_offset().y;
                Task::none()
            }
        }
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They are started at the
    /// beginning of the application, and persist through its lifetime.
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            desktop_files(self.core.main_window_id()).map(Message::FileEvent),
            listen_raw(|event, _, _| {
                return match event {
                    cosmic::iced::Event::Keyboard(keyboard::Event::KeyPressed {
                        key: keyboard::Key::Named(key),
                        ..
                    }) => match key {
                        Named::ArrowUp => Some(Message::SelectPreviousApp),
                        Named::ArrowDown => Some(Message::SelectNextApp),
                        Named::Enter => Some(Message::LaunchSelectedApplication),

                        _ => None,
                    },

                    _ => None,
                };
            }),
            // Watch for application configuration changes.
            self.core
                .watch_config::<AppletConfig>(Self::APP_ID)
                .map(|update| Message::UpdateConfig(update.config)),
            // DBUS subscription
            crate::dbus::dbus_service_subscription().map(|msg| msg),
            self.core
                .watch_config::<cosmic_app_list_config::AppListConfig>(
                    cosmic_app_list_config::APP_ID,
                )
                .map(|config| Message::AppListConfigUpdated(config.config)),
        ])
    }
}

impl Applet {
    pub fn handle_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Changed => {
                // Invalidate the cache
                log::debug!("App list has been updated, invalidating cache and loading new list!");
                crate::logic::apps::APPS_CACHE.lock().unwrap().cache_reset();

                Task::none()
            }
        }
    }

    fn toggle_popup(&mut self, popup_type: PopupType) -> Task<Message> {
        // reset popup state
        self.search_field.clear();
        self.selected_category = Some(ApplicationCategory::ALL);
        self.available_applications = load_apps();
        self.selected_item_index = None;

        let mut tasks = vec![];
        self.popup_type = popup_type;
        if self.popup_type == PopupType::MainMenu {
            tasks.push(Task::perform(
                tokio::task::spawn_blocking(|| crate::logic::apps::load_apps()),
                |res| cosmic::action::app(Message::UpdateAvailableApplications(res.unwrap())),
            ));
            tasks.push(Task::perform(
                tokio::task::spawn_blocking(|| crate::logic::apps::load_app_categories()),
                |res| cosmic::action::app(Message::UpdateAvailableCategories(res.unwrap())),
            ));
        }

        if let Some(p) = self.popup.take() {
            tasks.push(destroy_popup(p));
            Task::batch(tasks)
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
            let (anchor, gravity) = match self.core.applet.anchor {
                PanelAnchor::Left => (Anchor::TopRight, Gravity::BottomRight),
                PanelAnchor::Right => (Anchor::TopLeft, Gravity::BottomLeft),
                PanelAnchor::Top => (Anchor::BottomLeft, Gravity::BottomRight),
                PanelAnchor::Bottom => (Anchor::TopLeft, Gravity::TopRight),
            };
            popup_settings.positioner.anchor = anchor;
            popup_settings.positioner.gravity = gravity;

            tasks.push(get_popup(popup_settings));
            Task::batch(tasks)
        }
    }

    fn close_popup(&mut self, id: Id) -> Task<Message> {
        if self.popup.as_ref() == Some(&id) {
            self.popup = None;
        }

        Task::none()
    }

    fn clear_search(&mut self) -> Task<Message> {
        self.selected_category = Some(ApplicationCategory::ALL);
        self.search_field = "".to_string();

        Task::perform(
            tokio::task::spawn_blocking(|| crate::logic::apps::load_apps()),
            |res| cosmic::action::app(Message::UpdateAvailableApplications(res.unwrap())),
        )
    }

    fn update_search_field(&mut self, input: String) -> Task<Message> {
        self.selected_category = None;
        self.selected_item_index = None;

        self.search_field = input.clone();
        if self.search_field.is_empty() {
            return self.clear_search();
        }

        Task::batch([
            // reset scroll position
            cosmic::iced_runtime::task::widget(
                cosmic::iced_core::widget::operation::scrollable::snap_to(
                    self.scrollable_id.clone(),
                    RelativeOffset { x: 0., y: 0. },
                ),
            ),
            Task::perform(
                tokio::task::spawn_blocking(move || crate::logic::apps::load_filtered_apps(input)),
                |res| cosmic::Action::App(Message::UpdateAvailableApplications(res.unwrap())),
            ),
        ])
    }

    fn perform_power_action(&mut self, action: PowerAction) -> Task<Message> {
        let is_flatpak = std::env::var("FLATPAK_ID").is_ok();

        if action == PowerAction::Lock || action == PowerAction::Suspend {
            return action.perform();
        }

        let app_exec = match action {
            PowerAction::Logout => "cosmic-osd log-out",
            PowerAction::Reboot => "cosmic-osd restart",
            PowerAction::Shutdown => "cosmic-osd shutdown",
            _ => "",
        };
        let (main_exec, args) = if is_flatpak {
            (
                "flatpak-spawn",
                vec!["--host", "/bin/sh", "-l", "-c", app_exec],
            )
        } else {
            let mut parts = app_exec.split_whitespace();
            let exec = parts.next().unwrap_or("");
            let args: Vec<&str> = parts.collect();

            (exec, args)
        };

        // non sandboxed env
        if let Err(_) = process::Command::new(main_exec).args(args).spawn() {
            return action.perform();
        }

        if let Some(p) = self.popup.take() {
            return destroy_popup(p);
        }

        Task::none()
    }

    fn launch_application(
        &mut self,
        app: Arc<ApplicationEntry>,
        action: Option<DesktopAction>,
    ) -> Task<Message> {
        let mut app_exec = if action.is_some() {
            action
                .unwrap()
                .exec
                .clone()
                .split_whitespace()
                .filter(|arg| !arg.starts_with('%'))
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            app.exec
                .clone()
                .unwrap()
                .split_whitespace()
                .filter(|arg| !arg.starts_with('%'))
                .collect::<Vec<_>>()
                .join(" ")
        };
        let env_vars: Vec<(String, String)> = std::env::vars().collect();
        let app_id = Some(app.id.clone());
        let mut is_terminal = app.is_terminal;

        let is_flatpak = std::env::var("FLATPAK_ID").is_ok();

        if is_flatpak {
            if is_terminal {
                // For flatpaks handle terminal applications manually
                // not through libcosmic implementation
                let term = cosmic_settings_config::shortcuts::context()
                    .ok()
                    .and_then(|config| {
                        cosmic_settings_config::shortcuts::system_actions(&config)
                            .get(&cosmic_settings_config::shortcuts::action::System::Terminal)
                            .cloned()
                    })
                    .unwrap_or_else(|| String::from("cosmic-term"));

                app_exec = format!("{term} -- {}", app_exec);
                is_terminal = false;
            }

            app_exec = format!("flatpak-spawn --host /bin/sh -l -c '{}'", app_exec);
        }

        tokio::spawn(async move {
            cosmic::desktop::spawn_desktop_exec(app_exec, env_vars, app_id.as_deref(), is_terminal)
                .await;
        });

        self.update_recent_applications(app);

        if let Some(p) = self.popup.take() {
            return destroy_popup(p);
        }
        Task::none()
    }

    fn update_recent_applications(&mut self, app: Arc<ApplicationEntry>) {
        let current_recent_application = self
            .config
            .recent_applications
            .iter_mut()
            .find(|x| x.app_id == app.id);
        if let Some(recent_app) = current_recent_application {
            if recent_app.launch_count < u32::MAX {
                recent_app.launch_count += 1;
            }
        } else {
            self.config.recent_applications.push(RecentApplication {
                app_id: app.id.clone(),
                launch_count: 1,
            });
        }

        self.config
            .write_entry(AppletConfig::config_handler().as_ref().unwrap())
            .expect("Failed to write recent applications config");
    }

    fn select_category(&mut self, category: ApplicationCategory) -> Task<Message> {
        self.search_field.clear();
        self.selected_category = Some(category.clone());
        self.selected_item_index = None;

        Task::batch([
            // reset scroll position
            cosmic::iced_runtime::task::widget(
                cosmic::iced_core::widget::operation::scrollable::snap_to(
                    self.scrollable_id.clone(),
                    RelativeOffset { x: 0., y: 0. },
                ),
            ),
            Task::perform(
                tokio::task::spawn_blocking(move || {
                    crate::logic::apps::get_apps_of_category(category)
                }),
                |res| cosmic::Action::App(Message::UpdateAvailableApplications(res.unwrap())),
            ),
        ])
    }

    fn launch_tool(&mut self, tool: SystemTool) -> Task<Message> {
        tool.perform();
        if let Some(p) = self.popup.take() {
            return destroy_popup(p);
        }
        Task::none()
    }

    fn handle_zbus_result(&self, result: Result<(), zbus::Error>) -> Task<Message> {
        if let Err(e) = result {
            log::error!("cosmic-ext-classic-menu ERROR: '{}'", e);
        }

        Task::none()
    }

    fn view_main_menu(&self) -> Element<'_, Message> {
        // TODO: Implement grid view
        AppletMenu::view_main_menu_list(&self)
    }

    fn view_context_menu(&self) -> Element<'_, Message> {
        let context_menu = column![
            cosmic::applet::menu_button(
                row![cosmic::widget::text::body(fl!("settings")),].align_y(Alignment::Center)
            )
            .class(cosmic::theme::Button::AppletMenu)
            .on_press(Message::LaunchTool(SystemTool::APPLET_SETTINGS)),
            cosmic::applet::padded_control(cosmic::widget::divider::horizontal::default()),
            cosmic::applet::menu_button(
                row![cosmic::widget::text::body(fl!("settings-label")),].align_y(Alignment::Center)
            )
            .class(cosmic::theme::Button::AppletMenu)
            .on_press(Message::LaunchTool(SystemTool::SYSTEM_SETTINGS)),
            cosmic::applet::menu_button(
                row![cosmic::widget::text::body(fl!("system-monitor-label")),]
                    .align_y(Alignment::Center)
            )
            .class(cosmic::theme::Button::AppletMenu)
            .on_press(Message::LaunchTool(SystemTool::SYSTEM_MONITOR)),
            cosmic::applet::menu_button(
                row![cosmic::widget::text::body(fl!("disks-label")),].align_y(Alignment::Center)
            )
            .class(cosmic::theme::Button::AppletMenu)
            .on_press(Message::LaunchTool(SystemTool::DISK_MANAGEMENT)),
        ]
        .padding([8, 0]);

        self.core.applet.popup_container(context_menu).into()
    }

    fn select_previous_app(&mut self) -> cosmic::Task<cosmic::Action<Message>> {
        if self.selected_item_index.is_none() {
            return Task::none();
        }
        if let Some(index) = self.selected_item_index {
            if index > 0 {
                self.selected_item_index = Some(index - 1);
            }
        }

        return Task::batch([cosmic::iced_runtime::task::widget(
            cosmic::iced_core::widget::operation::scrollable::snap_to(
                self.scrollable_id.clone(),
                RelativeOffset {
                    x: 0.,
                    y: (self.selected_item_index.unwrap() as f32
                        / (self.available_applications.len() as f32 - 1.).max(1.))
                    .max(0.0),
                },
            ),
        )]);
    }

    fn select_next_app(&mut self) -> cosmic::Task<cosmic::Action<Message>> {
        // Disable keyboard selection during active search to avoid re-render overhead
        if !self.search_field.is_empty() {
            return Task::none();
        }

        if self.selected_item_index.is_none() && !self.available_applications.is_empty() {
            self.selected_item_index = Some(0);
        } else if let Some(index) = self.selected_item_index {
            if index < self.available_applications.len() - 1 {
                self.selected_item_index = Some(index + 1);
            }
        }

        return Task::batch([cosmic::iced_runtime::task::widget(
            cosmic::iced_core::widget::operation::scrollable::snap_to(
                self.scrollable_id.clone(),
                RelativeOffset {
                    x: 0.,
                    y: (self.selected_item_index.unwrap() as f32
                        / (self.available_applications.len() as f32 - 1.).max(1.))
                    .max(0.0),
                },
            ),
        )]);
    }
}
