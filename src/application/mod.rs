pub mod cleanup;
pub mod monitoring;
use crate::interfaces::{DesktopSession, MemoryCleaner, SettingsStore};

pub struct Dependencies {
    pub sampler: crate::interfaces::SamplerFactory,
    pub settings: Box<dyn SettingsStore>,
    pub cleaner: Box<dyn MemoryCleaner>,
    pub cleanup_history: Box<dyn crate::interfaces::CleanupHistory>,
    pub desktop: crate::interfaces::DesktopFactory,
    pub font: Option<(std::borrow::Cow<'static, str>, Vec<u8>)>,
}

pub struct Services {
    pub settings: Box<dyn SettingsStore>,
    pub desktop: Box<dyn DesktopSession>,
    pub cleaner: cleanup::CleanupService,
}
