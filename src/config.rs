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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum PowerOptionsPosition {
    Top,
    Bottom,
}

impl Display for PowerOptionsPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowerOptionsPosition::Top => write!(f, "Top"),
            PowerOptionsPosition::Bottom => write!(f, "Bottom"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum AppListPosition {
    Left,
    Right,
}

impl Display for AppListPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppListPosition::Left => write!(f, "Left"),
            AppListPosition::Right => write!(f, "Right"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum SearchFieldPosition {
    Top,
    Bottom,
}

impl Display for SearchFieldPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchFieldPosition::Top => write!(f, "Top"),
            SearchFieldPosition::Bottom => write!(f, "Bottom"),
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