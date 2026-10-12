mod processes;
use crate::{
    domain::{
        config::{Config, Metric, Row},
        report::Report,
    },
    infrastructure::windows::monitoring::sampler::Sampler,
    interfaces::{ReportCollector, ResourceSampler},
};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub struct WindowsReportCollector;
impl ReportCollector for WindowsReportCollector {
    fn collect(&self, config: &Config, cancel: &AtomicBool) -> Result<Report, String> {
        let config = Config {
            rows: Metric::ALL
                .into_iter()
                .map(|metric| Row {
                    metric,
                    enabled: true,
                    ..Default::default()
                })
                .collect(),
            ..config.clone()
        };
        let mut sampler = Sampler::new();
        sampler.sample(&config, false);
        let first = processes::capture(cancel)?;
        let start = Instant::now();
        for _ in 0..20 {
            if cancel.load(Ordering::Relaxed) {
                return Err("已取消采集".into());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let snapshot = sampler.sample(&config, false);
        let second = processes::capture(cancel)?;
        let mut report = processes::assemble(first, second, start.elapsed(), cancel)?;
        report.set_resources(&snapshot);
        unsafe {
            use windows_sys::Win32::{
                Foundation::SYSTEMTIME, System::SystemInformation::GetLocalTime,
            };
            let mut time = SYSTEMTIME::default();
            GetLocalTime(&mut time);
            report.captured_at = format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                time.wYear, time.wMonth, time.wDay, time.wHour, time.wMinute, time.wSecond
            );
        }
        Ok(report)
    }
}
