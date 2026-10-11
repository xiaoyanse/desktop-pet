mod cleanup;
mod desktop;
mod monitoring;
mod storage;

pub use cleanup::MemoryCleaner;
pub use desktop::{DesktopAction, DesktopFactory, DesktopSession};
pub use monitoring::{ResourceSampler, SamplerFactory};
use std::sync::Arc;
pub use storage::{CleanupHistory, SettingsStore};

pub type Wake = Arc<dyn Fn() + Send + Sync>;
