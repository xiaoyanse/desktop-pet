use crate::domain::{config::Config, report::Report};
use std::sync::atomic::AtomicBool;

pub trait ReportCollector: Send + Sync {
    fn collect(&self, config: &Config, cancel: &AtomicBool) -> Result<Report, String>;
}
pub trait ReportStore: Send + Sync {
    fn save(&self, text: &str) -> Result<String, String>;
}
