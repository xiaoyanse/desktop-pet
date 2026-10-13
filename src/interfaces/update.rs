use crate::domain::update::{Preferences, Release};
use std::sync::atomic::{AtomicBool, AtomicU64};
pub trait UpdateBackend: Send + Sync {
    fn check(&self, preferences: &Preferences) -> Result<Option<Release>, String>;
    fn download(
        &self,
        release: &Release,
        cancel: &AtomicBool,
        progress: &AtomicU64,
    ) -> Result<(), String>;
    fn launch_install(&self) -> Result<(), String>;
    fn schedule_install(&self) -> Result<(), String>;
    fn confirm_running(&self) -> Result<(), String> {
        Ok(())
    }
}
