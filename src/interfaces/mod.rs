use crate::domain::{config::Config, resources::Snapshot};
use std::sync::Arc;

pub type Wake = Arc<dyn Fn() + Send + Sync>;

pub type SamplerFactory = Box<dyn FnOnce() -> Box<dyn ResourceSampler> + Send>;
pub type DesktopFactory = Box<dyn FnOnce(isize, Wake) -> Box<dyn DesktopSession>>;
pub trait ResourceSampler {
    fn sample(&mut self, config: &Config, details: bool) -> Snapshot;
}

pub trait SettingsStore {
    fn load(&self) -> (Config, Option<String>);
    fn save(&self, config: &Config) -> Result<(), String>;
}

#[derive(Clone, Copy)]
pub enum DesktopAction {
    Unlock,
    Passthrough,
    Settings,
    ToggleMode,
    Pet,
    PetReact,
    PetDrop,
    Cleanup,
    Exit,
}

pub trait DesktopSession {
    fn configure_card(
        &self,
        collapsed: bool,
        draggable: bool,
        drag_height: f32,
        toolbar_width: f32,
    );
    fn geometry(&self) -> Option<crate::domain::desktop::DesktopGeometry>;
    fn cursor(&self) -> Option<[f32; 2]>;
    fn configure_auxiliary(&self);
    fn warning(&self) -> Option<String> {
        None
    }
    fn actions(&self) -> Vec<DesktopAction>;
    fn set_snapping(&self, enabled: bool);
    fn tooltip(&mut self, text: &str, active: bool);
    fn menu(&self, compact: bool) -> i32;
    fn autostart(&self) -> bool;
    fn set_autostart(&self, enable: bool) -> Result<(), String>;
    fn set_clean_hotkey(&self, enabled: bool) -> Result<(), String>;
}

pub trait MemoryCleaner {
    fn request(
        &self,
        operations: Vec<crate::domain::cleanup::Operation>,
        automatic: bool,
        wake: Wake,
    ) -> std::sync::mpsc::Receiver<Result<crate::domain::cleanup::CleanupReport, String>>;
    fn elevated(&self) -> bool;
    fn inspect(&self) -> Result<crate::domain::cleanup::MemoryStatus, String>;
}
pub trait CleanupHistory {
    fn load(&self) -> Result<Vec<crate::domain::cleanup::CleanupReport>, String>;
    fn save(&self, reports: &[crate::domain::cleanup::CleanupReport]) -> Result<(), String>;
}
