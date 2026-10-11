use crate::config::Metric;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Default, Serialize)]
pub struct Snapshot {
    pub values: HashMap<Metric, Reading>,
    pub gpus: Vec<Device>,
    pub gpu_values: Vec<GpuReading>,
    pub disks: Vec<Device>,
    pub networks: Vec<Device>,
    pub cpu_top: Vec<Process>,
    pub memory_top: Vec<Process>,
    pub process_status: String,
    pub sampled_at: u64,
}
#[derive(Clone, Serialize)]
pub struct Device { pub id: String, pub name: String }
#[derive(Clone, Serialize)]
pub struct GpuReading { pub device: Device, pub values: HashMap<Metric, Reading> }
#[derive(Clone, Serialize)]
pub struct Process { pub name: String, pub pid: u32, pub cpu: f64, pub memory: f64 }
#[derive(Clone, Serialize)]
pub struct Reading {
    pub value: Option<f64>,
    pub fraction: Option<f32>,
    pub text: String,
    pub source: String,
}
impl Reading {
    pub fn missing(reason: &str) -> Self {
        Self { value: None, fraction: None, text: "暂不可用".into(), source: reason.into() }
    }
    pub fn number(value: f64, text: String, fraction: Option<f32>, source: &str) -> Self {
        if !value.is_finite() { return Self::missing("返回了无效数据"); }
        Self { value: Some(value), fraction: fraction.map(|v| v.clamp(0.0, 1.0)), text, source: source.into() }
    }
    pub fn percent(value: f64, source: &str) -> Self {
        Self::number(value, format!("{value:.1}%"), Some(value as f32 / 100.0), source)
    }
    pub fn gib(bytes: u64, total: Option<u64>, source: &str) -> Self {
        let value = bytes as f64 / 1073741824.0;
        Self::number(value, format!("{value:.2} GB"), total.filter(|n| *n > 0).map(|n| bytes as f32 / n as f32), source)
    }
    pub fn rate(bytes: f64, source: &str) -> Self {
        Self::number(bytes / 1048576.0, rate_text(bytes), None, source)
    }
}
pub fn rate_text(bytes: f64) -> String {
    if bytes >= 1048576.0 { format!("{:.2} MB/s", bytes / 1048576.0) }
    else { format!("{:.1} KB/s", bytes / 1024.0) }
}
