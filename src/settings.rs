use config::{ConfigError, Config, File};

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