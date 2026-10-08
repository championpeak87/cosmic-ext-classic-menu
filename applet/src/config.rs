// SPDX-License-Identifier: GPL-3.0-only

use crate::fl;
use cosmic::{
    cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, Config, CosmicConfigEntry},
    Application,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
#[id = "cosmic-ext-classic-menu"]
pub struct AppletConfig {
    pub app_menu_position: HorizontalPosition,
    pub search_field_position: VerticalPosition,
    pub power_menu_position: PowerMenuPosition,
    pub applet_button_style: AppletButtonStyle,
    pub user_widget: UserWidgetStyle,
    pub button_label: String,
    pub button_icon: String,
    pub recent_applications: Vec<RecentApplication>,
}

impl Default for AppletConfig {
    fn default() -> Self {
        AppletConfig {
            app_menu_position: HorizontalPosition::default(),
            search_field_position: VerticalPosition::default(),
            power_menu_position: PowerMenuPosition::default(),
            applet_button_style: AppletButtonStyle::default(),
            user_widget: UserWidgetStyle::default(),
            button_label: fl!("menu-label").to_owned(),
            button_icon: format!("/usr/share/cosmic/{}/applet-buttons/default.svg", crate::applet::Applet::APP_ID).to_owned(),
            recent_applications: vec![],
        }
    }
}

impl AppletConfig {
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

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecentApplication {
    pub app_id: String,
    pub launch_count: u32,
}
