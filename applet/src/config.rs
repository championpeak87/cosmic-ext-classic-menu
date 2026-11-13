// SPDX-License-Identifier: GPL-3.0-only

use std::sync::LazyLock;

use crate::fl;
use cosmic::{
    cosmic_config::{
        self, cosmic_config_derive::CosmicConfigEntry, Config, ConfigGet, ConfigSet,
        CosmicConfigEntry,
    },
    Application,
};
use serde::{Deserialize, Serialize};

use crate::layout::{AppListOptions, Layout, SearchOptions, SidePanelOptions};
use crate::model::application_category::ApplicationCategory;
pub use crate::model::context_menu_item::ContextMenuItem;
pub use crate::model::place::Place;
use crate::model::power_action::PowerAction;

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
#[id = "cosmic-ext-classic-menu"]
pub struct AppletConfig {
    /// The blocks the menu is made of.
    pub layout: Layout,
    /// The last layout made by the user, kept while a preset is picked, so
    /// it can be picked again.
    pub custom_layout: Option<Layout>,
    pub applet_button_style: AppletButtonStyle,
    pub button_label: String,
    pub button_icon: String,
    pub recent_applications: Vec<RecentApplication>,
    /// Width of the menu popup in pixels, see [`AppletConfig::menu_size`].
    pub menu_width: u16,
    /// Height of the menu popup in pixels, see [`AppletConfig::menu_size`].
    pub menu_height: u16,
    /// Ids of apps added to favorites, in order.
    pub favorite_applications: Vec<String>,
    /// Whether the menu opens as a layer surface, taking the keyboard at
    /// once also when opened by the shortcut, rather than as a popup, which
    /// opens at the button and is frosted, see `crate::menu_surface`.
    /// Experimental, as layer surfaces of applets are not fully supported by
    /// cosmic-panel.
    pub focused_menu: bool,
    /// Where on the screen the focused menu opens, see [`MenuPosition`].
    pub menu_position: MenuPosition,
    /// Entries of the menu opened by right clicking the button, in order.
    pub context_menu: Vec<ContextMenuItem>,
    /// Buttons of [`ContextMenuItem::Power`], in order.
    pub context_menu_power: Vec<PowerAction>,
}

impl Default for AppletConfig {
    fn default() -> Self {
        AppletConfig {
            layout: Layout::default(),
            custom_layout: None,
            applet_button_style: AppletButtonStyle::default(),
            button_label: fl!("menu-label").to_owned(),
            button_icon: format!("/usr/share/cosmic/{}/applet-buttons/default.svg", crate::applet::Applet::APP_ID).to_owned(),
            recent_applications: vec![],
            menu_width: Self::DEFAULT_MENU_WIDTH,
            menu_height: Self::DEFAULT_MENU_HEIGHT,
            favorite_applications: vec![],
            focused_menu: false,
            menu_position: MenuPosition::default(),
            context_menu: ContextMenuItem::DEFAULT.to_vec(),
            context_menu_power: PowerAction::DEFAULT.to_vec(),
        }
    }
}

/// Options of an app list, for menus that have none.
static DEFAULT_APP_LIST: LazyLock<AppListOptions> = LazyLock::new(AppListOptions::default);

impl AppletConfig {
    pub const DEFAULT_MENU_WIDTH: u16 = 600;
    pub const DEFAULT_MENU_HEIGHT: u16 = 700;
    /// Bounds of `menu_width`. The upper one is the widest menu whose
    /// preview fits the settings window.
    pub const MENU_WIDTH_RANGE: std::ops::RangeInclusive<u16> = 500..=960;
    /// Bounds of `menu_height`.
    pub const MENU_HEIGHT_RANGE: std::ops::RangeInclusive<u16> = 400..=1000;

    /// Width and height of the menu popup in pixels.
    pub fn menu_size(&self) -> (u16, u16) {
        (
            self.menu_width
                .clamp(*Self::MENU_WIDTH_RANGE.start(), *Self::MENU_WIDTH_RANGE.end()),
            self.menu_height
                .clamp(*Self::MENU_HEIGHT_RANGE.start(), *Self::MENU_HEIGHT_RANGE.end()),
        )
    }

