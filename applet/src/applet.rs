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
use cosmic::iced::widget::operation::AbsoluteOffset;
use cosmic::iced::widget::scrollable::{RelativeOffset, Viewport};
use cosmic::iced::{
    Alignment,
    platform_specific::shell::commands::popup::destroy_popup,
    widget::{column, row},
    window::Id,
};
use cosmic::iced::{Subscription, keyboard};
use cosmic::surface::Action;
use cosmic::surface::action::{LiveSettings, app_popup};
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
    /// Context menu of `context_menu_target`, built when it is right clicked.
    pub context_menu: Option<Vec<cosmic::widget::menu::Tree<Message>>>,
    /// Scroll offset for virtualization (pixels from top)
    pub scroll_offset: f32,
    /// Viewport height for virtualization and selection scroll behavior.
    pub scroll_viewport_height: f32,
    /// Favorite apps, shown in the sidebar of some layouts. Falls back to
    /// the most used apps when there are no favorites.
    pub favorite_applications: Vec<Arc<ApplicationEntry>>,
    /// Most used apps, shown as recommendations in some layouts.
    pub recent_applications: Vec<Arc<ApplicationEntry>>,
    /// Index of the app in `available_applications` whose context menu is
    /// shown on right click.
    pub context_menu_target: Option<usize>,
    /// Whether an app claimed the current right click, see
    /// [`Message::ContextMenuRightPress`].
    context_menu_target_claimed: bool,
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
    /// All apps and their categories, loaded in the background.
    ApplicationsLoaded(Vec<Arc<ApplicationEntry>>, Vec<ApplicationCategory>),
    SelectPreviousApp,
    SelectNextApp,
    LaunchSelectedApplication,
    SuperKeyPressed,
    AppListConfigUpdated(AppListConfig),
    ContextMenuAction(Action<Message>),
    LaunchApplicationAt(usize),
    LaunchApplicationWithActionAt(usize, usize),
    PinToAppTrayIndex(usize, bool),
    ScrollUpdated(Viewport),
    /// Right press on an app, sent before [`Message::ContextMenuRightPress`].
    ContextMenuTarget(usize),
    /// Adds or removes the app at this index from the favorites.
    ToggleFavoriteAt(usize),
    OpenPlace(crate::model::place::Place),
    /// Right press anywhere in the app view.
    ContextMenuRightPress,
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
            context_menu: None,
            scroll_offset: 0.0,
            scroll_viewport_height: 0.0,
            favorite_applications: Vec::new(),
            recent_applications: Vec::new(),
            context_menu_target: None,
            context_menu_target_claimed: false,
        };

        // fetch current user asynchronously
        let fetch_current_user_task =
            Task::perform(crate::model::user::get_current_user(), |result| {
                cosmic::Action::App(Message::UpdateLoggedUser(result))
            });

        (
            window,
            Task::batch(vec![fetch_current_user_task, Applet::load_applications()]),
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
                // Layout and favorites decide what the sidebar and the
                // context menu show.
                self.update_featured_applications();
                self.update_context_menu();

                Task::none()
            }
            Message::UpdateAvailableApplications(items) => {
                self.available_applications = items;
                // Indices into the old list no longer apply.
                self.context_menu_target = None;
                self.context_menu = None;

                Task::none()
            }
            Message::ApplicationsLoaded(applications, categories) => {
                self.available_categories = categories;
                // Leave search results and other categories in place.
                if self.search_field.is_empty()
                    && self.selected_category == Some(ApplicationCategory::ALL)
                {
                    self.available_applications = applications;
                    self.context_menu_target = None;
                    self.context_menu = None;
                }
                self.update_featured_applications();

                Task::none()
            }
            Message::SelectPreviousApp => self.select_previous_app(),
            Message::SelectNextApp => self.select_next_app(),
            Message::LaunchSelectedApplication => {
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
                    if let Some(app_list_helper) =
                        Config::new(cosmic_app_list_config::APP_ID, AppListConfig::VERSION).ok()
                    {
                        if favorites {
                            // currently favorites==true indicates it is pinned; request unpin
                            self.app_list_config.remove_pinned(&app.id, &app_list_helper);
                        } else {
                            self.app_list_config.add_pinned(app.id.clone(), &app_list_helper);
                        }

                        // Reflect the new pin state
                        self.update_context_menu();
                    }
                }

                Task::none()
            }
            Message::ToggleFavoriteAt(app_index) => {
                if let Some(app) = self.available_applications.get(app_index).cloned() {
                    let mut favorites = self.config.favorite_applications.clone();
                    if let Some(position) = favorites.iter().position(|id| *id == app.id) {
                        favorites.remove(position);
                    } else {
                        favorites.push(app.id.clone());
                    }

                    // Write only this key, see `update_recent_applications`.
                    if let Err(err) = self.config.set_favorite_applications(
                        AppletConfig::config_handler().as_ref().unwrap(),
                        favorites,
                    ) {
                        log::error!("Failed to write favorite applications: {err}");
                    }
                    self.update_featured_applications();
                    self.update_context_menu();
                }

                Task::none()
            }
            Message::OpenPlace(place) => {
                place.open();
                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                }
                Task::none()
            }
            Message::AppListConfigUpdated(app_list_config) => {
                self.app_list_config = app_list_config;
                // The pin state shown in the context menu may have changed.
                self.update_context_menu();

                Task::none()
            }
            Message::ContextMenuAction(action) => {
                return cosmic::task::message(cosmic::Action::Surface(action));
            }
            Message::ContextMenuTarget(index) => {
                self.context_menu_target = Some(index);
                self.context_menu_target_claimed = true;
                self.update_context_menu();
                Task::none()
            }
            Message::ContextMenuRightPress => {
                if !self.context_menu_target_claimed {
                    self.context_menu_target = None;
                    self.context_menu = None;
                }
                self.context_menu_target_claimed = false;
                Task::none()
            }
            Message::ScrollUpdated(viewport) => {
                self.scroll_offset = viewport.absolute_offset().y;
                self.scroll_viewport_height = viewport.bounds().height;
                Task::none()
            }
        }
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They are started at the
    /// beginning of the application, and persist through its lifetime.
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            desktop_files().map(Message::FileEvent),
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
            // Watch the config files directly rather than through
            // `Core::watch_config`, which relies on cosmic-settings-daemon
            // and never delivers changes when the daemon is unavailable.
            cosmic::cosmic_config::config_subscription::<_, AppletConfig>(
                std::any::TypeId::of::<AppletConfig>(),
                Self::APP_ID.into(),
                AppletConfig::VERSION,
            )
                .map(|update| {
                    for error in &update.errors {
                        log::warn!("Failed to read applet config: {error:?}");
                    }
                    log::debug!("Applet config keys changed: {:?}", update.keys);
                    Message::UpdateConfig(update.config)
                }),
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
                // Invalidate the cache and reload it right away in the
                // background, so opening the menu does not have to.
                log::debug!("App list has been updated, invalidating cache and loading new list!");
                crate::logic::apps::APPS_CACHE.lock().unwrap().cache_reset();

                Applet::load_applications()
            }
        }
    }

    fn toggle_popup(&mut self, popup_type: PopupType) -> Task<Message> {
        if let Some(p) = self.popup.take() {
            return destroy_popup(p);
        }

        let mut tasks = vec![];
        self.popup_type = popup_type;
        if self.popup_type == PopupType::MainMenu {
            // Reset the menu. The apps come from the cache, which is kept
            // warm in the background, so the menu opens without waiting.
            self.search_field.clear();
            self.selected_category = Some(ApplicationCategory::ALL);
            self.selected_item_index = None;
            self.scroll_offset = 0.0;
            self.context_menu_target = None;
            self.context_menu = None;
            match crate::logic::apps::with_cached_apps(<[_]>::to_vec) {
                Some(applications) => {
                    self.available_applications = applications;
                    self.update_featured_applications();
                }
                None => tasks.push(Applet::load_applications()),
            }
        }

        {
            let new_id = Id::unique();
            self.popup.replace(new_id);
            // Open the popup as a libcosmic surface (rather than a raw iced
            // popup) so libcosmic tracks it and applies the frosted glass blur.
            let popup = app_popup::<Applet>(
                |_| LiveSettings::default(),
                move |state: &mut Applet| {
                    let mut popup_settings = state.core.applet.get_popup_settings(
                        state.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    let (anchor, gravity) = match state.core.applet.anchor {
                        PanelAnchor::Left => (Anchor::TopRight, Gravity::BottomRight),
                        PanelAnchor::Right => (Anchor::TopLeft, Gravity::BottomLeft),
                        PanelAnchor::Top => (Anchor::BottomLeft, Gravity::BottomRight),
                        PanelAnchor::Bottom => (Anchor::TopLeft, Gravity::TopRight),
                    };
                    popup_settings.positioner.anchor = anchor;
                    popup_settings.positioner.gravity = gravity;
                    popup_settings
                },
                None,
            );

            tasks.push(cosmic::task::message(cosmic::Action::Surface(popup)));
            Task::batch(tasks)
        }
    }

    /// Loads all apps and their categories in the background.
    fn load_applications() -> Task<Message> {
        Task::perform(
            tokio::task::spawn_blocking(|| {
                let applications = load_apps();
                let categories = crate::logic::apps::load_app_categories();
                (applications, categories)
            }),
            |res| match res {
                Ok((applications, categories)) => {
                    cosmic::action::app(Message::ApplicationsLoaded(applications, categories))
                }
                Err(err) => {
                    log::error!("Failed to load applications: {err}");
                    cosmic::action::none()
                }
            },
        )
    }

    /// Updates the recent and favorite apps, if the layout shows them.
    fn update_featured_applications(&mut self) {
        let layout = self.config.menu_layout;
        let show_favorites = layout.has_favorites();
        // Recent apps also stand in for missing favorites.
        let show_recent = layout == crate::config::MenuLayout::Modern || show_favorites;

        let config = &self.config;
        let featured = crate::logic::apps::with_cached_apps(|apps| {
            let recent = if show_recent {
                crate::logic::apps::get_recent_applications(&config.recent_applications, apps)
            } else {
                Vec::new()
            };
            let favorites = if show_favorites {
                crate::logic::apps::get_favorite_applications(&config.favorite_applications, apps)
            } else {
                Vec::new()
            };
            (recent, favorites)
        });

        // Without cached apps, they are updated once the apps are loaded.
        if let Some((recent, favorites)) = featured {
            // Keep the favorites bar useful until the user adds favorites.
            self.favorite_applications = if show_favorites && favorites.is_empty() {
                recent.iter().take(6).cloned().collect()
            } else {
                favorites
            };
            self.recent_applications = recent;
        }
    }

    /// Builds the menu for the right clicked app, the only one needed.
    fn update_context_menu(&mut self) {
        self.context_menu = self.context_menu_target.and_then(|app_index| {
            self.available_applications
                .get(app_index)
                .map(|app| self.build_context_menu(app_index, app))
        });
    }

    /// Context menu of the app at `app_index` in `available_applications`.
    fn build_context_menu(
        &self,
        app_index: usize,
        app: &ApplicationEntry,
    ) -> Vec<cosmic::widget::menu::Tree<Message>> {
        use crate::applet_menu::ContextMenuAction;
        use cosmic::widget::menu::Item;

        let is_app_in_favorites =
            crate::logic::apps::is_app_in_favorites(app, &self.app_list_config);

        let mut context_menu_buttons = vec![
            Item::Button(
                crate::fl!("launch"),
                None,
                ContextMenuAction::LaunchApplication(app_index),
            ),
            Item::CheckBox(
                crate::fl!("pin-to-panel"),
                None,
                is_app_in_favorites,
                ContextMenuAction::PinToPanel(app_index, is_app_in_favorites),
            ),
        ];

        // Only offered where the layout shows the favorites.
        if self.config.menu_layout.has_favorites() {
            context_menu_buttons.push(Item::CheckBox(
                crate::fl!("add-to-favorites"),
                None,
                self.config.favorite_applications.contains(&app.id),
                ContextMenuAction::ToggleFavorite(app_index),
            ));
        }

        if !app.desktop_actions.is_empty() {
            context_menu_buttons.push(Item::Divider);
            context_menu_buttons.extend(app.desktop_actions.iter().enumerate().map(
                |(action_index, action)| {
                    Item::Button(
                        action.name.to_string(),
                        None,
                        ContextMenuAction::LaunchApplicationWithAction(app_index, action_index),
                    )
                },
            ));
        }

        cosmic::widget::menu::items(&std::collections::HashMap::new(), context_menu_buttons)
    }

    /// Items per row and row height of the app view in the current layout,
    /// used to move the selection and keep it scrolled into view.
    fn app_view_metrics(&self) -> (usize, f32) {
        if self.config.menu_layout == crate::config::MenuLayout::Modern {
            (
                crate::widgets::VirtualizedAppGrid::COLUMNS,
                crate::widgets::VirtualizedAppGrid::ROW_HEIGHT,
            )
        } else {
            (1, cosmic::theme::active().cosmic().spacing.space_xl as f32)
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
        self.search_field.clear();

        match crate::logic::apps::with_cached_apps(<[_]>::to_vec) {
            Some(applications) => self.update(Message::UpdateAvailableApplications(applications)),
            None => Applet::load_applications(),
        }
    }

    fn update_search_field(&mut self, input: String) -> Task<Message> {
        self.selected_category = None;
        self.selected_item_index = None;

        if input.is_empty() {
            return self.clear_search();
        }
        self.search_field.clone_from(&input);

        Task::batch([
            // reset scroll position
            cosmic::iced::widget::operation::snap_to(
                self.scrollable_id.clone(),
                RelativeOffset { x: 0., y: 0. },
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
        let exec = match &action {
            Some(action) => action.exec.as_str(),
            None => app.exec.as_deref().unwrap_or_default(),
        };
        // Drop field codes like %U, which only apply to opening files.
        let mut app_exec = exec
            .split_whitespace()
            .filter(|arg| !arg.starts_with('%'))
            .collect::<Vec<_>>()
            .join(" ");
        let env_vars: Vec<(String, String)> = std::env::vars().collect();
        let app_id = Some(app.id.clone());
        let mut is_terminal = app.is_terminal;

        let is_flatpak = std::env::var("FLATPAK_ID").is_ok();

        if is_flatpak {
            if is_terminal {
                // For flatpaks handle terminal applications manually
                // not through libcosmic implementation
                let term = crate::model::place::terminal_command();

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
        let mut recent_applications = self.config.recent_applications.clone();
        if let Some(recent_app) = recent_applications.iter_mut().find(|x| x.app_id == app.id) {
            recent_app.launch_count = recent_app.launch_count.saturating_add(1);
        } else {
            recent_applications.push(RecentApplication {
                app_id: app.id.clone(),
                launch_count: 1,
            });
        }

        // Write only this key, so settings changed meanwhile by the settings
        // app are not overwritten with this applet's copy of the config.
        self.config
            .set_recent_applications(
                AppletConfig::config_handler().as_ref().unwrap(),
                recent_applications,
            )
            .expect("Failed to write recent applications config");
    }

    fn select_category(&mut self, category: ApplicationCategory) -> Task<Message> {
        self.search_field.clear();
        self.selected_category = Some(category.clone());
        self.selected_item_index = None;
        // Only needed, and so only copied, for the recently used category.
        let recent_applications = if category == ApplicationCategory::RECENTLY_USED {
            self.config.recent_applications.clone()
        } else {
            Vec::new()
        };

        Task::batch([
            // reset scroll position
            cosmic::iced::widget::operation::snap_to(
                self.scrollable_id.clone(),
                RelativeOffset { x: 0., y: 0. },
            ),
            Task::perform(
                tokio::task::spawn_blocking(move || {
                    crate::logic::apps::get_apps_of_category(category, &recent_applications)
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

        let (columns, item_height) = self.app_view_metrics();

        if let Some(index) = self.selected_item_index {
            if index >= columns {
                self.selected_item_index = Some(index - columns);
            }
        }

        if let Some(index) = self.selected_item_index {
            let viewport_height = self.scroll_viewport_height.max(item_height);
            let visible_top = self.scroll_offset;
            let visible_bottom = visible_top + viewport_height;

            let selected_top = (index / columns) as f32 * item_height;
            let selected_bottom = selected_top + item_height;

            if selected_top >= visible_top && selected_bottom <= visible_bottom {
                return Task::none();
            }

            let target_offset = if selected_top < visible_top {
                selected_top
            } else {
                selected_bottom - viewport_height
            };

            return Task::batch([cosmic::iced::widget::operation::scroll_to(
                self.scrollable_id.clone(),
                AbsoluteOffset { x: 0., y: target_offset },
            )]);
        }

        Task::none()
    }

    fn select_next_app(&mut self) -> cosmic::Task<cosmic::Action<Message>> {
        let (columns, item_height) = self.app_view_metrics();

        if self.selected_item_index.is_none() && !self.available_applications.is_empty() {
            self.selected_item_index = Some(0);
        } else if let Some(index) = self.selected_item_index {
            // In a grid, move down a row, stopping at the last app.
            let last = self.available_applications.len().saturating_sub(1);
            if index < last {
                self.selected_item_index = Some((index + columns).min(last));
            }
        }

        if let Some(index) = self.selected_item_index {
            let viewport_height = self.scroll_viewport_height.max(item_height);
            let visible_top = self.scroll_offset;
            let visible_bottom = visible_top + viewport_height;

            let selected_top = (index / columns) as f32 * item_height;
            let selected_bottom = selected_top + item_height;

            if selected_top >= visible_top && selected_bottom <= visible_bottom {
                return Task::none();
            }

            let target_offset = if selected_top < visible_top {
                selected_top
            } else {
                selected_bottom - viewport_height
            };

            return Task::batch([cosmic::iced::widget::operation::scroll_to(
                self.scrollable_id.clone(),
                AbsoluteOffset { x: 0., y: target_offset + 16.0 },
            )]);
        }

        Task::none()
    }
}
