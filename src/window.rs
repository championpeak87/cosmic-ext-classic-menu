use cosmic::app::Core;
use cosmic::cosmic_config::Config;
use cosmic::desktop::DesktopEntryData;
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
use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::Arc;

use crate::logic::{available_categories, load_apps};

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
    available_categories: HashSet<String>,
    available_applications: Vec<Arc<DesktopEntryData>>,
    all_applications: Vec<Arc<DesktopEntryData>>
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
    ApplicationSelected(Arc<DesktopEntryData>),
    CategorySelected(String),
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
            available_applications: load_apps(),
            available_categories: available_categories(),
            all_applications: load_apps()
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
                    self.available_applications = load_apps();
                } else {
                    self.available_applications = self
                        .available_applications
                        .clone()
                        .into_iter()
                        .filter(|x| {
                            x.name
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
            Message::ApplicationSelected(_app) => {
                // cosmic::desktop::spawn_desktop_exec(app.exec, None, None);
            }
            Message::CategorySelected(category) => {
                self.available_applications = load_apps()
                    .into_iter()
                    .filter(|app| app.categories.contains(&category))
                    .collect();
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
                let comment = "App comment";
                col.push(
                    cosmic::widget::button::custom(container(row![
                        app.icon
                            .as_cosmic_icon()
                            .width(Length::Fixed(20.))
                            .height(Length::Fixed(20.)),
                        column![text(app.name.clone()), text(comment).size(8.0)].padding(5)
                    ]))
                    .width(Length::Fill)
                    .on_press(Message::ApplicationSelected(app.clone())),
                )
                .width(Length::Fill)
            })
            .padding(5);
        let places_list = self.available_categories
            .iter()
            .fold(cosmic::widget::column(), move |col, category| {
                col.push(
                    button(category.as_str())
                        .on_press(Message::CategorySelected(category.to_string()))
                        .width(Length::Fill),
                )
                .width(Length::Fill)
            })
            .padding(5);

        let menu_layout = column![
            power_menu,
            search_field,
            row![
                scrollable(app_list).width(Length::FillPortion(2)),
                scrollable(places_list).width(Length::FillPortion(1))
            ]
            .padding(5)
        ]
        .padding(10);

        self.core
            .applet
            .popup_container(menu_layout)
            .max_height(500.)
            .into()
    }
}