    pub fn config_handler() -> Option<Config> {
        Config::new(crate::applet::Applet::APP_ID, 1).ok()
    }

    pub fn config() -> AppletConfig {
        match Self::config_handler() {
            Some(config_handler) => {
                let mut config = AppletConfig::get_entry(&config_handler)
                    .unwrap_or_else(|(_errs, config)| config);
                config.migrate(&config_handler);
                config
            }
            None => AppletConfig::default(),
        }
    }

    /// Builds the layout from the keys of versions before the menu was made
    /// of blocks, if the config has those but no layout yet, and stores it.
    fn migrate(&mut self, config_handler: &Config) {
        if config_handler.get::<Layout>("layout").is_ok() {
            return;
        }
        let Some(layout) = legacy::layout(config_handler) else {
            return;
        };
        log::info!("Migrating the menu layout to blocks");
        if let Err(err) = config_handler.set("layout", &layout) {
            log::error!("Failed to write the migrated layout: {err}");
        }
        self.layout = layout;
    }

    /// Makes `layout` the menu's, keeping it to be picked again if it is
    /// none of the presets.
    pub fn set_menu_layout(&mut self, layout: Layout) {
        if MenuLayout::matching(&layout).is_none() {
            self.custom_layout = Some(layout.clone());
        }
        self.layout = layout;
    }

    /// Options of the app list.
    pub fn app_list(&self) -> &AppListOptions {
        self.layout.app_list().unwrap_or(&DEFAULT_APP_LIST)
    }

    pub fn search(&self) -> Option<&SearchOptions> {
        self.layout.search()
    }

    /// The side panel showing categories, which tells which one the menu
    /// opens on.
    pub fn categories(&self) -> Option<&SidePanelOptions> {
        self.layout.categories()
    }

    /// Whether the menu opens on the default category of the categories:
    /// only if they are shown, so another one can be chosen, and without a
    /// start page.
    pub fn shows_default_category(&self) -> bool {
        self.categories().is_some() && !self.app_list().has_start_page()
    }

    /// Id of the category the menu opens on.
    pub fn default_category_id(&self) -> String {
        self.categories().map_or_else(
            || ApplicationCategory::ALL.id().to_owned(),
            |categories| categories.default_category.clone(),
        )
    }

    /// Whether favorites are shown anywhere, so apps can be added to them.
    pub fn shows_favorites(&self) -> bool {
        self.layout.shows_favorites()
    }
}

/// The layouts to pick from, see [`MenuLayout::layout`].
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum MenuLayout {
    /// User on top, app list and categories side by side.
    #[default]
    Classic,
    /// Colored header and footer bars around two panes.
    Retro,
    /// User and search share the header, categories beside the app list,
    /// power controls in the footer.
    Compact,
    /// Sidebar with pinned apps and power controls next to the categories
    /// and app list (Cinnamon like).
    Cinnamon,
    /// App grid with recently used apps below and user and power controls
    /// in the footer.
    Modern,
    /// Colored header and footer bars around a start page of favorite and
    /// recently used apps, which leads to all apps, next to places.
    Whistler,
}

impl MenuLayout {
    pub const ALL: [MenuLayout; 6] = [
        MenuLayout::Classic,
        MenuLayout::Retro,
        MenuLayout::Compact,
        MenuLayout::Cinnamon,
        MenuLayout::Modern,
        MenuLayout::Whistler,
    ];
}

/// How the apps are shown.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum AppView {
    /// One app per row, with its name next to its icon.
    #[default]
    Column,
    /// Several apps per row, with their names below their icons.
    Grid,
}

/// Size of the apps in the app view.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum ItemSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl ItemSize {
    pub const ALL: [ItemSize; 3] = [ItemSize::Small, ItemSize::Medium, ItemSize::Large];
}

/// Which apps a bar of apps shows.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum FeaturedApplications {
    /// Most launched apps.
    #[default]
    RecentlyUsed,
    /// Apps the user added to favorites.
    Favorites,
}

