// SPDX-License-Identifier: {{ license }}

use crate::fl;
use crate::layout_editor::{self, LayoutEditor};
use crate::shortcut;
use cosmic::app::context_drawer;
use cosmic::cosmic_config::CosmicConfigEntry;
use cosmic::dialog::file_chooser::FileFilter;
use cosmic::iced::keyboard::key::Named;
use cosmic::iced::keyboard::{Key, Modifiers};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{button, icon, menu, menu::{ItemWidth, ItemHeight}, nav_bar};
use cosmic::{iced::Background, widget::text, Element};
use cosmic_ext_classic_menu_applet::model::application_category::ApplicationCategory;
use cosmic_ext_classic_menu_applet::config::{
    AppView, AppletButtonStyle, AppletConfig, ContextMenuItem, FeaturedApplications, ItemSize,
    MenuPosition, Place,
};
use cosmic_ext_classic_menu_applet::layout::{
    AppListOptions, Block, BlockPadding, BlockPath, FeaturedOptions, SidePanelContent, UserName,
};
use cosmic_ext_classic_menu_applet::model::power_action::PowerAction;
use cosmic_settings_config::shortcuts::Binding;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Room the pane holding the layout editor takes around the editor: the
/// section's and the scrolling column's padding.
const LAYOUT_PANE_MARGIN: f32 = 80.0;
/// Narrowest the pane holding the layout editor gets, so the block palette
/// and the layout cards keep enough room beside a small preview.
const LAYOUT_PANE_MIN_WIDTH: f32 = 440.0;
/// Narrowest the pane with the options of the selected block gets next to
/// the layout editor.
const BLOCK_PANE_MIN_WIDTH: f32 = 360.0;

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
    /// Pages of settings listed in the nav bar, each holding its [`Page`].
    nav: nav_bar::Model,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    // Configuration data that persists between application runs.
    config: AppletConfig,
    /// Drag and drop state of the menu layout preview.
    layout_editor: LayoutEditor,
    /// The keyboard shortcut opening the menu.
    menu_shortcut: Option<Binding>,
    /// Whether the next key press becomes the shortcut.
    capturing_shortcut: bool,
    /// Whether the last key press was already used by another shortcut.
    shortcut_taken: bool,
    /// Index in the places list of the selected side panel of the entry
    /// being reordered by dragging.
    /// Separators and spacers may repeat, so the index tells them apart.
    dragged_place: Option<usize>,
    /// Index in the right click menu of the entry being reordered by
    /// dragging. Separators may repeat, so the index tells them apart.
    dragged_context_item: Option<usize>,
    /// The menu's categories, loaded in the background.
    categories: Vec<ApplicationCategory>,
    /// Whether the system can hibernate, so the power controls can offer
    /// it.
    can_hibernate: bool,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    /// Replace the whole configuration and save it, as when resetting it.
    UpdateConfig(AppletConfig),
    /// These keys of the config files changed, possibly by the applet.
    ConfigChanged(Vec<&'static str>),
    /// Start or stop waiting for the keys of the shortcut opening the menu.
    ShortcutCaptureToggled,
    ShortcutKeyPressed(Key, Modifiers),
    /// Whether Super alone opens the menu.
    ShortcutSuperToggled(bool),
    ShortcutCleared,
    LaunchUrl(String),
    LayoutEditor(layout_editor::Message),
    AppletButtonStyleChanged(usize),
    /// The block at this path got new options.
    SetBlock(BlockPath, Block),
    RemoveBlock(BlockPath),
    /// The block at this path got a new padding.
    SetBlockPadding(BlockPath, BlockPadding),
    /// Whether the row at this index is a colored bar.
    RowBannerToggled(usize, bool),
    /// The categories the menu can open on were loaded.
    CategoriesLoaded(Vec<ApplicationCategory>),
    /// Whether the system can hibernate, checked at start.
    HibernateAvailability(bool),
    /// Whether the menu opens focused, as a layer surface.
    FocusedMenuToggled(bool),
    /// Where on the screen the focused menu opens, by its index in
    /// `MenuPosition::ALL`.
    MenuPositionChanged(usize),
    /// A place of the selected side panel was turned on or off.
    PlaceToggled(Place, bool),
    /// The enabled place at this index was grabbed.
    PlaceDragStart(usize),
    /// Appends a separator or a spacer, see [`Place::is_repeatable`], to the
    /// enabled places.
    AddRepeatablePlace(Place),
    /// Removes the separator or spacer at this index.
    RemoveRepeatablePlace(usize),
    /// The cursor moved to this height over the places list.
    PlaceDragMove(f32),
    /// Left mouse button released anywhere in the window.
    PlaceDragEnd,
    /// Shows or hides an entry of the right click menu.
    ContextItemToggled(ContextMenuItem, bool),
    /// Adds a separator to the end of the right click menu.
    AddContextSeparator,
    /// Removes the separator at this index of the right click menu.
    RemoveContextSeparator(usize),
    /// Grabbed the entry at this index of the right click menu.
    ContextItemDragStart(usize),
    /// The cursor moved to this height over the right click menu's list.
    ContextItemDragMove(f32),
    ContextItemDragEnd,
    /// Shows or hides a power button of the right click menu.
    ContextPowerToggled(PowerAction, bool),
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
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
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

        let mut nav = nav_bar::Model::default();
        for page in Page::ALL {
            nav.insert()
                .text(page.title())
                .icon(icon::from_name(page.icon_name()))
                .data(page);
        }
        nav.activate_position(0);

        // Construct the app model with the runtime's core.
        let app = AppModel {
            core,
            about,
            context_page: ContextPage::default(),
            nav,
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config: AppletConfig::config(),
            layout_editor: LayoutEditor::default(),
            menu_shortcut: shortcut::current(),
            capturing_shortcut: false,
            shortcut_taken: false,
            dragged_place: None,
            dragged_context_item: None,
            categories: Vec::new(),
            can_hibernate: false,
        };

        // Loading the categories loads all apps, so it is done in the
        // background.
        let load_categories = Task::perform(
            tokio::task::spawn_blocking(
                cosmic_ext_classic_menu_applet::logic::apps::load_app_categories,
            ),
            |categories| match categories {
                Ok(categories) => cosmic::action::app(Message::CategoriesLoaded(categories)),
                Err(err) => {
                    log::error!("Failed to load categories: {err}");
                    cosmic::action::none()
                }
            },
        );

        let check_hibernate = Task::perform(
            cosmic_ext_classic_menu_applet::power_options::can_hibernate(),
            |can| cosmic::action::app(Message::HibernateAvailability(can)),
        );

        (app, Task::batch([load_categories, check_hibernate]))
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
        self.nav.activate(id);
        Task::none()
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
        // Measures the content area rather than the window, as the context
        // drawer takes room from it as well.
        cosmic::widget::responsive(move |size| self.view_sections(size.width)).into()
    }

    /// Listens for the mouse button release ending a drag and the moves
    /// resizing the menu, which may happen outside of the dragged widget.
    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        let input = if self.capturing_shortcut {
            cosmic::iced::event::listen_with(|event, _status, _window| match event {
                cosmic::iced::Event::Keyboard(cosmic::iced::keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    ..
                }) => Some(Message::ShortcutKeyPressed(key, modifiers)),
                _ => None,
            })
        } else if self.layout_editor.is_dragging() {
            cosmic::iced::event::listen_with(|event, _status, _window| match event {
                cosmic::iced::Event::Mouse(cosmic::iced::mouse::Event::ButtonReleased(
                    cosmic::iced::mouse::Button::Left,
                )) => Some(Message::LayoutEditor(layout_editor::Message::DragEnd)),
                cosmic::iced::Event::Mouse(cosmic::iced::mouse::Event::CursorMoved {
                    position,
                }) => Some(Message::LayoutEditor(layout_editor::Message::CursorMoved(
                    position,
                ))),
                _ => None,
            })
        } else if self.dragged_place.is_some() {
            cosmic::iced::event::listen_with(|event, _status, _window| match event {
                cosmic::iced::Event::Mouse(cosmic::iced::mouse::Event::ButtonReleased(
                    cosmic::iced::mouse::Button::Left,
                )) => Some(Message::PlaceDragEnd),
                _ => None,
            })
        } else if self.dragged_context_item.is_some() {
            cosmic::iced::event::listen_with(|event, _status, _window| match event {
                cosmic::iced::Event::Mouse(cosmic::iced::mouse::Event::ButtonReleased(
                    cosmic::iced::mouse::Button::Left,
                )) => Some(Message::ContextItemDragEnd),
                _ => None,
            })
        } else {
            cosmic::iced::Subscription::none()
        };

        cosmic::iced::Subscription::batch([
            input,
            // Follow changes of the config files, made by the applet (recent
            // and favorite apps) or by editing them, the same way the applet
            // does, so both show the same settings.
            cosmic::cosmic_config::config_subscription::<_, AppletConfig>(
                std::any::TypeId::of::<AppletConfig>(),
                Self::APP_ID.into(),
                AppletConfig::VERSION,
            )
            .filter_map(|update| {
                for error in &update.errors {
                    log::warn!("Failed to read applet config: {error:?}");
                }
                // The first update, with no keys, is the config just loaded.
                (!update.keys.is_empty()).then_some(Message::ConfigChanged(update.keys))
            }),
        ])
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
                self.layout_editor.select(None);

                self.save_config();

                Task::none()
            }
            Message::ConfigChanged(keys) => {
                // Read the keys from the files now rather than taking the
                // watcher's copy: that one may predate values this app wrote
                // since, which would then be undone. Nothing is written back,
                // so the applet and this app can't keep overwriting each
                // other. The layout editor drops a selected block that is no
                // longer in the layout by itself.
                if let Some(handler) = AppletConfig::config_handler() {
                    let (errors, changed) = self.config.update_keys(&handler, &keys);
                    for error in errors {
                        log::warn!("Failed to read applet config: {error:?}");
                    }
                    if !changed.is_empty() {
                        log::debug!("Applet config keys changed: {changed:?}");
                    }
                }
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
            Message::ShortcutCaptureToggled => {
                self.capturing_shortcut = !self.capturing_shortcut;
                self.shortcut_taken = false;
                Task::none()
            }
            Message::ShortcutKeyPressed(key, modifiers) => {
                if key == Key::Named(Named::Escape) && modifiers.is_empty() {
                    self.capturing_shortcut = false;
                } else if let Some(binding) = shortcut::binding_for(&key, modifiers) {
                    self.set_menu_shortcut(Some(binding));
                }
                Task::none()
            }
            Message::ShortcutSuperToggled(enabled) => {
                self.set_menu_shortcut(enabled.then(shortcut::super_key));
                Task::none()
            }
            Message::ShortcutCleared => {
                self.set_menu_shortcut(None);
                Task::none()
            }
            Message::LayoutEditor(message) => {
                let change = self.layout_editor.update(message, &self.config);

                match change {
                    Some(layout_editor::Change::Layout(layout)) => {
                        log::info!("Menu layout changed");
                        self.config.set_menu_layout(layout);
                    }
                    Some(layout_editor::Change::MenuSize(width, height)) => {
                        log::info!("Menu size changed to: {width}x{height}");
                        self.config.menu_width = width;
                        self.config.menu_height = height;
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
            Message::MenuPositionChanged(index) => {
                self.config.menu_position = MenuPosition::ALL
                    .get(index)
                    .copied()
                    .unwrap_or_default();
                self.save_config();
                Task::none()
            }
            Message::FocusedMenuToggled(focused) => {
                self.config.focused_menu = focused;
                self.save_config();
                // Without it, the menu opened by the shortcut could not be
                // typed in, so the shortcut goes with it.
                if !focused && self.menu_shortcut.is_some() {
                    self.set_menu_shortcut(None);
                }
                Task::none()
            }
            Message::SetBlock(path, block) => {
                let mut layout = self.config.layout.clone();
                if let Some(slot) = layout.get_mut(path) {
                    *slot = block;
                    self.set_layout(layout);
                }
                Task::none()
            }
            Message::SetBlockPadding(path, padding) => {
                let mut layout = self.config.layout.clone();
                if layout.get(path).is_some() {
                    layout.set_padding(path, padding);
                    self.set_layout(layout);
                }
                Task::none()
            }
            Message::RemoveBlock(path) => {
                let mut layout = self.config.layout.clone();
                if layout.get(path).is_some_and(|block| !block.kind().is_required()) {
                    layout.remove(path);
                    self.layout_editor.select(None);
                    self.set_layout(layout);
                }
                Task::none()
            }
            Message::RowBannerToggled(row, banner) => {
                let mut layout = self.config.layout.clone();
                if let Some(row) = layout.rows.get_mut(row) {
                    row.banner = banner;
                    self.set_layout(layout);
                }
                Task::none()
            }
            Message::PlaceToggled(place, enabled) => {
                self.edit_places(|places| {
                    places.retain(|p| *p != place);
                    if enabled {
                        places.push(place);
                    }
                });
                Task::none()
            }
            Message::PlaceDragStart(index) => {
                self.dragged_place = Some(index);
                Task::none()
            }
            Message::AddRepeatablePlace(place) => {
                if place.is_repeatable() {
                    self.edit_places(|places| places.push(place));
                }
                Task::none()
            }
            Message::RemoveRepeatablePlace(index) => {
                self.edit_places(|places| {
                    if places.get(index).is_some_and(|place| place.is_repeatable()) {
                        places.remove(index);
                    }
                });
                Task::none()
            }
            Message::PlaceDragMove(y) => {
                // Reorder live, so the list shows where the place will end up.
                // Enabled places come first, so the row under the cursor is
                // also the index in `places`; beyond the ends it clamps.
                let dragged = self.dragged_place;
                let Some(places) = self.selected_places_mut() else {
                    return Task::none();
                };
                if let Some(moved_to) = dragged.and_then(|from| drag_entry(places, from, y)) {
                    self.dragged_place = Some(moved_to);
                }
                Task::none()
            }
            Message::PlaceDragEnd => {
                if self.dragged_place.take().is_some() {
                    self.set_layout(self.config.layout.clone());
                }
                Task::none()
            }
            Message::ContextItemToggled(item, shown) => {
                self.config.context_menu.retain(|i| *i != item);
                if shown {
                    self.config.context_menu.push(item);
                }
                self.save_config();
                Task::none()
            }
            Message::AddContextSeparator => {
                self.config.context_menu.push(ContextMenuItem::Separator);
                self.save_config();
                Task::none()
            }
            Message::RemoveContextSeparator(index) => {
                let items = &mut self.config.context_menu;
                if items.get(index).is_some_and(|item| item.is_repeatable()) {
                    items.remove(index);
                    self.save_config();
                }
                Task::none()
            }
            Message::ContextItemDragStart(index) => {
                self.dragged_context_item = Some(index);
                Task::none()
            }
            Message::ContextItemDragMove(y) => {
                // Reordered live, and saved once dropped.
                if let Some(moved_to) = self
                    .dragged_context_item
                    .and_then(|from| drag_entry(&mut self.config.context_menu, from, y))
                {
                    self.dragged_context_item = Some(moved_to);
                }
                Task::none()
            }
            Message::ContextItemDragEnd => {
                if self.dragged_context_item.take().is_some() {
                    self.save_config();
                }
                Task::none()
            }
            Message::ContextPowerToggled(action, shown) => {
                let actions = &mut self.config.context_menu_power;
                actions.retain(|a| *a != action);
                if shown {
                    // In the usual order.
                    actions.push(action);
                    actions.sort_by_key(|a| PowerAction::ALL.iter().position(|b| b == a));
                }
                self.save_config();
                Task::none()
            }
            Message::CategoriesLoaded(categories) => {
                self.categories = categories;
                Task::none()
            }
            Message::HibernateAvailability(can) => {
                self.can_hibernate = can;
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
    /// Makes `layout` the menu's and saves it.
    fn set_layout(&mut self, layout: cosmic_ext_classic_menu_applet::layout::Layout) {
        self.config.set_menu_layout(layout);
        self.save_config();
    }

    /// The selected block and where it is.
    fn selected_block(&self) -> Option<(BlockPath, &Block)> {
        let path = self.layout_editor.selected(&self.config.layout)?;
        Some((path, self.config.layout.get(path)?))
    }

    /// Places of the selected side panel.
    fn selected_places_mut(&mut self) -> Option<&mut Vec<Place>> {
        let path = self.layout_editor.selected(&self.config.layout)?;
        match self.config.layout.get_mut(path)? {
            Block::SidePanel(options) => Some(&mut options.places),
            _ => None,
        }
    }

    /// Changes the places of the selected side panel with `edit` and saves
    /// them.
    fn edit_places(&mut self, edit: impl FnOnce(&mut Vec<Place>)) {
        if let Some(places) = self.selected_places_mut() {
            edit(places);
            self.set_layout(self.config.layout.clone());
        }
    }

    /// The categories the menu can open on, as `options` lists them.
    fn default_category_options(
        &self,
        options: &cosmic_ext_classic_menu_applet::layout::SidePanelOptions,
    ) -> Vec<ApplicationCategory> {
        cosmic_ext_classic_menu_applet::applet_menu::AppletMenu::listed_categories(
            &self.categories,
            options,
        )
        .into_iter()
        .cloned()
        .collect()
    }

    /// Options of the block selected in the layout editor.
    fn block_section(&self) -> Element<'_, Message> {
        let Some((path, block)) = self.selected_block() else {
            return cosmic::widget::settings::section()
                .title(fl!("block-options"))
                .add(text::caption(fl!("block-options-hint")))
                .into();
        };

        let mut section = cosmic::widget::settings::section()
            .title(layout_editor::block_label(block));
        for item in self.block_options(path, block) {
            section = section.add(item);
        }
        for item in Self::padding_options(path, self.config.layout.padding(path).unwrap_or_default()) {
            section = section.add(item);
        }

        let banner = self
            .config
            .layout
            .rows
            .get(path.row)
            .is_some_and(|row| row.banner);
        section = section.add(Self::setting_item(
            fl!("row-banner"),
            cosmic::widget::toggler(banner)
                .on_toggle(move |banner| Message::RowBannerToggled(path.row, banner)),
        ));

        let required = block.kind().is_required();
        let remove = button::destructive(fl!("remove-block"))
            .leading_icon(icon::from_name("edit-delete-symbolic"))
            .on_press_maybe((!required).then_some(Message::RemoveBlock(path)));
        section
            .add(cosmic::iced::widget::row![
                cosmic::widget::Space::new().width(Length::Fill),
                remove
            ])
            .into()
    }

    /// Settings rows for the options of `block`.
    fn block_options(&self, path: BlockPath, block: &Block) -> Vec<Element<'_, Message>> {
        /// Sends `block` with `set` applied to its options.
        fn on<O: Clone + 'static, T>(
            path: BlockPath,
            options: &O,
            wrap: fn(O) -> Block,
            set: impl Fn(&mut O, T) + 'static,
        ) -> impl Fn(T) -> Message + 'static {
            let options = options.clone();
            move |value| {
                let mut options = options.clone();
                set(&mut options, value);
                Message::SetBlock(path, wrap(options))
            }
        }
        let toggle = |title: String, value: bool, on_toggle: Box<dyn Fn(bool) -> Message>| {
            Self::setting_item(title, cosmic::widget::toggler(value).on_toggle(on_toggle))
        };

        let mut items = Vec::new();
        match block {
            Block::User(options) => {
                items.push(Self::setting_item(
                    fl!("user-name"),
                    cosmic::widget::dropdown(
                        vec![fl!("username-prefered"), fl!("realname-prefered")],
                        Some(match options.name {
                            UserName::Username => 0,
                            UserName::FullName => 1,
                        }),
                        on(path, options, Block::User, |o, index: usize| {
                            o.name = if index == 1 {
                                UserName::FullName
                            } else {
                                UserName::Username
                            };
                        }),
                    ),
                ));
                items.push(toggle(
                    fl!("user-show-avatar"),
                    options.show_avatar,
                    Box::new(on(path, options, Block::User, |o, v| o.show_avatar = v)),
                ));
            }
            Block::Search(options) => {
                items.push(Self::setting_item(
                    fl!("search-size"),
                    cosmic::widget::dropdown(
                        vec![
                            fl!("item-size-small"),
                            fl!("item-size-medium"),
                            fl!("item-size-large"),
                        ],
                        ItemSize::ALL.iter().position(|size| *size == options.size),
                        on(path, options, Block::Search, |o, index: usize| {
                            o.size = ItemSize::ALL.get(index).copied().unwrap_or_default();
                        }),
                    ),
                ));
                items.push(
                    cosmic::widget::settings::item::builder(fl!("search-launcher-results"))
                        .description(fl!("search-launcher-results-description"))
                        .control(cosmic::widget::toggler(options.launcher_results).on_toggle(
                            on(path, options, Block::Search, |o, v| o.launcher_results = v),
                        ))
                        .into(),
                );
            }
            Block::AppList(options) => items.extend(self.app_list_options(path, options)),
            Block::SidePanel(options) => {
                items.push(Self::setting_item(
                    fl!("side-panel-content"),
                    cosmic::widget::dropdown(
                        vec![fl!("layout-categories"), fl!("layout-places")],
                        Some(match options.content {
                            SidePanelContent::Categories => 0,
                            SidePanelContent::Places => 1,
                        }),
                        on(path, options, Block::SidePanel, |o, index: usize| {
                            o.content = if index == 1 {
                                SidePanelContent::Places
                            } else {
                                SidePanelContent::Categories
                            };
                        }),
                    ),
                ));
                items.push(toggle(
                    fl!("side-panel-hide-while-searching"),
                    options.hide_while_searching,
                    Box::new(on(path, options, Block::SidePanel, |o, v| {
                        o.hide_while_searching = v;
                    })),
                ));
                items.push(toggle(
                    fl!("side-panel-tinted"),
                    options.tinted,
                    Box::new(on(path, options, Block::SidePanel, |o, v| o.tinted = v)),
                ));
                match options.content {
                    SidePanelContent::Categories => {
                        items.push(toggle(
                            fl!("categories-recently-used"),
                            options.show_recently_used,
                            Box::new(on(path, options, Block::SidePanel, |o, v| {
                                o.show_recently_used = v;
                            })),
                        ));
                        items.push(toggle(
                            fl!("categories-favorites"),
                            options.show_favorites,
                            Box::new(on(path, options, Block::SidePanel, |o, v| {
                                o.show_favorites = v;
                            })),
                        ));
                        // Shown once the categories are loaded, and where
                        // the menu opens on them.
                        let categories = self.default_category_options(options);
                        let is_default = self.config.categories() == Some(options)
                            && self.config.shows_default_category();
                        if is_default && !categories.is_empty() {
                            let ids: Vec<String> =
                                categories.iter().map(|c| c.id().to_owned()).collect();
                            let selected =
                                ids.iter().position(|id| *id == options.default_category);
                            items.push(Self::setting_item(
                                fl!("default-category"),
                                cosmic::widget::dropdown(
                                    categories
                                        .iter()
                                        .map(|c| c.get_display_name())
                                        .collect::<Vec<_>>(),
                                    selected,
                                    on(path, options, Block::SidePanel, move |o, index: usize| {
                                        if let Some(id) = ids.get(index) {
                                            o.default_category = id.clone();
                                        }
                                    }),
                                ),
                            ));
                        }
                    }
                    SidePanelContent::Places => {
                        let add_separator = button::standard(fl!("add-separator"))
                            .leading_icon(icon::from_name("list-add-symbolic"))
                            .on_press(Message::AddRepeatablePlace(Place::Separator));
                        let add_spacer = button::standard(fl!("add-spacer"))
                            .leading_icon(icon::from_name("list-add-symbolic"))
                            .on_press(Message::AddRepeatablePlace(Place::Spacer));
                        items.push(self.places_list(&options.places));
                        items.push(
                            cosmic::iced::widget::row![
                                cosmic::widget::Space::new().width(Length::Fill),
                                add_spacer,
                                add_separator
                            ]
                            .spacing(cosmic::theme::active().cosmic().space_xs())
                            .into(),
                        );
                    }
                }
            }
            Block::Power(options) => {
                for action in PowerAction::ALL
                    .into_iter()
                    .filter(|action| action.is_available(self.can_hibernate))
                {
                    let shown = options.actions.contains(&action);
                    items.push(toggle(
                        power_action_name(action),
                        shown,
                        Box::new(on(path, options, Block::Power, move |o, shown: bool| {
                            o.actions.retain(|a| *a != action);
                            if shown {
                                // In the usual order.
                                o.actions.push(action);
                                o.actions.sort_by_key(|a| {
                                    PowerAction::ALL.iter().position(|b| b == a)
                                });
                            }
                        })),
                    ));
                }
                items.push(toggle(
                    fl!("power-vertical"),
                    options.vertical,
                    Box::new(on(path, options, Block::Power, |o, v| o.vertical = v)),
                ));
                items.push(toggle(
                    fl!("power-tooltips"),
                    options.show_tooltips,
                    Box::new(on(path, options, Block::Power, |o, v| o.show_tooltips = v)),
                ));
            }
            Block::Featured(options) => items.extend(Self::featured_options(path, options)),
            Block::Spacer | Block::Divider => {
                items.push(text::caption(fl!("block-no-options")).into());
            }
        }
        items
    }

    /// Settings rows for the padding of the block at `path`, one per side.
    fn padding_options(path: BlockPath, padding: BlockPadding) -> Vec<Element<'static, Message>> {
        let side = |title: String, value: u16, set: fn(&mut BlockPadding, u16)| {
            Self::setting_item(
                title.clone(),
                cosmic::widget::spin_button(
                    format!("{value} px"),
                    title,
                    value,
                    1,
                    *BlockPadding::RANGE.start(),
                    *BlockPadding::RANGE.end(),
                    move |value| {
                        let mut padding = padding;
                        set(&mut padding, value);
                        Message::SetBlockPadding(path, padding)
                    },
                ),
            )
        };
        vec![
            side(fl!("padding-top"), padding.top, |p, v| p.top = v),
            side(fl!("padding-bottom"), padding.bottom, |p, v| p.bottom = v),
            side(fl!("padding-left"), padding.left, |p, v| p.left = v),
            side(fl!("padding-right"), padding.right, |p, v| p.right = v),
        ]
    }

    fn app_list_options(
        &self,
        path: BlockPath,
        options: &AppListOptions,
    ) -> Vec<Element<'_, Message>> {
        fn set(
            path: BlockPath,
            options: &AppListOptions,
            set: impl Fn(&mut AppListOptions, bool) + 'static,
        ) -> impl Fn(bool) -> Message + 'static {
            let options = options.clone();
            move |value| {
                let mut options = options.clone();
                set(&mut options, value);
                Message::SetBlock(path, Block::AppList(options))
            }
        }
        let toggle = |title: String, value: bool, on_toggle: Box<dyn Fn(bool) -> Message>| {
            Self::setting_item(title, cosmic::widget::toggler(value).on_toggle(on_toggle))
        };
        let count = |title: String, value: u16, on_change: Box<dyn Fn(u16) -> Message>| {
            Self::setting_item(
                title.clone(),
                cosmic::widget::spin_button(
                    value.to_string(),
                    title,
                    AppListOptions::clamp_count(value),
                    1,
                    *AppListOptions::COUNT_RANGE.start(),
                    *AppListOptions::COUNT_RANGE.end(),
                    on_change,
                ),
            )
        };

        let view_options = options.clone();
        let size_options = options.clone();
        let favorites_options = options.clone();
        let recents_options = options.clone();
        let mut items = vec![
            Self::setting_item(
                fl!("app-view-style"),
                cosmic::widget::dropdown(
                    vec![fl!("app-view-column"), fl!("app-view-grid")],
                    Some(match options.view {
                        AppView::Column => 0,
                        AppView::Grid => 1,
                    }),
                    move |index| {
                        let mut options = view_options.clone();
                        options.view = if index == 1 { AppView::Grid } else { AppView::Column };
                        Message::SetBlock(path, Block::AppList(options))
                    },
                ),
            ),
            Self::setting_item(
                fl!("item-size"),
                cosmic::widget::dropdown(
                    vec![
                        fl!("item-size-small"),
                        fl!("item-size-medium"),
                        fl!("item-size-large"),
                    ],
                    ItemSize::ALL.iter().position(|size| *size == options.item_size),
                    move |index| {
                        let mut options = size_options.clone();
                        options.item_size = ItemSize::ALL.get(index).copied().unwrap_or_default();
                        Message::SetBlock(path, Block::AppList(options))
                    },
                ),
            ),
            toggle(
                match options.view {
                    AppView::Column => fl!("show-description"),
                    AppView::Grid => fl!("show-description-on-hover"),
                },
                options.show_description,
                Box::new(set(path, options, |o, v| o.show_description = v)),
            ),
            toggle(
                fl!("app-list-heading"),
                options.show_heading,
                Box::new(set(path, options, |o, v| o.show_heading = v)),
            ),
            toggle(
                fl!("app-list-favorites"),
                options.favorites,
                Box::new(set(path, options, |o, v| o.favorites = v)),
            ),
        ];
        if options.favorites {
            items.push(count(
                fl!("app-list-favorites-count"),
                options.favorites_count,
                Box::new(move |value| {
                    let mut options = favorites_options.clone();
                    options.favorites_count = value;
                    Message::SetBlock(path, Block::AppList(options))
                }),
            ));
        }
        items.push(toggle(
            fl!("app-list-recents"),
            options.recents,
            Box::new(set(path, options, |o, v| o.recents = v)),
        ));
        if options.recents {
            items.push(count(
                fl!("app-list-recents-count"),
                options.recents_count,
                Box::new(move |value| {
                    let mut options = recents_options.clone();
                    options.recents_count = value;
                    Message::SetBlock(path, Block::AppList(options))
                }),
            ));
        }
        if options.has_home() {
            items.push(
                cosmic::widget::settings::item::builder(fl!("start-page"))
                    .description(fl!("start-page-description"))
                    .control(
                        cosmic::widget::toggler(options.start_page)
                            .on_toggle(set(path, options, |o, v| o.start_page = v)),
                    )
                    .into(),
            );
        }
        items
    }

    fn featured_options(path: BlockPath, options: &FeaturedOptions) -> Vec<Element<'static, Message>> {
        let change = |f: fn(&mut FeaturedOptions, bool)| {
            let options = options.clone();
            move |value| {
                let mut options = options.clone();
                f(&mut options, value);
                Message::SetBlock(path, Block::Featured(options))
            }
        };
        let source_options = options.clone();
        let count_options = options.clone();

        let mut items: Vec<Element<'static, Message>> = vec![
            Self::setting_item(
                fl!("featured-applications"),
                cosmic::widget::dropdown(
                    vec![fl!("featured-recently-used"), fl!("featured-favorites")],
                    Some(match options.source {
                        FeaturedApplications::RecentlyUsed => 0,
                        FeaturedApplications::Favorites => 1,
                    }),
                    move |index| {
                        let mut options = source_options.clone();
                        options.source = if index == 1 {
                            FeaturedApplications::Favorites
                        } else {
                            FeaturedApplications::RecentlyUsed
                        };
                        Message::SetBlock(path, Block::Featured(options))
                    },
                ),
            ),
            Self::setting_item(
                fl!("featured-vertical"),
                cosmic::widget::toggler(options.vertical)
                    .on_toggle(change(|o, v| o.vertical = v)),
            ),
            Self::setting_item(
                fl!("featured-count"),
                cosmic::widget::spin_button(
                    options.count().to_string(),
                    fl!("featured-count"),
                    options.count() as u16,
                    1,
                    *FeaturedOptions::COUNT_RANGE.start(),
                    *FeaturedOptions::COUNT_RANGE.end(),
                    move |value| {
                        let mut options = count_options.clone();
                        options.count = value;
                        Message::SetBlock(path, Block::Featured(options))
                    },
                ),
            ),
        ];
        if !options.vertical {
            items.push(Self::setting_item(
                fl!("featured-heading"),
                cosmic::widget::toggler(options.show_heading)
                    .on_toggle(change(|o, v| o.show_heading = v)),
            ));
        }
        items.push(Self::setting_item(
            fl!("featured-hide-while-searching"),
            cosmic::widget::toggler(options.hide_while_searching)
                .on_toggle(change(|o, v| o.hide_while_searching = v)),
        ));
        items
    }

    /// Places with toggles; enabled ones first, in order, reordered by
    /// dragging their rows.
    fn places_list(&self, places: &[Place]) -> Element<'_, Message> {
        let enabled = places;
        let disabled = Place::ALL.into_iter().filter(|p| !enabled.contains(p));

        // Enabled entries know their index in `places`; disabled ones have none.
        let entries = enabled
            .iter()
            .copied()
            .enumerate()
            .map(|(index, place)| (place, Some(index)))
            .chain(disabled.map(|place| (place, None)));

        let rows = entries.map(|(place, index)| self.place_item(place, index)).collect();
        reorderable_list(rows, self.dragged_place.is_some(), Message::PlaceDragMove)
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
                Self::remove_place_button(index, fl!("remove-separator")),
            ]),
            // Empty room, so it shows as a dashed outline with its name.
            (Place::Spacer, Some(index)) => cosmic::widget::settings::item_row(vec![
                handle,
                cosmic::widget::container(text::caption(place.display_name()))
                    .center(Length::Fill)
                    .class(cosmic::theme::Container::custom(|theme| {
                        let cosmic = theme.cosmic();
                        cosmic::widget::container::Style {
                            border: cosmic::iced::Border {
                                color: cosmic.palette.neutral_6.into(),
                                width: 1.0,
                                radius: cosmic.radius_s().into(),
                            },
                            ..Default::default()
                        }
                    }))
                    .into(),
                Self::remove_place_button(index, fl!("remove-spacer")),
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

        reorderable_row(
            row,
            index.map(Message::PlaceDragStart),
            dragged,
            self.dragged_place.is_some(),
        )
    }

    /// The entries of the menu opened by right clicking the button, and its
    /// power buttons.
    fn context_menu_section(&self) -> Element<'_, Message> {
        let items = &self.config.context_menu;
        let hidden = ContextMenuItem::ALL.into_iter().filter(|item| !items.contains(item));
        // Shown entries know their index in `items`, so they come first,
        // in order; hidden ones have none.
        let rows = items
            .iter()
            .copied()
            .enumerate()
            .map(|(index, item)| (item, Some(index)))
            .chain(hidden.map(|item| (item, None)))
            .map(|(item, index)| self.context_menu_row(item, index))
            .collect();
        let list = reorderable_list(
            rows,
            self.dragged_context_item.is_some(),
            Message::ContextItemDragMove,
        );

        let add_separator = button::standard(fl!("add-separator"))
            .leading_icon(icon::from_name("list-add-symbolic"))
            .on_press(Message::AddContextSeparator);

        let mut section = cosmic::widget::settings::section()
            .title(fl!("context-menu"))
            .add(text::caption(fl!("context-menu-description")))
            .add(list)
            .add(
                cosmic::iced::widget::row![
                    cosmic::widget::Space::new().width(Length::Fill),
                    add_separator
                ],
            );

        if items.contains(&ContextMenuItem::Power) {
            section = section.add(text::heading(fl!("context-power-buttons")));
            for action in PowerAction::ALL
                .into_iter()
                .filter(|action| action.is_available(self.can_hibernate))
            {
                section = section.add(
                    cosmic::widget::settings::item::builder(power_action_name(action)).toggler(
                        self.config.context_menu_power.contains(&action),
                        move |shown| Message::ContextPowerToggled(action, shown),
                    ),
                );
            }
        }
        section.into()
    }

    /// Row showing or hiding an entry of the right click menu, or removing a
    /// separator, grabbed to reorder it if shown, at `index`.
    fn context_menu_row(&self, item: ContextMenuItem, index: Option<usize>) -> Element<'_, Message> {
        let dragged = index.is_some() && self.dragged_context_item == index;
        let handle: Element<'_, Message> = if index.is_some() {
            icon::from_name("open-menu-symbolic").size(16).into()
        } else {
            cosmic::widget::Space::new().width(16).into()
        };

        let row = match (item, index) {
            (ContextMenuItem::Separator, Some(index)) => cosmic::widget::settings::item_row(vec![
                handle,
                cosmic::widget::container(cosmic::widget::divider::horizontal::heavy())
                    .center_y(Length::Fill)
                    .width(Length::Fill)
                    .into(),
                cosmic::widget::tooltip(
                    cosmic::widget::button::icon(icon::from_name("edit-delete-symbolic"))
                        .on_press(Message::RemoveContextSeparator(index)),
                    text::caption(fl!("remove-separator")),
                    cosmic::widget::tooltip::Position::Left,
                )
                .into(),
            ]),
            _ => cosmic::widget::settings::item_row(vec![
                handle,
                icon::from_name(item.icon_name()).size(16).into(),
                text::body(item.display_name())
                    .wrapping(cosmic::iced::core::text::Wrapping::None)
                    .ellipsize(cosmic::iced::core::text::Ellipsize::End(
                        cosmic::iced::core::text::EllipsizeHeightLimit::Lines(1),
                    ))
                    .width(Length::Fill)
                    .into(),
                cosmic::widget::toggler(index.is_some())
                    .on_toggle(move |shown| Message::ContextItemToggled(item, shown))
                    .into(),
            ]),
        };

        reorderable_row(
            row,
            index.map(Message::ContextItemDragStart),
            dragged,
            self.dragged_context_item.is_some(),
        )
    }


    /// Button removing the separator or spacer at `index` of the places.
    fn remove_place_button(index: usize, label: String) -> Element<'static, Message> {
        cosmic::widget::tooltip(
            cosmic::widget::button::icon(icon::from_name("edit-delete-symbolic"))
                .on_press(Message::RemoveRepeatablePlace(index)),
            text::caption(label),
            cosmic::widget::tooltip::Position::Left,
        )
        .into()
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

    /// Width of the pane holding the layout editor next to the options of
    /// the selected block. It follows the size of the menu, so a small menu
    /// leaves more room to its options.
    fn layout_pane_width(editor_width: f32) -> f32 {
        (editor_width + LAYOUT_PANE_MARGIN).max(LAYOUT_PANE_MIN_WIDTH)
    }

    /// The settings fitting `width`, with the layout editor next to the
    /// options of the selected block if both fit side by side, or above them.
    fn view_sections(&self, width: f32) -> Element<'_, Message> {
        let page = self.nav.active_data::<Page>().copied().unwrap_or_default();
        match page {
            Page::Layout => {
                let layout_editor = self
                    .layout_editor
                    .view(&self.config)
                    .map(Message::LayoutEditor);
                let layout_section: Element<'_, Message> = cosmic::widget::settings::section()
                    .title(fl!("layout"))
                    .add(layout_editor)
                    .into();
                // Decided on the saved size of the menu, so resizing it does
                // not move the editor away from under the cursor.
                let saved_pane_width =
                    Self::layout_pane_width(LayoutEditor::saved_width(&self.config));
                if width >= saved_pane_width + BLOCK_PANE_MIN_WIDTH {
                    let pane_width =
                        Self::layout_pane_width(self.layout_editor.width(&self.config));
                    cosmic::iced::widget::row![
                        Self::scrolling_sections(vec![layout_section], Length::Fixed(pane_width)),
                        Self::scrolling_sections(vec![self.block_section()], Length::Fill),
                    ]
                    .height(Length::Fill)
                    .into()
                } else {
                    Self::scrolling_sections(
                        vec![layout_section, self.block_section()],
                        Length::Fill,
                    )
                }
            }
            Page::Button => Self::scrolling_sections(vec![self.button_section()], Length::Fill),
            Page::ContextMenu => {
                Self::scrolling_sections(vec![self.context_menu_section()], Length::Fill)
            }
            Page::Shortcut => {
                Self::scrolling_sections(vec![self.shortcut_section()], Length::Fill)
            }
            Page::Experimental => {
                Self::scrolling_sections(vec![self.experimental_section()], Length::Fill)
            }
        }
    }

    /// Look of the button opening the menu from the panel.
    fn button_section(&self) -> Element<'_, Message> {
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
        let button_label =
            cosmic::widget::text_input(fl!("button-label-placeholder"), &self.config.button_label)
                .on_input(Message::ButtonLabelChanged)
                .width(Length::Fixed(200.0));
        let button_icon = cosmic::widget::button::text(fl!("button-icon-placeholder"))
            .on_press(Message::OpenIconPicker);

        cosmic::widget::settings::section()
            .title(fl!("general"))
            .add(Self::setting_item(fl!("applet-button-style"), applet_button_style))
            .add(Self::setting_item(fl!("button-label"), button_label))
            .add(Self::setting_item(fl!("button-icon"), button_icon))
            .into()
    }

    /// `sections` in a column that scrolls if needed. The extra right padding
    /// keeps the scrollbar off the sections.
    fn scrolling_sections(
        sections: Vec<Element<'_, Message>>,
        width: Length,
    ) -> Element<'_, Message> {
        cosmic::widget::scrollable(cosmic::widget::settings::view_column(sections).padding(
            cosmic::iced::Padding {
                top: 5.0,
                bottom: 5.0,
                left: 10.0,
                right: 20.0,
            },
        ))
        .width(width)
        .height(Length::Fill)
        .into()
    }

    /// Makes `binding` the shortcut opening the menu.
    fn set_menu_shortcut(&mut self, binding: Option<Binding>) {
        self.capturing_shortcut = false;
        self.shortcut_taken = false;
        match shortcut::set(binding.clone()) {
            Ok(()) => self.menu_shortcut = binding,
            Err(shortcut::Error::Taken) => self.shortcut_taken = true,
            Err(shortcut::Error::Config(error)) => {
                log::error!("Failed to write the keyboard shortcut: {error}");
            }
        }
    }

    /// Options that may not work well yet.
    fn experimental_section(&self) -> Element<'_, Message> {
        let focused = self.config.focused_menu;
        // Only the focused menu can open anywhere on the screen; a popup
        // opens at the button, so without it the position is only named.
        let position = cosmic::widget::settings::item::builder(fl!("menu-position"));
        let position: Element<'_, Message> = if focused {
            position
                .description(fl!("menu-position-description"))
                .control(cosmic::widget::dropdown(
                    MenuPosition::ALL.map(menu_position_name).to_vec(),
                    MenuPosition::ALL
                        .iter()
                        .position(|position| *position == self.config.menu_position),
                    Message::MenuPositionChanged,
                ))
                .into()
        } else {
            position
                .description(fl!("menu-position-needs-focused-menu"))
                .control(text::body(menu_position_name(self.config.menu_position)))
                .into()
        };

        cosmic::widget::settings::section()
            .title(fl!("experimental"))
            .add(
                cosmic::widget::settings::item::builder(fl!("focused-menu"))
                    .description(fl!("focused-menu-description"))
                    .toggler(focused, Message::FocusedMenuToggled),
            )
            .add(position)
            .into()
    }

    fn shortcut_section(&self) -> Element<'_, Message> {
        let label = if self.capturing_shortcut {
            fl!("keyboard-shortcut-press")
        } else {
            self.menu_shortcut
                .as_ref()
                .map_or_else(|| fl!("keyboard-shortcut-none"), ToString::to_string)
        };
        // Only the focused menu can be typed in when opened by the shortcut.
        let enabled = self.config.focused_menu;
        let keys = button::standard(label)
            .class(if self.capturing_shortcut {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Standard
            })
            .on_press_maybe(enabled.then_some(Message::ShortcutCaptureToggled));
        let clear = self.menu_shortcut.is_some().then(|| {
            button::icon(icon::from_name("edit-clear-symbolic"))
                .on_press_maybe(enabled.then_some(Message::ShortcutCleared))
        });
        let keys = cosmic::iced::widget::row![keys]
            .push_maybe(clear)
            .spacing(cosmic::theme::active().cosmic().space_xxs())
            .align_y(Alignment::Center);

        let is_super = self
            .menu_shortcut
            .as_ref()
            .is_some_and(Binding::is_super);

        cosmic::widget::settings::section()
            .title(fl!("keyboard-shortcut"))
            .add(Self::setting_item(fl!("keyboard-shortcut-keys"), keys))
            .add(Self::setting_item(
                fl!("keyboard-shortcut-super"),
                cosmic::widget::toggler(is_super)
                    .on_toggle_maybe(enabled.then_some(Message::ShortcutSuperToggled)),
            ))
            .add_maybe((!enabled).then(|| -> Element<'_, Message> {
                text::caption(fl!("keyboard-shortcut-needs-focused-menu")).into()
            }))
            .add_maybe(self.shortcut_taken.then(|| -> Element<'_, Message> {
                text::caption(fl!("keyboard-shortcut-taken")).into()
            }))
            .into()
    }

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
            let icon_pathbuf: PathBuf = PathBuf::from(result.url().path());
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

