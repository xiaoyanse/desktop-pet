pub mod cleanup;
pub mod monitoring;
pub mod report;
pub mod work_timer;
use crate::interfaces::{DesktopSession, MemoryCleaner, SettingsStore};

pub struct Dependencies {
    pub sampler: crate::interfaces::SamplerFactory,
    pub settings: Box<dyn SettingsStore>,
    pub cleaner: Box<dyn MemoryCleaner>,
    pub cleanup_history: Box<dyn crate::interfaces::CleanupHistory>,
    pub desktop: crate::interfaces::DesktopFactory,
    pub font: Option<(std::borrow::Cow<'static, str>, Vec<u8>)>,
    pub report_collector: std::sync::Arc<dyn crate::interfaces::ReportCollector>,
    pub report_store: std::sync::Arc<dyn crate::interfaces::ReportStore>,
}

pub struct Services {
    pub settings: Box<dyn SettingsStore>,
    pub desktop: Box<dyn DesktopSession>,
    pub cleaner: cleanup::CleanupService,
    pub timer: work_timer::WorkTimer,
    pub report: report::ReportService,
}
