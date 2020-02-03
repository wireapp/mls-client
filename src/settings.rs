extern crate lazy_static;

use config::{ConfigError, Config, File};
use self::lazy_static::lazy_static;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub server: String
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();
        s.merge(File::with_name("Settings.toml"))?;
        s.try_into()
    }
}

lazy_static! {
    pub static ref SETTINGS: Settings = Settings::new().unwrap();
}