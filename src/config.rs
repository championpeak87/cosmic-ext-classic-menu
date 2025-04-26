// SPDX-License-Identifier: GPL-3.0-only

use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
#[id = "cosmic-classic-menu"]
pub struct Config {
    pub power_menu_position: VerticalPosition,
    pub app_menu_position: HorizontalPosition,
    pub search_field_position: VerticalPosition,
    pub recent_applications: Vec<RecentApplication>
}

impl Default for Config {
    fn default() -> Self {
        Config {
            power_menu_position: VerticalPosition::default(),
            app_menu_position: HorizontalPosition::default(),
            search_field_position: VerticalPosition::default(),
            recent_applications: vec![]
        }
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
