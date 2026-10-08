// SPDX-License-Identifier: GPL-3.0-only

use crate::fl;
use cosmic::{
    cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, Config, CosmicConfigEntry},
    Application,
};
use serde::{Deserialize, Serialize};

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
    /// Ids of apps added to favorites, shown in layouts with a favorites bar.
    pub favorite_applications: Vec<String>,
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
        }
    }
}

impl AppletConfig {
    /// Side panel width in percent, matching the original 5:3 split.
    pub const DEFAULT_SIDE_PANEL_WIDTH: u16 = 38;
    /// Bounds of `side_panel_width`, keeping both panes usable.
    pub const SIDE_PANEL_WIDTH_RANGE: std::ops::RangeInclusive<u16> = 20..=60;

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

    /// Switches to `layout` and resets the element positions to the ones
    /// that best resemble it.
    pub fn apply_layout(&mut self, layout: MenuLayout) {
        self.menu_layout = layout;
        match layout {
            MenuLayout::Classic => {
                self.app_menu_position = HorizontalPosition::Left;
                self.search_field_position = VerticalPosition::Top;
                self.power_menu_position = PowerMenuPosition::Categories;
                self.side_panel = SidePanel::Categories;
            }
            MenuLayout::Retro => {
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
}

impl Default for MenuLayout {
    fn default() -> Self {
        MenuLayout::Classic
    }
}

impl MenuLayout {
    pub const ALL: [MenuLayout; 5] = [
        MenuLayout::Classic,
        MenuLayout::Retro,
        MenuLayout::Compact,
        MenuLayout::Cinnamon,
        MenuLayout::Modern,
    ];

    /// Power controls positions this layout supports. The last one is the
    /// fallback when the configured position is not supported.
    pub fn power_menu_positions(self) -> &'static [PowerMenuPosition] {
        match self {
            MenuLayout::Classic | MenuLayout::Retro => &[
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

    /// Whether the layout shows the favorite apps.
    pub fn has_favorites(self) -> bool {
        self == MenuLayout::Cinnamon
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
