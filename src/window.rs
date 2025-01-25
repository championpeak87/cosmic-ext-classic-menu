use crate::logic::get_apps;
use crate::{spawn_detached, launcher};
use cosmic::app::Core;
use cosmic::cosmic_config::Config;
use cosmic::iced::{
    platform_specific::shell::commands::popup::{destroy_popup, get_popup},
    widget::{column, row},
    window::Id,
    Alignment, Length, Limits, Task,
};
use cosmic::iced_runtime::core::window;
use cosmic::iced_widget::button;
use cosmic::process::spawn;
use cosmic::widget::{container, text_input};
use cosmic::widget::{scrollable, text};
use cosmic::Element;
use freedesktop_desktop_entry::{get_languages_from_env, DesktopEntry};
use tokio::sync::mpsc;
use tracing::{debug, error, info};
use std::borrow::Cow;
use std::fmt::Debug;
use std::process::Command;
use pop_launcher::SearchResult;
use pop_launcher_service::Service;

const ID: &str = "com.championpeak87.cosmic-classic-menu";
const CONFIG_VERS: u64 = 1;
const POPUP_MAX_WIDTH: f32 = 360.0;
const POPUP_MIN_WIDTH: f32 = 300.0;
const POPUP_MAX_HEIGHT: f32 = 1080.0;
const POPUP_MIN_HEIGHT: f32 = 200.0;

/// Holds the applet's state
pub struct Window {
    core: Core,
    config: Config,
    popup: Option<Id>,
    search_field: String,
    available_applications: Vec<DesktopEntry>,
    launcher_items: Vec<SearchResult>,
    tx: Option<mpsc::Sender<launcher::Request>>,
}

/// Messages to be sent to the Libcosmic Update function
#[derive(Clone, Debug)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    SearchFieldInput(String),
    ShutdownClicked,
    RestartClicked,
    LogOutClicked,
    LockScreenClicked,
    ApplicationSelected(DesktopEntry),
    ApplicationSecondaryClick,
}

impl Window {
    fn request(&self, r: launcher::Request) {
        debug!("request: {:?}", r);
        if let Some(tx) = &self.tx {
            if let Err(e) = tx.blocking_send(r) {
                error!("tx: {e}");
            }
        } else {
            info!("tx not found");
        }
    }
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
        let window = Window {
            core,
            config: Config::new(ID, CONFIG_VERS).unwrap(),
            popup: None,
            search_field: String::new(),
            available_applications: get_apps(),
            tx: None,
            launcher_items: vec![]
        };

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
            Message::TogglePopup => {
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
                }
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::SearchFieldInput(input) => {
                if input.is_empty() {
                    self.available_applications = get_apps();
                } else {
                    self.available_applications = self
                        .available_applications
                        .clone()
                        .into_iter()
                        .filter(|x| {
                            x.name(get_languages_from_env().as_slice())
                                .unwrap_or(Cow::from(""))
                                .to_lowercase()
                                .starts_with(input.to_lowercase().as_str())
                        })
                        .collect();
                }
                self.search_field = input;
            }
            Message::ShutdownClicked => todo!("Shutdown not implemented"),
            Message::RestartClicked => todo!("Restart not implemented"),
            Message::LogOutClicked => todo!("Logout not implemented"),
            Message::LockScreenClicked => todo!("Lock screen not implemented"),
            Message::ApplicationSelected(app) => {
                let app_exec = app.exec().unwrap().split(" ");
                let mut app_exec_arr: Vec<&str> = app_exec.collect();
                let app_exec_cmd: &str = app_exec_arr.remove(0);

                if let Some(p) = self.popup.take() {
                    destroy_popup::<Message>(p);
                }

                spawn_detached::spawn_process_detached(
                    &mut Command::new(app_exec_cmd).args(app_exec_arr),
                );
            }
            Message::ApplicationSecondaryClick => {
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
                }
            }
        }
        Task::none()
    }

    // Libcosmic's view function
    fn view(&self) -> Element<Self::Message> {
        self.core
            .applet
            .icon_button("application-menu-symbolic")
            .on_press(Message::TogglePopup)
            .into()
    }

    // Libcosmic's applet view_window function
    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        let power_menu = container(
            row![
                cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                    "system-shutdown-symbolic"
                ))
                .on_press(Message::ShutdownClicked)
                .tooltip("Shutdown the computer")
                .icon_size(25)
                .height(25)
                .width(25)
                .padding(5),
                cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                    "system-restart-symbolic"
                ))
                .on_press(Message::RestartClicked)
                .tooltip("Restart the computer")
                .icon_size(25)
                .height(25)
                .width(25)
                .padding(5),
                cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                    "system-log-out-symbolic"
                ))
                .on_press(Message::LogOutClicked)
                .tooltip("Logout current user")
                .icon_size(25)
                .height(25)
                .width(25)
                .padding(5),
                cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                    "system-lock-screen-symbolic"
                ))
                .on_press(Message::LockScreenClicked)
                .tooltip("Lock current session")
                .icon_size(25)
                .height(25)
                .width(25)
                .padding(5)
            ]
            .padding(5)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Alignment::End);

        let search_field = text_input("Search app", &self.search_field)
            .on_input(Message::SearchFieldInput)
            .padding(5);

        let app_list: cosmic::widget::Column<Message> = self
            .available_applications
            .iter()
            .fold(cosmic::widget::column(), |col, app| {
                let app_name = app.name(get_languages_from_env().as_slice());
                let comment = app.comment(get_languages_from_env().as_slice());
                let icon_path = app.icon();
                col.push(
                    cosmic::widget::button::custom(container(row![
                        cosmic::widget::button::icon(match icon_path {
                            Some(x) => cosmic::widget::icon::from_name(x),
                            None => cosmic::widget::icon::from_name("system-lock-screen-symbolic")
                                .into(),
                        }),
                        column![
                            text(app_name.unwrap_or(Cow::from(""))),
                            text(comment.unwrap_or(Cow::from(""))).size(8.0)
                        ]
                    ]))
                    .width(Length::Fill)
                    .on_press(Message::ApplicationSelected(app.clone()))
                    .padding(2),
                )
                .width(Length::Fill)
            })
            .padding(5);
        let places_list = column![
            button("Favorites").width(Length::Fill),
            button("Recently used").width(Length::Fill),
            button("All").width(Length::Fill),
            button("Accessories").width(Length::Fill),
            button("Development").width(Length::Fill),
            button("Games").width(Length::Fill),
            button("Graphics").width(Length::Fill),
            button("Internet").width(Length::Fill),
            button("Multimedia").width(Length::Fill),
            button("Office").width(Length::Fill),
            button("Other").width(Length::Fill),
            button("System").width(Length::Fill)
        ]
        .padding(5);

        let menu_layout = column![
            power_menu,
            search_field,
            row![
                scrollable(app_list).width(Length::FillPortion(2)),
                scrollable(places_list).width(Length::FillPortion(1))
            ]
        ]
        .padding(10);

        self.core
            .applet
            .popup_container(menu_layout)
            .max_height(500.)
            .into()
    }
}
