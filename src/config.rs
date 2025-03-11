use std::fmt::{Display, self};

use cosmic::cosmic_config::{Config, ConfigGet, ConfigSet};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub fn update_config<T>(config: Config, key: &str, value: T)
where
    T: Serialize + Display + Clone,
{
    let config_set = config.set(key, value.clone());

    match config_set {
        Ok(_) => println!("Config variable for {key} was set to {value}"),
        Err(e) => eprintln!("Something went wrong setting {key} to {value}: {e}"),
    }

    let config_tx = config.transaction();
    let tx_result = config_tx.commit();

    match tx_result {
        Ok(_) => println!("Config transaction has been completed!"),
        Err(e) => eprintln!("Something with the config transaction when wrong: {e}"),
    }
}

pub fn load_config<T>(key: &str, config_vers: u64) -> (Option<T>, String)
where
    T: DeserializeOwned,
{
    let config = match Config::new("com.championpeak87.cosmic-classic-menu", config_vers) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Loading config file had an error: {e}");
            Config::system("com.championpeak87.cosmic-classic-menu", 1).unwrap()
        }
    };

    match config.get(key) {
        Ok(value) => (Some(value), "".to_owned()),
        Err(_e) => {
            update_config(config, key, "");
            (None, "Created config for key".to_owned())
        }
    }
}

pub fn load_or_default_config<T>(config: Config, key: &str, config_vers: u64, default: T) -> T
where
    T: DeserializeOwned + Clone + Serialize + Display,
{
    let config_get = load_config::<T>(key, config_vers).0;

    match config_get {
        Some(value) => value,
        None => {
            update_config(config, key, default.clone());
            default
        }
    }
}

pub const POWER_OPTIONS_POSITION: &str = "power_options_position";
pub const APP_LIST_POSITION: &str = "app_list_position";
pub const SEARCH_FIELD_POSITION: &str = "search_field_position";
pub const RECENT_APPLICATIONS: &str = "recent_applications";
pub const SEARCH_POWER_ONELINE: &str = "search_power_oneline";
pub const POWER_OPTIONS_ALIGNMENT: &str = "power_options_alignment";
pub const APP_LIST_VIEW: &str = "app_list_view";
pub const DEFAULT_VIEW: &str = "default_view";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum HorizontalPosition {
    Left,
    Right,
}

impl Display for HorizontalPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HorizontalPosition::Left => write!(f, "Left"),
            HorizontalPosition::Right => write!(f, "Right"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum VerticalPosition {
    Top,
    Bottom,
}

impl Display for VerticalPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerticalPosition::Top => write!(f, "Top"),
            VerticalPosition::Bottom => write!(f, "Bottom"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum AppListView {
    List,
    // TODO: Grid to be implemented
    // Grid
}

impl Display for AppListView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppListView::List => write!(f, "List"),
        }
    }
}

#[derive(Debug, Serialize, Default, Clone, Deserialize)]
pub struct RecentApplicationConfig {
    pub recent_applications: Vec<RecentApplication>
}

impl Display for RecentApplicationConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", json)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RecentApplication {
    pub app_id: String,
    pub launch_count: u32
}

impl Display for RecentApplication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", json)
    }
}