/// Name of a power action, as shown in the settings.
fn menu_position_name(position: MenuPosition) -> String {
    match position {
        MenuPosition::Auto => fl!("menu-position-auto"),
        MenuPosition::TopLeft => fl!("menu-position-top-left"),
        MenuPosition::Top => fl!("menu-position-top"),
        MenuPosition::TopRight => fl!("menu-position-top-right"),
        MenuPosition::Left => fl!("menu-position-left"),
        MenuPosition::Center => fl!("menu-position-center"),
        MenuPosition::Right => fl!("menu-position-right"),
        MenuPosition::BottomLeft => fl!("menu-position-bottom-left"),
        MenuPosition::Bottom => fl!("menu-position-bottom"),
        MenuPosition::BottomRight => fl!("menu-position-bottom-right"),
    }
}

fn power_action_name(action: PowerAction) -> String {
    match action {
        PowerAction::Logout => fl!("power-logout"),
        PowerAction::Suspend => fl!("power-suspend"),
        PowerAction::Hibernate => fl!("power-hibernate"),
        PowerAction::Lock => fl!("power-lock"),
        PowerAction::Reboot => fl!("power-reboot"),
        PowerAction::Shutdown => fl!("power-shutdown"),
    }
}

/// Moves the entry at `from` of `entries` to the row at height `y` of its
/// list, clamped to the list, returning where it went if it moved.
fn drag_entry<T>(entries: &mut Vec<T>, from: usize, y: f32) -> Option<usize> {
    if from >= entries.len() {
        return None;
    }
    let to = ((y / PLACE_ROW_PITCH).max(0.0) as usize).min(entries.len() - 1);
    if to == from {
        return None;
    }
    let entry = entries.remove(from);
    entries.insert(to, entry);
    Some(to)
}

