pub mod cleanup;
pub mod monitoring;
pub mod office;
pub mod report;
pub mod screenshot;
pub mod work_timer;
use crate::interfaces::{DesktopSession, MemoryCleaner, SettingsStore};

pub struct Dependencies {
    pub sampler: crate::interfaces::SamplerFactory,
    pub settings: Box<dyn SettingsStore>,
    pub cleaner: Box<dyn MemoryCleaner>,
    pub cleanup_history: Box<dyn crate::interfaces::CleanupHistory>,
    pub desktop: crate::interfaces::DesktopFactory,
    pub font: Option<crate::interfaces::FontSource>,
    pub report_collector: std::sync::Arc<dyn crate::interfaces::ReportCollector>,
    pub report_store: std::sync::Arc<dyn crate::interfaces::ReportStore>,
    pub screen_capture: std::sync::Arc<dyn crate::interfaces::ScreenCapture>,
    pub screenshot_output: std::sync::Arc<dyn crate::interfaces::ScreenshotOutput>,
    pub office_store: std::sync::Arc<dyn crate::interfaces::OfficeStore>,
    pub local_time: std::sync::Arc<dyn crate::interfaces::LocalTime>,
}

pub struct Services {
    pub settings: Box<dyn SettingsStore>,
    pub desktop: Box<dyn DesktopSession>,
    pub cleaner: cleanup::CleanupService,
    pub timer: work_timer::WorkTimer,
    pub report: report::ReportService,
    pub screenshot: screenshot::ScreenshotService,
    pub office: office::OfficeService,
}
