// SPDX-License-Identifier: {{ license }}

use crate::{app, fl};
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::widget::{self, icon, menu, nav_bar};
use cosmic::{applet, prelude::*};
use cosmic::{cosmic_theme, theme};
use cosmic_classic_menu::applet_button;
use cosmic_classic_menu::config::{
    AppletButtonStyle, CosmicClassicMenuConfig, HorizontalPosition, RecentApplication,
    UserWidgetStyle, VerticalPosition,
};
use futures_util::SinkExt;
use std::collections::HashMap;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!(
    "../../res/icons/hicolor/scalable/apps/com.championpeak87.cosmic-classic-menu.svg"
);

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    // Configuration data that persists between application runs.
    config: CosmicClassicMenuConfig,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    OpenRepositoryUrl,
    SubscriptionChannel,
    UpdateConfig(CosmicClassicMenuConfig),
    LaunchUrl(String),
    AppPositionChanged(HorizontalPosition),
    SearchFieldPositionChanged(VerticalPosition),
    AppletButtonStyleChanged(usize),
    UserWidgetChanged(usize),
    ButtonLabelChanged(String),
    ButtonIconChanged(String),
    ToggleContextPage(ContextPage),
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "com.championpeak87.cosmic-classic-menu";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        mut core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        core.window.show_maximize = false;

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config: CosmicClassicMenuConfig::config(),
        };

        (app, Task::none())
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<Self::Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(fl!("settings")).apply(Element::from),
            menu::items(
                &self.key_binds,
                vec![
                    menu::Item::Button(
                        fl!("default-settings"),
                        None,
                        MenuAction::SetDefaultSettings,
                    ),
                    menu::Item::Button(fl!("about"), None, MenuAction::About),
                ],
            ),
        )]);

        vec![menu_bar.into()]
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<Self::Message> {
        let app_menu_position = cosmic::iced::widget::row![
            cosmic::widget::Radio::new(
                cosmic::widget::text::heading(fl!("left")),
                HorizontalPosition::Left,
                Some(self.config.app_menu_position),
                Message::AppPositionChanged
            ),
            cosmic::widget::Space::new(5, Length::Shrink),
            cosmic::widget::Radio::new(
                cosmic::widget::text::heading(fl!("right")),
                HorizontalPosition::Right,
                Some(self.config.app_menu_position),
                Message::AppPositionChanged
            )
        ];
        let search_field_position = cosmic::iced::widget::row![
            cosmic::widget::Space::new(Length::Fill, 5),
            cosmic::widget::Radio::new(
                cosmic::widget::text::heading(fl!("top")),
                VerticalPosition::Top,
                Some(self.config.search_field_position),
                Message::SearchFieldPositionChanged
            ),
            cosmic::widget::Space::new(5, Length::Shrink),
            cosmic::widget::Radio::new(
                cosmic::widget::text::heading(fl!("bottom")),
                VerticalPosition::Bottom,
                Some(self.config.search_field_position),
                Message::SearchFieldPositionChanged
            )
        ];
        let applet_button_style = cosmic::iced::widget::row![
            cosmic::widget::Space::new(Length::Fill, 5),
            cosmic::widget::dropdown(
                vec![
                    fl!("icon-only"),
                    fl!("label-only"),
                    fl!("icon-and-label"),
                    fl!("auto")
                ],
                Some(self.config.applet_button_style as usize),
                Message::AppletButtonStyleChanged
            )
        ];
        let user_widget = cosmic::iced::widget::row![
            cosmic::widget::Space::new(Length::Fill, 5),
            cosmic::widget::dropdown(
                vec![
                    fl!("username-prefered"),
                    fl!("realname-prefered"),
                    fl!("none")
                ],
                Some(self.config.user_widget as usize),
                Message::UserWidgetChanged
            )
        ];
        let button_label = cosmic::iced::widget::row![
            cosmic::widget::Space::new(Length::Fill, 5),
            cosmic::widget::text_input(
                fl!("button-label-placeholder"),
                &self.config.button_label
            )
            .on_input(Message::ButtonLabelChanged)
        ];
        let button_icon = cosmic::iced::widget::row![
            cosmic::widget::Space::new(Length::Fill, 5),
            cosmic::widget::text_input(
                fl!("button-icon-placeholder"),
                &self.config.button_icon
            )
            .on_input(Message::ButtonIconChanged)
        ];

        let settings_container =
            cosmic::widget::settings::view_column(vec![cosmic::widget::settings::section()
                .title(fl!("general"))
                .add(cosmic::widget::settings::item(
                    fl!("app-menu-position"),
                    app_menu_position,
                ))
                .add(cosmic::widget::settings::item(
                    fl!("search-field-position"),
                    search_field_position,
                ))
                .add(cosmic::widget::settings::item(
                    fl!("applet-button-style"),
                    applet_button_style,
                ))
                .add(cosmic::widget::settings::item(
                    fl!("user-widget"),
                    user_widget,
                ))
                .add(cosmic::widget::settings::item(
                    fl!("button-label"),
                    button_label,
                ))
                .add(cosmic::widget::settings::item(
                    fl!("button-icon"),
                    button_icon,
                ))
                .into()]);

        settings_container.padding([5, 10]).into()
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::context_drawer(
                self.about(),
                Message::ToggleContextPage(ContextPage::About),
            )
            .title(fl!("about")),
        })
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They are started at the
    /// beginning of the application, and persist through its lifetime.
    fn subscription(&self) -> Subscription<Self::Message> {
        struct MySubscription;

        Subscription::batch(vec![
            // Create a subscription which emits updates through a channel.
            Subscription::run_with_id(
                std::any::TypeId::of::<MySubscription>(),
                cosmic::iced::stream::channel(4, move |mut channel| async move {
                    _ = channel.send(Message::SubscriptionChannel).await;

                    futures_util::future::pending().await
                }),
            ),
            // Watch for application configuration changes.
            self.core()
                .watch_config::<CosmicClassicMenuConfig>(Self::APP_ID)
                .map(|update| {
                    // for why in update.errors {
                    //     tracing::error!(?why, "app config error");
                    // }

                    Message::UpdateConfig(update.config)
                }),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::OpenRepositoryUrl => {
                        _ = open::that_detached(REPOSITORY);
                    }
            Message::SubscriptionChannel => {
                        // For example purposes only.
                    }
            Message::UpdateConfig(config) => {
                        self.config = config;

                        self.config
                            .write_entry(CosmicClassicMenuConfig::config_handler().as_ref().unwrap())
                            .expect("Failed to write recent applications config");
                    }
            Message::LaunchUrl(url) => match open::that_detached(&url) {
                        Ok(()) => {}
                        Err(err) => {
                            eprintln!("failed to open {url:?}: {err}");
                        }
                    },
            Message::AppPositionChanged(horizontal_position) => {
                        println!("App position changed to: {:?}", horizontal_position);
                        self.config.app_menu_position = horizontal_position;

                        self.config
                            .write_entry(CosmicClassicMenuConfig::config_handler().as_ref().unwrap())
                            .expect("Failed to write recent applications config");
                    }
            Message::SearchFieldPositionChanged(vertical_position) => {
                        println!("Search field position changed to: {:?}", vertical_position);
                        self.config.search_field_position = vertical_position;

                        self.config
                            .write_entry(CosmicClassicMenuConfig::config_handler().as_ref().unwrap())
                            .expect("Failed to write search field position config");
                    }
            Message::AppletButtonStyleChanged(applet_button_style) => {
                        println!("Applet button style changed to: {:?}", applet_button_style);
                        self.config.applet_button_style = match applet_button_style {
                            0 => AppletButtonStyle::IconOnly,
                            1 => AppletButtonStyle::LabelOnly,
                            2 => AppletButtonStyle::IconAndLabel,
                            3 => AppletButtonStyle::Auto,
                            _ => AppletButtonStyle::Auto,
                        };

                        self.config
                            .write_entry(CosmicClassicMenuConfig::config_handler().as_ref().unwrap())
                            .expect("Failed to write applet button style config");
                    }
            Message::UserWidgetChanged(user_widget_style) => {
                        println!("User widget style changed to: {:?}", user_widget_style);
                        self.config.user_widget = match user_widget_style {
                            0 => UserWidgetStyle::UsernamePrefered,
                            1 => UserWidgetStyle::RealNamePrefered,
                            2 => UserWidgetStyle::None,
                            _ => UserWidgetStyle::None,
                        };

                        self.config
                            .write_entry(CosmicClassicMenuConfig::config_handler().as_ref().unwrap())
                            .expect("Failed to write user widget style config");
                    }
            Message::ButtonLabelChanged(new_label) => {
                        let mut new_label = new_label;
                        if new_label.len() == 0 {
                            // If the field is empty, reset to default.
                            new_label = CosmicClassicMenuConfig::default().button_label;
                        }

                        println!("Button label changed to: {:?}", new_label);
                        self.config.button_label = new_label;

                        self.config
                            .write_entry(CosmicClassicMenuConfig::config_handler().as_ref().unwrap())
                            .expect("Failed to write button label config");
                    }
            Message::ButtonIconChanged(new_icon) => {
                        println!("Button icon changed to: {:?}", new_icon);
                        self.config.button_icon = new_icon;

                        self.config
                            .write_entry(CosmicClassicMenuConfig::config_handler().as_ref().unwrap())
                            .expect("Failed to write button icon config");
                    }
            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            },
        }
        Task::none()
    }
}

impl AppModel {
    /// The about page for this app.
    pub fn about(&self) -> Element<Message> {
        let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

        let icon = widget::svg(widget::svg::Handle::from_memory(APP_ICON));

        let title = widget::text::title3(fl!("app-title"));

        let hash = env!("VERGEN_GIT_SHA");
        let short_hash: String = hash.chars().take(7).collect();
        let date = env!("VERGEN_GIT_COMMIT_DATE");

        let link = widget::button::link(REPOSITORY)
            .on_press(Message::OpenRepositoryUrl)
            .padding(0);

        widget::column()
            .push(icon)
            .push(title)
            .push(link)
            .push(
                widget::button::link(fl!(
                    "git-description",
                    hash = short_hash.as_str(),
                    date = date
                ))
                .on_press(Message::LaunchUrl(format!("{REPOSITORY}/commits/{hash}")))
                .padding(0),
            )
            .align_x(Alignment::Center)
            .spacing(space_xxs)
            .into()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
    SetDefaultSettings,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => Message::ToggleContextPage(ContextPage::About),
            MenuAction::SetDefaultSettings => {
                Message::UpdateConfig(CosmicClassicMenuConfig::default())
            }
        }
    }
}

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
}