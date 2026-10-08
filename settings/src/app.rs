// SPDX-License-Identifier: {{ license }}

use crate::fl;
use crate::layout_editor::{self, LayoutEditor};
use cosmic::app::context_drawer;
use cosmic::cosmic_config::CosmicConfigEntry;
use cosmic::dialog::file_chooser::FileFilter;
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{button, icon, menu, menu::{ItemWidth, ItemHeight}};
use cosmic::{iced::Background, widget::text, Element};
use cosmic_ext_classic_menu_applet::config::{
    AppletButtonStyle, AppletConfig, Place, SidePanel, UserWidgetStyle,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Height of a row in the places list.
const PLACE_ROW_HEIGHT: f32 = 44.0;
const PLACE_DIVIDER_HEIGHT: f32 = 1.0;
/// Distance between the tops of two rows in the places list.
const PLACE_ROW_PITCH: f32 = PLACE_ROW_HEIGHT + PLACE_DIVIDER_HEIGHT;

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// The about page for this app.
    about: cosmic::widget::about::About,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    // Configuration data that persists between application runs.
    config: AppletConfig,
    /// Drag and drop state of the menu layout preview.
    layout_editor: LayoutEditor,
    /// Index in the places list of the entry being reordered by dragging.
    /// Separators may repeat, so the index tells them apart.
    dragged_place: Option<usize>,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    UpdateConfig(AppletConfig),
    LaunchUrl(String),
    LayoutEditor(layout_editor::Message),
    AppletButtonStyleChanged(usize),
    UserWidgetChanged(usize),
    SidePanelChanged(usize),
    PlaceToggled(Place, bool),
    /// The enabled place at this index was grabbed.
    PlaceDragStart(usize),
    /// Appends a separator to the enabled places.
    AddSeparator,
    /// Removes the separator at this index.
    RemoveSeparator(usize),
    /// The cursor moved to this height over the places list.
    PlaceDragMove(f32),
    /// Left mouse button released anywhere in the window.
    PlaceDragEnd,
    ButtonLabelChanged(String),
    ToggleContextPage(ContextPage),
    OpenIconPicker,
    ButtonIconChanged(PathBuf),
    CustomIconSelected,
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
    const APP_ID: &'static str = cosmic_ext_classic_menu_applet::applet::APP_ID;

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

        let about = cosmic::widget::about::About::default()
            .name(fl!("app-title"))
            .icon(icon::from_name(Self::APP_ID))
            .version(env!("CARGO_PKG_VERSION"))
            .license("GPL-3.0-only")
            .developers([("Kamil Lihan", "k.lihan@outlook.com")])
            .links([
                (
                    fl!("repository"),
                    "https://github.com/championpeak87/cosmic-ext-classic-menu",
                ),
                (
                    fl!("support"),
                    "https://github.com/championpeak87/cosmic-ext-classic-menu/issues",
                ),
            ]);

        // Construct the app model with the runtime's core.
        let app = AppModel {
            core,
            about,
            context_page: ContextPage::default(),
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config: AppletConfig::config(),
            layout_editor: LayoutEditor::default(),
            dragged_place: None,
        };

        (app, Task::none())
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&'_ self) -> Vec<Element<'_, Self::Message>> {
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
        )])
        .item_height(ItemHeight::Dynamic(40))
        .item_width(ItemWidth::Uniform(240));

        vec![menu_bar.into()]
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&'_ self) -> Element<'_, Self::Message> {
        let layout_editor = self
            .layout_editor
            .view(&self.config)
            .map(Message::LayoutEditor);
        let applet_button_style = cosmic::widget::dropdown(
            vec![
                fl!("icon-only"),
                fl!("label-only"),
                fl!("icon-and-label"),
                fl!("auto"),
            ],
            Some(self.config.applet_button_style as usize),
            Message::AppletButtonStyleChanged,
        );
        let user_widget = cosmic::widget::dropdown(
            vec![
                fl!("username-prefered"),
                fl!("realname-prefered"),
                fl!("none"),
            ],
            Some(self.config.user_widget as usize),
            Message::UserWidgetChanged,
        );
        let button_label =
            cosmic::widget::text_input(fl!("button-label-placeholder"), &self.config.button_label)
                .on_input(Message::ButtonLabelChanged)
                .width(Length::Fixed(200.0));
        let button_icon = cosmic::widget::button::text(fl!("button-icon-placeholder"))
            .on_press(Message::OpenIconPicker);

        let settings_container =
            cosmic::widget::settings::view_column(vec![
                cosmic::widget::settings::section()
                    .title(fl!("layout"))
                    .add(layout_editor)
                    .into(),
            ]
            .into_iter()
            .chain(self.side_panel_section())
            .chain([cosmic::widget::settings::section()
                    .title(fl!("general"))
                    .add(Self::setting_item(fl!("applet-button-style"), applet_button_style))
                    .add(Self::setting_item(fl!("user-widget"), user_widget))
                    .add(Self::setting_item(fl!("button-label"), button_label))
                    .add(Self::setting_item(fl!("button-icon"), button_icon))
                    .into()])
            .collect());

        // The window has a fixed height, so let the sections scroll if needed.
        // The extra right padding keeps the scrollbar off the sections.
        cosmic::widget::scrollable(settings_container.padding(cosmic::iced::Padding {
            top: 5.0,
            bottom: 5.0,
            left: 10.0,
            right: 20.0,
        }))
        .into()
    }

    /// Listens for the mouse button release ending a drag, which may happen
    /// outside of the dragged widget.
    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        if self.layout_editor.is_dragging() {
            cosmic::iced::event::listen_with(|event, _status, _window| match event {
                cosmic::iced::Event::Mouse(cosmic::iced::mouse::Event::ButtonReleased(
                    cosmic::iced::mouse::Button::Left,
                )) => Some(Message::LayoutEditor(layout_editor::Message::DragEnd)),
                _ => None,
            })
        } else if self.dragged_place.is_some() {
            cosmic::iced::event::listen_with(|event, _status, _window| match event {
                cosmic::iced::Event::Mouse(cosmic::iced::mouse::Event::ButtonReleased(
                    cosmic::iced::mouse::Button::Left,
                )) => Some(Message::PlaceDragEnd),
                _ => None,
            })
        } else {
            cosmic::iced::Subscription::none()
        }
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&'_ self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::about(
                &self.about,
                |url| Message::LaunchUrl(url.to_string()),
                Message::ToggleContextPage(ContextPage::About),
            )
            .title(fl!("about")),
            ContextPage::IconPicker => context_drawer::context_drawer(
                self.icon_picker(), // 3. Show icon picker
                Message::ToggleContextPage(ContextPage::IconPicker),
            )
            .title(fl!("button-icon")),
        })
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::UpdateConfig(config) => {
                self.config = config;

                self.save_config();

                Task::none()
            }
            Message::LaunchUrl(url) => {
                match open::that_detached(&url) {
                    Ok(()) => {}
                    Err(err) => {
                        log::error!("failed to open {url:?}: {err}");
                    }
                }
                Task::none()
            }
            Message::LayoutEditor(message) => {
                let change = self.layout_editor.update(message, &self.config);

                match change {
                    Some(layout_editor::Change::MenuLayout(menu_layout)) => {
                        log::info!("Menu layout changed to: {:?}", menu_layout);
                        self.config.apply_layout(menu_layout);
                    }
                    Some(layout_editor::Change::AppMenuPosition(horizontal_position)) => {
                        log::info!("App position changed to: {:?}", horizontal_position);
                        self.config.app_menu_position = horizontal_position;
                    }
                    Some(layout_editor::Change::SearchFieldPosition(vertical_position)) => {
                        log::info!("Search field position changed to: {:?}", vertical_position);
                        self.config.search_field_position = vertical_position;
                    }
                    Some(layout_editor::Change::SidePanelWidth(width)) => {
                        log::info!("Side panel width changed to: {width}%");
                        self.config.side_panel_width = width;
                    }
                    Some(layout_editor::Change::PowerMenuPosition(power_menu_position)) => {
                        log::info!("Power menu position changed to: {:?}", power_menu_position);
                        self.config.power_menu_position = power_menu_position;
                    }
                    None => return Task::none(),
                }

                self.save_config();

                Task::none()
            }
            Message::AppletButtonStyleChanged(applet_button_style) => {
                log::info!("Applet button style changed to: {:?}", applet_button_style);
                self.config.applet_button_style = match applet_button_style {
                    0 => AppletButtonStyle::IconOnly,
                    1 => AppletButtonStyle::LabelOnly,
                    2 => AppletButtonStyle::IconAndLabel,
                    3 => AppletButtonStyle::Auto,
                    _ => AppletButtonStyle::Auto,
                };

                self.save_config();

                Task::none()
            }
            Message::SidePanelChanged(index) => {
                self.config.side_panel = if index == 1 {
                    SidePanel::Places
                } else {
                    SidePanel::Categories
                };
                log::info!("Side panel changed to: {:?}", self.config.side_panel);
                self.save_config();

                Task::none()
            }
            Message::PlaceToggled(place, enabled) => {
                self.config.places.retain(|p| *p != place);
                if enabled {
                    self.config.places.push(place);
                }
                self.save_config();

                Task::none()
            }
            Message::PlaceDragStart(index) => {
                self.dragged_place = Some(index);
                Task::none()
            }
            Message::AddSeparator => {
                self.config.places.push(Place::Separator);
                self.save_config();
                Task::none()
            }
            Message::RemoveSeparator(index) => {
                if self.config.places.get(index) == Some(&Place::Separator) {
                    self.config.places.remove(index);
                    self.save_config();
                }
                Task::none()
            }
            Message::PlaceDragMove(y) => {
                // Reorder live, so the list shows where the place will end up.
                // Enabled places come first, so the row under the cursor is
                // also the index in `places`; beyond the ends it clamps.
                let places = &mut self.config.places;
                if let Some(from) = self.dragged_place
                    && from < places.len()
                {
                    let to = ((y / PLACE_ROW_PITCH).max(0.0) as usize).min(places.len() - 1);
                    if to != from {
                        let place = places.remove(from);
                        places.insert(to, place);
                        self.dragged_place = Some(to);
                    }
                }
                Task::none()
            }
            Message::PlaceDragEnd => {
                if self.dragged_place.take().is_some() {
                    self.save_config();
                }
                Task::none()
            }
            Message::UserWidgetChanged(user_widget_style) => {
                log::info!("User widget style changed to: {:?}", user_widget_style);
                self.config.user_widget = match user_widget_style {
                    0 => UserWidgetStyle::UsernamePrefered,
                    1 => UserWidgetStyle::RealNamePrefered,
                    2 => UserWidgetStyle::None,
                    _ => UserWidgetStyle::None,
                };

                self.save_config();

                Task::none()
            }
            Message::ButtonLabelChanged(new_label) => {
                let mut new_label = new_label;
                if new_label.len() == 0 {
                    // If the field is empty, reset to default.
                    new_label = AppletConfig::default().button_label;
                }

                log::info!("Button label changed to: {:?}", new_label);
                self.config.button_label = new_label;

                self.save_config();

                Task::none()
            }
            Message::ButtonIconChanged(new_icon) => {
                log::info!(
                    "Button icon changed to: {:?}",
                    new_icon.clone().to_string_lossy()
                );
                self.config.button_icon = new_icon.to_string_lossy().into_owned();

                self.save_config();

                Task::none()
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

                Task::none()
            }
            Message::OpenIconPicker => {
                self.context_page = ContextPage::IconPicker;
                self.core.window.show_context = true;

                Task::none()
            }
            Message::CustomIconSelected => Task::perform(AppModel::pick_custom_icon(), |res| {
                if let Some(icon_pathbuf) = res {
                    // Icon exists and was selected
                    cosmic::action::app(Message::ButtonIconChanged(icon_pathbuf))
                } else {
                    // Icon selection was cancelled or failed, revert to default
                    cosmic::action::none()
                }
            }),
        }
    }
}

