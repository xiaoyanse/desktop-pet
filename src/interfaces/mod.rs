mod cleanup;
mod desktop;
mod monitoring;
mod report;
mod screenshot;
mod storage;

pub use cleanup::MemoryCleaner;
pub use desktop::{DesktopAction, DesktopFactory, DesktopSession, FontSource};
pub use monitoring::{ResourceSampler, SamplerFactory};
pub use report::{ReportCollector, ReportStore};
pub use screenshot::{ScreenCapture, ScreenshotOutput};
use std::sync::Arc;
pub use storage::{CleanupHistory, SettingsStore};

pub type Wake = Arc<dyn Fn() + Send + Sync>;
