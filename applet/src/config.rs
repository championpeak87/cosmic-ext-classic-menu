// SPDX-License-Identifier: GPL-3.0-only

use crate::fl;
use cosmic::{
    cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, Config, CosmicConfigEntry},
    Application,
};
use serde::{Deserialize, Serialize};

use crate::model::application_category::ApplicationCategory;
pub use crate::model::place::Place;

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
#[id = "cosmic-ext-classic-menu"]
pub struct AppletConfig {
    pub menu_layout: MenuLayout,
    pub app_menu_position: HorizontalPosition,
    pub search_field_position: VerticalPosition,
    pub power_menu_position: PowerMenuPosition,
    pub applet_button_style: AppletButtonStyle,
    pub user_widget: UserWidgetStyle,
    pub button_label: String,
    pub button_icon: String,
    pub recent_applications: Vec<RecentApplication>,
    /// What the pane next to the app list shows.
    pub side_panel: SidePanel,
    /// Share of the panes' width taken by the side panel, in percent.
    pub side_panel_width: u16,
    /// Places shown when `side_panel` is [`SidePanel::Places`], in order.
    pub places: Vec<Place>,
    /// Ids of apps added to favorites, shown in layouts with a favorites bar
    /// and when `featured_applications` is [`FeaturedApplications::Favorites`].
    pub favorite_applications: Vec<String>,
    /// Which apps the "Recently used" category and recommendations show.
    pub featured_applications: FeaturedApplications,
    /// How apps are shown; `None` uses the layout's own view, see
    /// [`AppletConfig::effective_app_view`].
    pub app_view: Option<AppView>,
    /// Size of the apps in the column view.
    pub column_item_size: ItemSize,
    /// Whether the column view shows app descriptions.
    pub column_show_description: bool,
    /// Size of the apps in the grid view.
    pub grid_item_size: ItemSize,
    /// Whether the grid view shows app descriptions.
    pub grid_show_description: bool,
    /// How many of the first favorites the start page shows, see
    /// [`MenuLayout::has_start_page`].
    pub start_page_favorite_count: u16,
    /// Id of the category the menu opens on, see
    /// [`AppletConfig::shows_default_category`].
    pub default_category: String,
}

impl Default for AppletConfig {
    fn default() -> Self {
        AppletConfig {
            menu_layout: MenuLayout::default(),
            app_menu_position: HorizontalPosition::default(),
            search_field_position: VerticalPosition::default(),
            power_menu_position: PowerMenuPosition::default(),
            applet_button_style: AppletButtonStyle::default(),
            user_widget: UserWidgetStyle::default(),
            button_label: fl!("menu-label").to_owned(),
            button_icon: format!("/usr/share/cosmic/{}/applet-buttons/default.svg", crate::applet::Applet::APP_ID).to_owned(),
            recent_applications: vec![],
            side_panel: SidePanel::default(),
            side_panel_width: Self::DEFAULT_SIDE_PANEL_WIDTH,
            places: Place::DEFAULT.to_vec(),
            favorite_applications: vec![],
            featured_applications: FeaturedApplications::default(),
            app_view: None,
            column_item_size: ItemSize::default(),
            column_show_description: true,
            grid_item_size: ItemSize::default(),
            grid_show_description: false,
            start_page_favorite_count: Self::DEFAULT_START_PAGE_FAVORITE_COUNT,
            default_category: ApplicationCategory::ALL.id().to_owned(),
        }
    }
}

impl AppletConfig {
    /// Side panel width in percent, matching the original 5:3 split.
    pub const DEFAULT_SIDE_PANEL_WIDTH: u16 = 38;
    /// Bounds of `side_panel_width`, keeping both panes usable.
    pub const SIDE_PANEL_WIDTH_RANGE: std::ops::RangeInclusive<u16> = 20..=60;

    /// Favorites on the start page, leaving room for recently used apps.
    pub const DEFAULT_START_PAGE_FAVORITE_COUNT: u16 = 2;
    /// Bounds of `start_page_favorite_count`.
    pub const START_PAGE_FAVORITE_COUNT_RANGE: std::ops::RangeInclusive<u16> = 1..=10;

    /// `FillPortion`s of the app list and the side panel.
    pub fn pane_portions(&self) -> (u16, u16) {
        let side_panel = self.side_panel_width.clamp(
            *Self::SIDE_PANEL_WIDTH_RANGE.start(),
            *Self::SIDE_PANEL_WIDTH_RANGE.end(),
        );
        (100 - side_panel, side_panel)
    }

    pub fn config_handler() -> Option<Config> {
        Config::new(crate::applet::Applet::APP_ID, 1).ok()
    }

