extern crate lazy_static;

use self::lazy_static::lazy_static;
use config::{Config, ConfigError, File};

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub server: String,
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
