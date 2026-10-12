mod cleanup;
mod clipboard;
mod desktop;
mod meeting;
mod monitoring;
mod office;
mod report;
mod screenshot;
mod storage;

pub use cleanup::MemoryCleaner;
pub use clipboard::{ClipboardBackend, ClipboardEvent, ClipboardWatch};
pub use desktop::{DesktopAction, DesktopFactory, DesktopSession, FontSource};
pub use meeting::MicrophoneSource;
pub use monitoring::{ResourceSampler, SamplerFactory};
pub use office::{LocalTime, OfficeStore};
pub use report::{ReportCollector, ReportStore};
pub use screenshot::{ScreenCapture, ScreenshotOutput};
use std::sync::Arc;
pub use storage::{CleanupHistory, SettingsStore};

pub type Wake = Arc<dyn Fn() + Send + Sync>;
