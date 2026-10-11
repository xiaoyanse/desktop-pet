use crate::domain::{cleanup::CleanupReport, config::Config};

pub trait SettingsStore {
    fn load(&self) -> (Config, Option<String>);
    fn save(&self, config: &Config) -> Result<(), String>;
}

pub trait CleanupHistory {
    fn load(&self) -> Result<Vec<CleanupReport>, String>;
    fn save(&self, reports: &[CleanupReport]) -> Result<(), String>;
}