impl AppModel {
    /// Choice between categories and places, for layouts with a side panel.
    fn side_panel_section(&self) -> Option<Element<'_, Message>> {
        if !self.config.menu_layout.has_side_panel() {
            return None;
        }

        let content = cosmic::widget::dropdown(
            vec![fl!("layout-categories"), fl!("layout-places")],
            Some(match self.config.side_panel {
                SidePanel::Categories => 0,
                SidePanel::Places => 1,
            }),
            Message::SidePanelChanged,
        );

        let mut section = cosmic::widget::settings::section()
            .title(fl!("side-panel"))
            .add(Self::setting_item(fl!("side-panel-content"), content));

        if self.config.side_panel == SidePanel::Places {
            let add_separator = cosmic::widget::button::standard(fl!("add-separator"))
                .leading_icon(icon::from_name("list-add-symbolic"))
                .on_press(Message::AddSeparator);
            section = section.add(self.places_list()).add(
                cosmic::iced::widget::row![
                    cosmic::widget::Space::new().width(Length::Fill),
                    add_separator
                ],
            );
        }

        Some(section.into())
    }

    /// Places with toggles; enabled ones first, in order, reordered by
    /// dragging their rows.
    fn places_list(&self) -> Element<'_, Message> {
        let enabled = &self.config.places;
        let disabled = Place::ALL.into_iter().filter(|p| !enabled.contains(p));

        // Enabled entries know their index in `places`; disabled ones have none.
        let entries = enabled
            .iter()
            .copied()
            .enumerate()
            .map(|(index, place)| (place, Some(index)))
            .chain(disabled.map(|place| (place, None)));

        let mut list = cosmic::iced::widget::column![];
        for (row, (place, index)) in entries.enumerate() {
            if row > 0 {
                list = list.push(
                    cosmic::widget::container(cosmic::widget::divider::horizontal::default())
                        .height(Length::Fixed(PLACE_DIVIDER_HEIGHT)),
                );
            }
            list = list.push(self.place_item(place, index));
        }

        // The whole list tracks the cursor, so a place can be dragged past
        // the first and last rows.
        let mut area = cosmic::widget::mouse_area(list);
        if self.dragged_place.is_some() {
            area = area
                .on_move(|point| Message::PlaceDragMove(point.y))
                .interaction(cosmic::iced::mouse::Interaction::Grabbing);
        }
        area.into()
    }

    /// Row toggling a place, or removing a separator. Enabled entries, which
    /// have an `index` in the places list, can be grabbed anywhere but on
    /// their toggle or button, which handle their own clicks.
    fn place_item(&self, place: Place, index: Option<usize>) -> Element<'_, Message> {
        let enabled = index.is_some();
        let dragged = index.is_some() && self.dragged_place == index;

        // Disabled places keep an empty spot, keeping the rows aligned.
        let handle: Element<'_, Message> = if enabled {
            icon::from_name("open-menu-symbolic").size(16).into()
        } else {
            cosmic::widget::Space::new().width(16).into()
        };

        let row = match (place, index) {
            (Place::Separator, Some(index)) => cosmic::widget::settings::item_row(vec![
                handle,
                cosmic::widget::container(cosmic::widget::divider::horizontal::heavy())
                    .center_y(Length::Fill)
                    .width(Length::Fill)
                    .into(),
                cosmic::widget::tooltip(
                    cosmic::widget::button::icon(icon::from_name("edit-delete-symbolic"))
                        .on_press(Message::RemoveSeparator(index)),
                    text::caption(fl!("remove-separator")),
                    cosmic::widget::tooltip::Position::Left,
                )
                .into(),
            ]),
            _ => cosmic::widget::settings::item_row(vec![
                handle,
                icon::from_name(place.icon_name()).size(16).into(),
                text::body(place.display_name())
                    .wrapping(cosmic::iced::core::text::Wrapping::None)
                    .ellipsize(cosmic::iced::core::text::Ellipsize::End(
                        cosmic::iced::core::text::EllipsizeHeightLimit::Lines(1),
                    ))
                    .width(Length::Fill)
                    .into(),
                cosmic::widget::toggler(enabled)
                    .on_toggle(move |enabled| Message::PlaceToggled(place, enabled))
                    .into(),
            ]),
        };

        // Highlight the row being dragged.
        let row = cosmic::widget::container(row)
            .center_y(Length::Fixed(PLACE_ROW_HEIGHT))
            .padding([0, cosmic::theme::active().cosmic().space_xxs()])
            .class(cosmic::theme::Container::custom(move |theme| {
                let cosmic = theme.cosmic();
                let mut style = cosmic::widget::container::Style::default();
                if dragged {
                    let mut accent: cosmic::iced::Color = cosmic.accent_color().into();
                    accent.a = 0.2;
                    style.background = Some(Background::Color(accent));
                    style.border.radius = cosmic.radius_s().into();
                }
                style
            }));

        if let Some(index) = index {
            cosmic::widget::mouse_area(row)
                .on_press(Message::PlaceDragStart(index))
                .interaction(if self.dragged_place.is_some() {
                    cosmic::iced::mouse::Interaction::Grabbing
                } else {
                    cosmic::iced::mouse::Interaction::Grab
                })
                .into()
        } else {
            row.into()
        }
    }

    /// Writes the configuration, keeping the recently used and favorite apps
    /// the applet recorded since this app loaded its copy of the config.
    fn save_config(&mut self) {
        // Keys the applet writes itself.
        let applet_config = AppletConfig::config();
        self.config.recent_applications = applet_config.recent_applications;
        self.config.favorite_applications = applet_config.favorite_applications;
        self.config
            .write_entry(AppletConfig::config_handler().as_ref().unwrap())
            .expect("Failed to write applet config");
    }

    /// A settings row whose title is cut off with an ellipsis, rather than
    /// squeezing the control, when the window is too narrow for both.
    fn setting_item<'a>(
        title: String,
        control: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        cosmic::widget::settings::item_row(vec![
            text::body(title)
                .wrapping(cosmic::iced::core::text::Wrapping::None)
                .ellipsize(cosmic::iced::core::text::Ellipsize::End(
                    cosmic::iced::core::text::EllipsizeHeightLimit::Lines(1),
                ))
                .width(Length::Fill)
                .into(),
            control.into(),
        ])
        .into()
    }

    /// Helper to find available system icons in standard locations.
    fn system_icon_names() -> Vec<String> {
        // Prefer runtime discovery using XDG_DATA_DIRS so the app works correctly
        // inside Flatpak (where icons live under /app/share) as well as on
        // a system installation (under /usr/share).
        let mut icons: Vec<String> = Vec::new();

        // Build a list of candidate data dirs from XDG_DATA_DIRS. If the
        // variable isn't set, fall back to common locations including /usr and
        // /app so we cover both host and Flatpak runtimes.
        let mut candidate_dirs: Vec<String> = Vec::new();
        if let Ok(xdg) = std::env::var("XDG_DATA_DIRS") {
            for part in xdg.split(':') {
                let part = part.trim_end_matches('/');
                if !part.is_empty() {
                    candidate_dirs.push(part.to_string());
                }
            }
        } else {
            candidate_dirs.push("/usr/share".to_string());
            candidate_dirs.push("/app/share".to_string());
        }

        for data_dir in candidate_dirs {
            let dir = format!("{}/cosmic/{}/applet-buttons", data_dir, cosmic_ext_classic_menu_applet::applet::APP_ID);
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "svg" || ext == "png" {
                            if let Ok(abs_path) = path.canonicalize() {
                                icons.push(abs_path.to_string_lossy().into_owned());
                            }
                        }
                    }
                }
            }
        }

        icons.sort();
        icons.dedup();
        icons
    }

    async fn pick_custom_icon() -> Option<PathBuf> {
        if let Ok(result) = cosmic::dialog::file_chooser::open::Dialog::new()
            .title(fl!("select-custom-icon"))
            .accept_label(fl!("select"))
            .current_filter(FileFilter::new("icon-file").glob("*.svg").glob("*.png"))
            .open_file()
            .await
        {
            let icon_pathbuf: PathBuf = PathBuf::from(result.0.uris()[0].path());
            if icon_pathbuf.exists() {
                return Some(icon_pathbuf);
            } else {
                return None;
            }
        }

        None
    }

    pub fn icon_picker(&'_ self) -> Element<'_, Message> {
        let mut icons = Self::system_icon_names();
        let icons_per_row = 3;
        let theme = cosmic::theme::active();
        let theme = theme.cosmic();

        let mut grid = cosmic::iced::widget::Column::new().spacing(theme.space_xs());

        // handle custom icon selection
        let currently_selected_icon: PathBuf = PathBuf::from(&self.config.button_icon);
        if currently_selected_icon.exists()
            && !icons.contains(&currently_selected_icon.to_string_lossy().into_owned())
        {
            icons.insert(0, currently_selected_icon.to_string_lossy().into_owned());
        }

        // Add default set of icons
        for chunk in icons.chunks(icons_per_row) {
            let mut row = cosmic::iced::widget::Row::new().spacing(theme.space_xs());
            for icon_path in chunk {
                let icon_pathbuf: PathBuf = icon_path.into();
                let icon_name = icon_pathbuf
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                row = row.push(Self::button(
                    &icon_name,
                    icon_pathbuf,
                    self.config.button_icon == *icon_path,
                    Message::ButtonIconChanged,
                ));
            }
            grid = grid.push(row);
        }

        let custom_icon_button = cosmic::widget::button::standard(fl!("select-custom-icon"))
            .on_press(Message::CustomIconSelected);

        cosmic::iced::widget::column![custom_icon_button, grid]
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .spacing(theme.space_xs())
            .into()
    }

    fn button(
        name: &String,
        icon_pathbuf: PathBuf,
        selected: bool,
        callback: impl Fn(PathBuf) -> Message,
    ) -> Element<'static, Message> {
        const ICON_THUMB_SIZE: u16 = 32;
        const ICON_NAME_TRUNC: usize = 20;

        let theme = cosmic::theme::active();
        let theme = theme.cosmic();
        let background = Background::Color(theme.palette.neutral_4.into());

        cosmic::iced::widget::column::Column::new()
            .push(
                cosmic::widget::button::custom_image_button(
                    cosmic::widget::icon::from_path(icon_pathbuf.clone())
                        .icon()
                        .size(ICON_THUMB_SIZE),
                    None,
                )
                .on_press(callback(icon_pathbuf.clone()))
                .selected(selected)
                .padding(theme.space_xs())
                // Image button's style mostly works, but it needs a background to fit the design
                .class(button::ButtonClass::Custom {
                    active: Box::new(move |focused, theme| {
                        let mut appearance = <cosmic::theme::Theme as button::Catalog>::active(
                            theme,
                            focused,
                            selected,
                            &cosmic::theme::Button::Image,
                        );
                        appearance.background = Some(background);
                        appearance
                    }),
                    disabled: Box::new(move |theme| {
                        let mut appearance = <cosmic::theme::Theme as button::Catalog>::disabled(
                            theme,
                            &cosmic::theme::Button::Image,
                        );
                        appearance.background = Some(background);
                        appearance
                    }),
                    hovered: Box::new(move |focused, theme| {
                        let mut appearance = <cosmic::theme::Theme as button::Catalog>::hovered(
                            theme,
                            focused,
                            selected,
                            &cosmic::theme::Button::Image,
                        );
                        appearance.background = Some(background);
                        appearance
                    }),
                    pressed: Box::new(move |focused, theme| {
                        let mut appearance = <cosmic::theme::Theme as button::Catalog>::pressed(
                            theme,
                            focused,
                            selected,
                            &cosmic::theme::Button::Image,
                        );
                        appearance.background = Some(background);
                        appearance
                    }),
                }),
            )
            .push(
                text::body(if name.len() > ICON_NAME_TRUNC {
                    format!("{name:.ICON_NAME_TRUNC$}...")
                } else {
                    name.into()
                })
                .width(Length::Fixed((ICON_THUMB_SIZE * 3) as _)),
            )
            .spacing(theme.space_xxs())
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
                Message::UpdateConfig(AppletConfig::default())
            }
        }
    }
}

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
    IconPicker, // 1. Add new variant
}
