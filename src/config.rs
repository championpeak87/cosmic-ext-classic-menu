// SPDX-License-Identifier: GPL-3.0-only

use cosmic::{cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, Config, CosmicConfigEntry}, Application};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
#[id = "cosmic-classic-menu"]
pub struct CosmicClassicMenuConfig {
    pub app_menu_position: HorizontalPosition,
    pub search_field_position: VerticalPosition,
    pub applet_button_style: AppletButtonStyle,
    pub user_widget: UserWidgetStyle,
    pub recent_applications: Vec<RecentApplication>
}

impl Default for CosmicClassicMenuConfig {
    fn default() -> Self {
        CosmicClassicMenuConfig {
            app_menu_position: HorizontalPosition::default(),
            search_field_position: VerticalPosition::default(),
            applet_button_style: AppletButtonStyle::default(),
            user_widget: UserWidgetStyle::default(),
            recent_applications: vec![],
        }
    }
}

impl CosmicClassicMenuConfig {
    pub fn config_handler() -> Option<Config> {
        Config::new(crate::applet::CosmicClassicMenu::APP_ID, 1).ok()
    }

    pub fn config() -> CosmicClassicMenuConfig {
        match Self::config_handler() {
            Some(config_handler) => {
                CosmicClassicMenuConfig::get_entry(&config_handler).unwrap_or_else(|(_errs, config)| {
                    config
                })
            }
            None => CosmicClassicMenuConfig::default(),
        }
    }
}


#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]

pub enum AppletButtonStyle {
    IconOnly,
    LabelOnly,
    IconAndLabel,
    Auto
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
    None
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


#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecentApplication {
    pub app_id: String,
    pub launch_count: u32
}