    pub fn config() -> AppletConfig {
        match Self::config_handler() {
            Some(config_handler) => AppletConfig::get_entry(&config_handler)
                .unwrap_or_else(|(_errs, config)| config),
            None => AppletConfig::default(),
        }
    }

    /// Where the power controls are shown, given what the current layout
    /// supports. `None` means the layout places them itself.
    pub fn effective_power_menu_position(&self) -> Option<PowerMenuPosition> {
        let supported = self.menu_layout.power_menu_positions();
        if supported.contains(&self.power_menu_position) {
            Some(self.power_menu_position)
        } else {
            supported.last().copied()
        }
    }

    /// Which apps the "Recently used" category and recommendations show.
    /// Layouts showing the favorites elsewhere keep the recently used apps.
    pub fn effective_featured_applications(&self) -> FeaturedApplications {
        if self.menu_layout.has_favorites() {
            FeaturedApplications::RecentlyUsed
        } else {
            self.featured_applications
        }
    }

    /// Whether the menu opens on `default_category`: only next to the
    /// categories, so another one can be chosen, and without a start page.
    pub fn shows_default_category(&self) -> bool {
        self.menu_layout.has_side_panel()
            && self.side_panel == SidePanel::Categories
            && !self.menu_layout.has_start_page()
    }

    /// Id of the category the menu opens on. Favorites and recently used
    /// apps take each other's place, see `featured_applications`, so either
    /// stands for the one shown.
    pub fn default_category_id(&self) -> String {
        let featured = [ApplicationCategory::RECENTLY_USED, ApplicationCategory::FAVORITES];
        if featured.iter().any(|category| category.id() == self.default_category) {
            self.effective_featured_applications().category().id().to_owned()
        } else {
            self.default_category.clone()
        }
    }

    /// Whether favorites are shown anywhere, so apps can be added to them.
    pub fn shows_favorites(&self) -> bool {
        self.menu_layout.has_favorites()
            || self.featured_applications == FeaturedApplications::Favorites
    }

    /// How apps are shown, given the layout when none was chosen.
    pub fn effective_app_view(&self) -> AppView {
        self.app_view.unwrap_or(self.menu_layout.default_app_view())
    }

    /// Switches to `layout` and resets the element positions to the ones
    /// that best resemble it.
    pub fn apply_layout(&mut self, layout: MenuLayout) {
        self.menu_layout = layout;
        self.app_view = None;
        match layout {
            MenuLayout::Classic => {
                self.app_menu_position = HorizontalPosition::Left;
                self.search_field_position = VerticalPosition::Top;
                self.power_menu_position = PowerMenuPosition::Categories;
                self.side_panel = SidePanel::Categories;
            }
            MenuLayout::Retro | MenuLayout::Whistler => {
                self.app_menu_position = HorizontalPosition::Left;
                self.search_field_position = VerticalPosition::Bottom;
                self.power_menu_position = PowerMenuPosition::Footer;
                self.side_panel = SidePanel::Places;
            }
            MenuLayout::Compact | MenuLayout::Cinnamon => {
                self.app_menu_position = HorizontalPosition::Right;
                self.search_field_position = VerticalPosition::Top;
                self.power_menu_position = PowerMenuPosition::Footer;
                self.side_panel = SidePanel::Categories;
            }
            MenuLayout::Modern => {
                self.search_field_position = VerticalPosition::Top;
                self.power_menu_position = PowerMenuPosition::Footer;
            }
        }
    }
}

/// Overall arrangement of the menu popup.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum MenuLayout {
    /// User on top, app list and categories side by side.
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

impl Default for MenuLayout {
    fn default() -> Self {
        MenuLayout::Classic
    }
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

    /// Power controls positions this layout supports. The last one is the
    /// fallback when the configured position is not supported.
    pub fn power_menu_positions(self) -> &'static [PowerMenuPosition] {
        match self {
            MenuLayout::Classic | MenuLayout::Retro | MenuLayout::Whistler => &[
                PowerMenuPosition::Header,
                PowerMenuPosition::AppList,
                PowerMenuPosition::Categories,
                PowerMenuPosition::Footer,
            ],
            MenuLayout::Compact | MenuLayout::Modern => {
                &[PowerMenuPosition::Header, PowerMenuPosition::Footer]
            }
            // Always in the sidebar.
            MenuLayout::Cinnamon => &[],
        }
    }

    /// Whether the layout has a pane next to the app list, which can show
    /// categories or places.
    pub fn has_side_panel(self) -> bool {
        self != MenuLayout::Modern
    }

    /// Whether the layout shows the favorite apps itself, rather than in
    /// a category.
    pub fn has_favorites(self) -> bool {
        matches!(self, MenuLayout::Cinnamon | MenuLayout::Whistler)
    }

    /// Whether the app list starts with the favorite and recently used
    /// apps, leading to all apps.
    pub fn has_start_page(self) -> bool {
        self == MenuLayout::Whistler
    }

    /// How the layout shows apps unless another view is chosen.
    pub fn default_app_view(self) -> AppView {
        if self == MenuLayout::Modern {
            AppView::Grid
        } else {
            AppView::Column
        }
    }
}