impl FeaturedApplications {
    /// The category listing these apps.
    pub fn category(self) -> ApplicationCategory {
        match self {
            FeaturedApplications::RecentlyUsed => ApplicationCategory::RECENTLY_USED,
            FeaturedApplications::Favorites => ApplicationCategory::FAVORITES,
        }
    }
}

/// Where on the screen the focused menu opens. A popup always opens at the
/// applet's button, as the compositor places it.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum MenuPosition {
    /// Against the panel, at the side of it the applet is on.
    #[default]
    Auto,
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl MenuPosition {
    /// Every position, in the order the settings list them.
    pub const ALL: [MenuPosition; 10] = [
        MenuPosition::Auto,
        MenuPosition::TopLeft,
        MenuPosition::Top,
        MenuPosition::TopRight,
        MenuPosition::Left,
        MenuPosition::Center,
        MenuPosition::Right,
        MenuPosition::BottomLeft,
        MenuPosition::Bottom,
        MenuPosition::BottomRight,
    ];
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum AppletButtonStyle {
    IconOnly,
    LabelOnly,
    IconAndLabel,
    #[default]
    Auto,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecentApplication {
    pub app_id: String,
    pub launch_count: u32,
}

/// The config of versions before the menu was made of blocks.
mod legacy {
    use serde::Deserialize;
    use serde::de::DeserializeOwned;

    use super::{AppView, FeaturedApplications, ItemSize, MenuLayout, Place};
    use crate::layout::{
        Block, BlockKind, BlockPath, Drop, Layout, SidePanelContent, Side, UserName,
    };
    use crate::model::application_category::ApplicationCategory;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
    enum HorizontalPosition {
        Left,
        Right,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
    enum VerticalPosition {
        Top,
        Bottom,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
    enum PowerMenuPosition {
        Header,
        AppList,
        Categories,
        Search,
        Footer,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
    enum UserWidgetStyle {
        UsernamePrefered,
        RealNamePrefered,
        None,
    }

    /// Where the old keys are read from.
    pub(super) trait Source {
        fn read<T: DeserializeOwned>(&self, key: &str) -> Option<T>;
    }

    impl Source for cosmic::cosmic_config::Config {
        fn read<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
            cosmic::cosmic_config::ConfigGet::get(self, key).ok()
        }
    }

    fn read<T: DeserializeOwned>(source: &impl Source, key: &str) -> Option<T> {
        source.read(key)
    }

    /// The layout the keys of an older version describe, if it has them.
    pub(super) fn layout(get: &impl Source) -> Option<Layout> {
        let preset: MenuLayout = read(get, "menu_layout")?;
        let mut layout = preset.layout();

        match read(get, "user_widget") {
            Some(UserWidgetStyle::None) => {
                if let Some(path) = find(&layout, BlockKind::User) {
                    layout.remove(path);
                }
            }
            Some(UserWidgetStyle::RealNamePrefered) => {
                for_each(&mut layout, |block| {
                    if let Block::User(options) = block {
                        options.name = UserName::FullName;
                    }
                });
            }
            _ => {}
        }

        let app_view: Option<AppView> = read::<Option<AppView>>(get, "app_view").flatten();
        let (column_size, column_description) = (
            read::<ItemSize>(get, "column_item_size"),
            read::<bool>(get, "column_show_description"),
        );
        let (grid_size, grid_description) = (
            read::<ItemSize>(get, "grid_item_size"),
            read::<bool>(get, "grid_show_description"),
        );
        let start_page_favorites: Option<u16> = read(get, "start_page_favorite_count");
        if let Some(options) = layout.app_list_mut() {
            options.view = app_view.unwrap_or(options.view);
            let (size, description) = match options.view {
                AppView::Column => (column_size, column_description),
                AppView::Grid => (grid_size, grid_description),
            };
            options.item_size = size.unwrap_or(options.item_size);
            options.show_description = description.unwrap_or(options.show_description);
            if options.start_page
                && let Some(count) = start_page_favorites
            {
                options.favorites_count = count;
            }
        }

        let side_panel: Option<SidePanelContent> = read(get, "side_panel");
        let places: Option<Vec<Place>> = read(get, "places");
        let hide: Option<bool> = read(get, "hide_side_panel_while_searching");
        let featured: Option<FeaturedApplications> = read(get, "featured_applications");
        let default_category: Option<String> = read(get, "default_category");
        // Layouts showing the favorites elsewhere kept the recently used apps.
        let favorites_category = featured == Some(FeaturedApplications::Favorites)
            && !matches!(preset, MenuLayout::Cinnamon | MenuLayout::Whistler);
        for_each(&mut layout, |block| match block {
            Block::SidePanel(options) => {
                options.content = side_panel.unwrap_or(options.content);
                options.places = places.clone().unwrap_or_else(|| options.places.clone());
                options.hide_while_searching = hide.unwrap_or(options.hide_while_searching);
                if favorites_category {
                    options.show_favorites = true;
                    options.show_recently_used = false;
                }
                if let Some(id) = &default_category {
                    // The favorites and recently used apps stood in for each
                    // other.
                    let featured = [ApplicationCategory::RECENTLY_USED, ApplicationCategory::FAVORITES];
                    options.default_category = if featured.iter().any(|c| c.id() == id) {
                        if favorites_category {
                            ApplicationCategory::FAVORITES.id().to_owned()
                        } else {
                            ApplicationCategory::RECENTLY_USED.id().to_owned()
                        }
                    } else {
                        id.clone()
                    };
                }
            }
            Block::Featured(options) if preset == MenuLayout::Modern => {
                options.source = featured.unwrap_or(options.source);
            }
            _ => {}
        });

        arrange_panes(&mut layout, preset, get);
        if let Some(position) = read::<VerticalPosition>(get, "search_field_position") {
            move_search(&mut layout, position == VerticalPosition::Top);
        }
        if preset == MenuLayout::Cinnamon
            && read::<HorizontalPosition>(get, "favorites_bar_position")
                == Some(HorizontalPosition::Right)
            && let Some(path) = find(&layout, BlockKind::Featured)
        {
            let row = &mut layout.rows[path.row].columns;
            let sidebar = row.remove(path.column);
            row.push(sidebar);
        }
        if let Some(position) = read::<PowerMenuPosition>(get, "power_menu_position") {
            move_power(&mut layout, preset, position);
        }

        Some(layout)
    }

    /// Where the first block of `kind` is.
    fn find(layout: &Layout, kind: BlockKind) -> Option<BlockPath> {
        layout
            .blocks()
            .find(|(_, block)| block.kind() == kind)
            .map(|(path, _)| path)
    }

    fn for_each(layout: &mut Layout, mut f: impl FnMut(&mut Block)) {
        for block in layout
            .rows
            .iter_mut()
            .flat_map(|row| &mut row.columns)
            .flat_map(|column| &mut column.blocks)
        {
            f(block);
        }
    }

    /// Sides and widths of the app list and the side panel next to it.
    fn arrange_panes(
        layout: &mut Layout,
        preset: MenuLayout,
        get: &impl Source,
    ) {
        let (Some(app_list), Some(side_panel)) = (
            find(layout, BlockKind::AppList),
            find(layout, BlockKind::SidePanel),
        ) else {
            return;
        };
        if app_list.row != side_panel.row {
            return;
        }
        let columns = &mut layout.rows[app_list.row].columns;
        if let Some(width) = read::<u16>(get, "side_panel_width") {
            let width = width.clamp(20, 60);
            columns[app_list.column].portion = 100 - width;
            columns[side_panel.column].portion = width;
        }
        let app_list_left = app_list.column < side_panel.column;
        let position = read::<HorizontalPosition>(get, "app_menu_position");
        let preset_left = !matches!(preset, MenuLayout::Compact | MenuLayout::Cinnamon);
        if let Some(position) = position
            && (position == HorizontalPosition::Left) != app_list_left
            && (position == HorizontalPosition::Left) != preset_left
        {
            columns.swap(app_list.column, side_panel.column);
        }
    }

    /// Puts the search field above the app list if `top`, otherwise below.
    fn move_search(layout: &mut Layout, top: bool) {
        let Some(search) = find(layout, BlockKind::Search) else {
            return;
        };
        let Some(panes) = layout.rows.iter().position(crate::layout::Row::fills_height) else {
            return;
        };
        if (search.row < panes) == top {
            return;
        }
        let alone = layout.rows[search.row].columns.len() == 1
            && layout.rows[search.row].columns[0].blocks.len() == 1;
        let index = if top { panes } else { panes + 1 };
        if alone {
            let row = layout.rows.remove(search.row);
            let index = if search.row < index { index - 1 } else { index };
            layout.rows.insert(index, row);
        } else if !top
            && let Some(power) = find(layout, BlockKind::Power)
            && power.row > panes
        {
            // Beside the power controls in the footer, as in the compact
            // layout.
            let Some(moved) = layout.move_block(
                search,
                Drop::Beside {
                    path: power,
                    side: Side::Before,
                },
            ) else {
                return;
            };
            // The spacer that pushed them to the right is not needed.
            let spacer = layout
                .blocks()
                .find(|(path, block)| path.row == moved.row && block.kind() == BlockKind::Spacer)
                .map(|(path, _)| path);
            if let Some(spacer) = spacer {
                layout.remove(spacer);
            }
        } else {
            layout.move_block(search, Drop::NewRow(index));
        }
    }

    /// Puts the power controls where `position` says, if the preset had
    /// them elsewhere.
    fn move_power(layout: &mut Layout, preset: MenuLayout, position: PowerMenuPosition) {
        use PowerMenuPosition::*;
        let supported: &[PowerMenuPosition] = match preset {
            MenuLayout::Classic | MenuLayout::Retro => &[Header, AppList, Categories, Footer],
            MenuLayout::Whistler => &[Header, AppList, Categories, Search, Footer],
            MenuLayout::Compact | MenuLayout::Modern => &[Header, Footer],
            MenuLayout::Cinnamon => &[],
        };
        // Unsupported positions fell back to the last one.
        let position = if supported.contains(&position) {
            position
        } else {
            match supported.last() {
                Some(position) => *position,
                None => return,
            }
        };
        let preset_position = if preset == MenuLayout::Classic {
            Categories
        } else {
            Footer
        };
        if position == preset_position {
            return;
        }
        let Some(power) = find(layout, BlockKind::Power) else {
            return;
        };
        let beside = |kind, side| {
            find(layout, kind).map(|path| Drop::Beside { path, side })
        };
        let drop = match position {
            Header => beside(BlockKind::User, Side::After).or(Some(Drop::NewRow(0))),
            AppList => beside(BlockKind::AppList, Side::Below),
            Categories => beside(BlockKind::SidePanel, Side::Below),
            Search => beside(BlockKind::Search, Side::After),
            Footer => Some(Drop::NewRow(layout.rows.len())),
        };
        let Some(drop) = drop else {
            return;
        };
        if let Some(moved) = layout.move_block(power, drop)
            && position == Header
            && moved.column > 0
        {
            // Pushed to the right, as in the header of the presets.
            layout.insert(
                Block::Spacer,
                Drop::Beside {
                    path: moved,
                    side: Side::Before,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Block, BlockKind, SidePanelContent, UserName};

    /// The layout migrated from a config with `keys`, given as RON.
    fn migrated(keys: &[(&str, &str)]) -> Option<Layout> {
        let keys: Vec<(String, String)> = keys
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        legacy::layout(&Keys(keys))
    }

    /// Old keys, as RON.
    struct Keys(Vec<(String, String)>);

    impl legacy::Source for Keys {
        fn read<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
            let (_, value) = self.0.iter().find(|(k, _)| k == key)?;
            ron::from_str(value).ok()
        }
    }

    fn kinds(layout: &Layout) -> Vec<Vec<Vec<BlockKind>>> {
        layout
            .rows
            .iter()
            .map(|row| {
                row.columns
                    .iter()
                    .map(|column| column.blocks.iter().map(Block::kind).collect())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn configs_without_the_old_keys_are_not_migrated() {
        assert!(migrated(&[]).is_none());
    }

    #[test]
    fn the_old_layout_becomes_its_preset() {
        assert_eq!(
            migrated(&[("menu_layout", "Retro")]),
            Some(MenuLayout::Retro.layout())
        );
    }

    #[test]
    fn the_old_options_go_to_their_blocks() {
        let layout = migrated(&[
            ("menu_layout", "Classic"),
            ("user_widget", "RealNamePrefered"),
            ("app_view", "Some(Grid)"),
            ("grid_item_size", "Large"),
            ("grid_show_description", "true"),
            ("side_panel", "Places"),
            ("places", "[Home, Separator, Terminal]"),
            ("side_panel_width", "30"),
        ])
        .unwrap();

        let app_list = layout.app_list().unwrap();
        assert_eq!(app_list.view, AppView::Grid);
        assert_eq!(app_list.item_size, ItemSize::Large);
        assert!(app_list.show_description);

        let side_panel = layout.side_panels().next().unwrap();
        assert_eq!(side_panel.content, SidePanelContent::Places);
        assert_eq!(side_panel.places, [Place::Home, Place::Separator, Place::Terminal]);
        let portions: Vec<u16> = layout.rows[2].columns.iter().map(|c| c.portion).collect();
        assert_eq!(portions, [70, 30]);

        assert!(layout.blocks().any(|(_, block)| matches!(
            block,
            Block::User(options) if options.name == UserName::FullName
        )));
    }

    #[test]
    fn the_old_positions_rearrange_the_blocks() {
        use BlockKind::*;
        let layout = migrated(&[
            ("menu_layout", "Classic"),
            ("user_widget", "None"),
            ("app_menu_position", "Right"),
            ("search_field_position", "Bottom"),
            ("power_menu_position", "Footer"),
        ])
        .unwrap();

        assert_eq!(
            kinds(&layout),
            [
                vec![vec![SidePanel], vec![AppList]],
                vec![vec![Search]],
                vec![vec![Power]],
            ]
        );
    }

    #[test]
    fn the_compact_search_moves_next_to_the_power_controls() {
        use BlockKind::*;
        let layout = migrated(&[
            ("menu_layout", "Compact"),
            ("search_field_position", "Bottom"),
        ])
        .unwrap();

        assert_eq!(
            kinds(&layout),
            [
                vec![vec![User]],
                vec![vec![SidePanel], vec![AppList]],
                vec![vec![Search], vec![Power]],
            ]
        );
    }

    #[test]
    fn power_controls_in_the_header_go_after_the_user() {
        use BlockKind::*;
        let layout = migrated(&[
            ("menu_layout", "Classic"),
            ("power_menu_position", "Header"),
        ])
        .unwrap();

        assert_eq!(kinds(&layout)[0], [vec![User], vec![Spacer], vec![Power]]);
    }

    #[test]
    fn favorites_take_the_place_of_the_recently_used_category() {
        let layout = migrated(&[
            ("menu_layout", "Classic"),
            ("featured_applications", "Favorites"),
            ("default_category", "\"recently-used\""),
        ])
        .unwrap();

        let categories = layout.categories().unwrap();
        assert!(categories.show_favorites);
        assert!(!categories.show_recently_used);
        assert_eq!(categories.default_category, "favorites");
    }

    #[test]
    fn layouts_of_the_user_are_kept_while_a_preset_is_picked() {
        let mut config = AppletConfig::default();
        let mut custom = MenuLayout::Classic.layout();
        custom.rows.reverse();

        config.set_menu_layout(custom.clone());
        config.set_menu_layout(MenuLayout::Modern.layout());

        assert_eq!(config.layout, MenuLayout::Modern.layout());
        assert_eq!(config.custom_layout, Some(custom));
    }

    #[test]
    fn default_category_only_next_to_the_categories() {
        let mut config = AppletConfig::default();
        assert!(config.shows_default_category());

        config.layout = MenuLayout::Modern.layout();
        assert!(!config.shows_default_category());

        config.layout = MenuLayout::Whistler.layout();
        assert!(!config.shows_default_category());
    }

    #[test]
    fn menu_size_stays_within_its_bounds() {
        let mut config = AppletConfig::default();
        assert_eq!(config.menu_size(), (600, 700));

        config.menu_width = 1;
        config.menu_height = u16::MAX;
        assert_eq!(config.menu_size(), (500, 1000));
    }
}
