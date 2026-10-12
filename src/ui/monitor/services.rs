use crate::{
    application::{Dependencies, Services},
    domain::config::Config,
    interfaces::{SamplerFactory, Wake},
};
pub fn build(
    dependencies: Dependencies,
    hwnd: isize,
    wake: Wake,
    config: &Config,
) -> (Services, SamplerFactory) {
    let mut services = Services {
        clipboard: crate::application::clipboard::ClipboardService::new(
            dependencies.clipboard,
            wake.clone(),
            config.clipboard.clone(),
        ),
        meeting: crate::application::meeting::MeetingService::new(
            dependencies.microphone,
            wake.clone(),
        ),
        office: crate::application::office::OfficeService::new(
            dependencies.office_store,
            dependencies.local_time,
            wake.clone(),
        ),
        screenshot: crate::application::screenshot::ScreenshotService::new(
            dependencies.screen_capture,
            dependencies.screenshot_output,
            wake.clone(),
        ),
        report: crate::application::report::ReportService::new(
            dependencies.report_collector,
            dependencies.report_store,
            wake.clone(),
        ),
        timer: Default::default(),
        settings: dependencies.settings,
        cleaner: crate::application::cleanup::CleanupService::new(
            dependencies.cleaner,
            dependencies.cleanup_history,
            wake.clone(),
        ),
        desktop: (dependencies.desktop)(hwnd, wake),
    };
    if !config.screenshot_folder.is_empty() {
        services.screenshot.folder = config.screenshot_folder.clone();
    }
    (services, dependencies.sampler)
}
