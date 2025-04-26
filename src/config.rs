// SPDX-License-Identifier: GPL-3.0-only

use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
pub struct Config {
    pub power_menu_position: PowerOptionsPosition,
    pub app_menu_position: AppListPosition,
    pub search_field_position: SearchFieldPosition,
    pub recent_applications: Vec<RecentApplication>
}

impl Default for Config {
    fn default() -> Self {
        Config {
            power_menu_position: PowerOptionsPosition::default(),
            app_menu_position: AppListPosition::default(),
            search_field_position: SearchFieldPosition::default(),
            recent_applications: vec![]
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]

pub enum PowerOptionsPosition {
    Top,
    Bottom,
}

impl Default for PowerOptionsPosition {
    fn default() -> Self {
        PowerOptionsPosition::Top
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum AppListPosition {
    Left,
    Right,
}

impl Default for AppListPosition {
    fn default() -> Self {
        AppListPosition::Left
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum SearchFieldPosition {
    Top,
    Bottom,
}

impl Default for SearchFieldPosition {
    fn default() -> Self {
        SearchFieldPosition::Top
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecentApplication {
    pub app_id: String,
    pub launch_count: u32
}