/// How the apps are shown.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum AppView {
    /// One app per row, with its name next to its icon.
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

/// Apps shown in the category next to "All applications" and below the app
/// grid of the modern layout.
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
    pub fn category(self) -> crate::model::application_category::ApplicationCategory {
        use crate::model::application_category::ApplicationCategory;
        match self {
            FeaturedApplications::RecentlyUsed => ApplicationCategory::RECENTLY_USED,
            FeaturedApplications::Favorites => ApplicationCategory::FAVORITES,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]

pub enum AppletButtonStyle {
    IconOnly,
    LabelOnly,
    IconAndLabel,
    Auto,
}

impl Default for AppletButtonStyle {
    fn default() -> Self {
        AppletButtonStyle::Auto
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]

pub enum UserWidgetStyle {
    UsernamePrefered,
    RealNamePrefered,
    None,
}

impl Default for UserWidgetStyle {
    fn default() -> Self {
        UserWidgetStyle::UsernamePrefered
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]

pub enum HorizontalPosition {
    Left,
    Right,
}

impl Default for HorizontalPosition {
    fn default() -> Self {
        HorizontalPosition::Left
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum VerticalPosition {
    Top,
    Bottom,
}

impl Default for VerticalPosition {
    fn default() -> Self {
        VerticalPosition::Top
    }
}

/// Where the power controls are placed in the menu.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum PowerMenuPosition {
    /// Next to the user widget, above the panes.
    Header,
    /// Below the app list pane.
    AppList,
    /// Below the categories pane.
    Categories,
    /// Full-width row at the bottom of the menu.
    Footer,
}

impl Default for PowerMenuPosition {
    fn default() -> Self {
        PowerMenuPosition::Categories
    }
}

/// What the pane next to the app list shows.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum SidePanel {
    /// App categories filtering the app list.
    #[default]
    Categories,
    /// Shortcuts to folders and system tools.
    Places,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecentApplication {
    pub app_id: String,
    pub launch_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(menu_layout: MenuLayout, app_view: Option<AppView>) -> AppletConfig {
        AppletConfig {
            menu_layout,
            app_view,
            ..Default::default()
        }
    }

    #[test]
    fn app_view_defaults_to_the_layouts_own() {
        assert_eq!(config(MenuLayout::Classic, None).effective_app_view(), AppView::Column);
        assert_eq!(config(MenuLayout::Modern, None).effective_app_view(), AppView::Grid);
        assert_eq!(
            config(MenuLayout::Modern, Some(AppView::Column)).effective_app_view(),
            AppView::Column
        );
    }

    #[test]
    fn default_category_only_next_to_the_categories() {
        let mut config = config(MenuLayout::Classic, None);
        config.side_panel = SidePanel::Categories;
        assert!(config.shows_default_category());

        config.side_panel = SidePanel::Places;
        assert!(!config.shows_default_category());

        config.side_panel = SidePanel::Categories;
        config.menu_layout = MenuLayout::Modern;
        assert!(!config.shows_default_category());

        config.menu_layout = MenuLayout::Whistler;
        assert!(!config.shows_default_category());
    }

    #[test]
    fn default_category_follows_the_featured_applications() {
        let mut config = config(MenuLayout::Classic, None);
        config.default_category = "recently-used".to_owned();
        config.featured_applications = FeaturedApplications::Favorites;
        assert_eq!(config.default_category_id(), "favorites");

        config.default_category = "favorites".to_owned();
        config.featured_applications = FeaturedApplications::RecentlyUsed;
        assert_eq!(config.default_category_id(), "recently-used");

        config.default_category = "office".to_owned();
        assert_eq!(config.default_category_id(), "office");
    }

    #[test]
    fn applying_a_layout_resets_the_app_view() {
        let mut config = config(MenuLayout::Classic, Some(AppView::Grid));
        config.apply_layout(MenuLayout::Retro);

        assert_eq!(config.effective_app_view(), AppView::Column);
    }
}