/// `rows` divided by lines, reordered by dragging. The whole list tracks
/// the cursor while `dragging`, so a row can be dragged past the first and
/// last ones.
fn reorderable_list<'a>(
    rows: Vec<Element<'a, Message>>,
    dragging: bool,
    on_move: fn(f32) -> Message,
) -> Element<'a, Message> {
    let mut list = cosmic::iced::widget::column![];
    for (index, row) in rows.into_iter().enumerate() {
        if index > 0 {
            list = list.push(
                cosmic::widget::container(cosmic::widget::divider::horizontal::default())
                    .height(Length::Fixed(PLACE_DIVIDER_HEIGHT)),
            );
        }
        list = list.push(row);
    }

    let mut area = cosmic::widget::mouse_area(list);
    if dragging {
        area = area
            .on_move(move |point| on_move(point.y))
            .interaction(cosmic::iced::mouse::Interaction::Grabbing);
    }
    area.into()
}

/// A row of [`reorderable_list`], highlighted if `dragged`. Rows with a
/// message `on_grab` can be grabbed anywhere but on their controls, which
/// handle their own clicks.
fn reorderable_row<'a>(
    row: impl Into<Element<'a, Message>>,
    on_grab: Option<Message>,
    dragged: bool,
    dragging: bool,
) -> Element<'a, Message> {
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

    match on_grab {
        Some(on_grab) => cosmic::widget::mouse_area(row)
            .on_press(on_grab)
            .interaction(if dragging {
                cosmic::iced::mouse::Interaction::Grabbing
            } else {
                cosmic::iced::mouse::Interaction::Grab
            })
            .into(),
        None => row.into(),
    }
}

/// A page of settings, listed in the nav bar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
    /// The menu's layout editor and the options of its selected block.
    #[default]
    Layout,
    /// The button opening the menu from the panel.
    Button,
    /// The menu shown by right clicking the button.
    ContextMenu,
    /// The keyboard shortcut opening the menu.
    Shortcut,
    /// Options that may not work well yet.
    Experimental,
}

impl Page {
    /// All pages, in the order the nav bar lists them.
    const ALL: [Self; 5] = [
        Self::Layout,
        Self::Button,
        Self::ContextMenu,
        Self::Shortcut,
        Self::Experimental,
    ];

    fn title(self) -> String {
        match self {
            Self::Layout => fl!("layout"),
            Self::Button => fl!("general"),
            Self::ContextMenu => fl!("context-menu"),
            Self::Shortcut => fl!("keyboard-shortcut"),
            Self::Experimental => fl!("experimental"),
        }
    }

    fn icon_name(self) -> &'static str {
        match self {
            Self::Layout => "application-menu-symbolic",
            Self::Button => "preferences-panel-symbolic",
            Self::ContextMenu => "input-mouse-symbolic",
            Self::Shortcut => "input-keyboard-symbolic",
            Self::Experimental => "applications-science-symbolic",
        }
    }
}
