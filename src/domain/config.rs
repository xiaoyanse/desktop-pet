use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Metric {
    Cpu,
    Frequency,
    Memory,
    Used,
    Available,
    Commit,
    Paged,
    Nonpaged,
    Gpu,
    Dedicated,
    Shared,
    DiskSpace,
    DiskFree,
    DiskBusy,
    DiskRead,
    DiskWrite,
    Upload,
    Download,
}
impl Metric {
    pub const ALL: [Self; 18] = [
        Self::Cpu,
        Self::Frequency,
        Self::Memory,
        Self::Used,
        Self::Available,
        Self::Commit,
        Self::Paged,
        Self::Nonpaged,
        Self::Gpu,
        Self::Dedicated,
        Self::Shared,
        Self::DiskSpace,
        Self::DiskFree,
        Self::DiskBusy,
        Self::DiskRead,
        Self::DiskWrite,
        Self::Upload,
        Self::Download,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Cpu => "CPU 占用",
            Self::Frequency => "CPU 频率",
            Self::Memory => "内存占用",
            Self::Used => "已用内存",
            Self::Available => "可用内存",
            Self::Commit => "提交内存",
            Self::Paged => "分页池",
            Self::Nonpaged => "非分页池",
            Self::Gpu => "GPU 占用",
            Self::Dedicated => "专用显存",
            Self::Shared => "共享显存",
            Self::DiskSpace => "磁盘空间",
            Self::DiskFree => "磁盘剩余",
            Self::DiskBusy => "磁盘活动",
            Self::DiskRead => "磁盘读取",
            Self::DiskWrite => "磁盘写入",
            Self::Upload => "网络上传",
            Self::Download => "网络下载",
        }
    }
    pub fn group(self) -> &'static str {
        match self {
            Self::Cpu | Self::Frequency => "CPU",
            Self::Memory
            | Self::Used
            | Self::Available
            | Self::Commit
            | Self::Paged
            | Self::Nonpaged => "内存",
            Self::Gpu | Self::Dedicated | Self::Shared => "GPU",
            Self::DiskSpace
            | Self::DiskFree
            | Self::DiskBusy
            | Self::DiskRead
            | Self::DiskWrite => "磁盘",
            Self::Upload | Self::Download => "网络",
        }
    }
    pub fn percent(self) -> bool {
        matches!(
            self,
            Self::Cpu | Self::Memory | Self::Gpu | Self::DiskSpace | Self::DiskBusy
        )
    }
    pub fn low_warning(self) -> bool {
        matches!(self, Self::Available | Self::DiskFree)
    }
    pub fn alert_supported(self) -> bool {
        matches!(
            self,
            Self::Cpu
                | Self::Memory
                | Self::Available
                | Self::Commit
                | Self::Dedicated
                | Self::DiskSpace
                | Self::DiskFree
        )
    }
    pub fn alert_percent(self) -> bool {
        matches!(
            self,
            Self::Cpu | Self::Memory | Self::Commit | Self::Dedicated | Self::DiskSpace
        )
    }
    pub fn alert_value(self, reading: &super::resources::Reading) -> Option<f64> {
        if !self.alert_supported() {
            return None;
        }
        if matches!(self, Self::Commit | Self::Dedicated) {
            reading.fraction.map(|f| f as f64 * 100.0)
        } else {
            reading.value
        }
    }
    pub fn exceeds(self, value: f64, threshold: f64) -> bool {
        self.alert_supported()
            && value.is_finite()
            && threshold.is_finite()
            && if self.low_warning() {
                value <= threshold
            } else {
                value >= threshold
            }
    }
    pub fn color_supported(self) -> bool {
        self.percent() || self.alert_supported()
    }
    pub fn color_percent(self) -> bool {
        self.percent() || self.alert_percent()
    }
    pub fn color_value(self, reading: &super::resources::Reading) -> Option<f64> {
        if self.percent() {
            reading.value
        } else {
            self.alert_value(reading)
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Row {
    pub metric: Metric,
    pub enabled: bool,
    pub trend: bool,
    pub threshold: f64,
    pub warning_threshold: f64,
    pub alert: bool,
}
impl Default for Row {
    fn default() -> Self {
        Self {
            metric: Metric::Cpu,
            enabled: true,
            trend: false,
            threshold: 90.0,
            warning_threshold: 70.0,
            alert: true,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub rows: Vec<Row>,
    pub seconds: u64,
    pub color: [u8; 3],
    pub opacity: f32,
    pub warnings: bool,
    pub status_colors: bool,
    pub compact: bool,
    pub pet_mode: bool,
    pub pet_kind: super::desktop::PetKind,
    pub pet_animation: bool,
    pub background: bool,
    pub border: bool,
    pub snap_edges: bool,
    pub position: Option<[f32; 2]>,
    pub gpu: String,
    pub disk: String,
    pub network: String,
    pub cleanup: super::cleanup::CleanupConfig,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            version: 3,
            rows: Metric::ALL
                .into_iter()
                .map(|metric| Row {
                    metric,
                    enabled: matches!(
                        metric,
                        Metric::Cpu
                            | Metric::Memory
                            | Metric::Gpu
                            | Metric::DiskRead
                            | Metric::DiskWrite
                            | Metric::Upload
                            | Metric::Download
                    ),
                    trend: false,
                    threshold: if metric.color_percent() {
                        90.0
                    } else if metric == Metric::Available {
                        2.0
                    } else {
                        8.0
                    },
                    warning_threshold: if metric.color_percent() {
                        70.0
                    } else if metric == Metric::Available {
                        4.0
                    } else {
                        16.0
                    },
                    alert: metric.alert_supported(),
                })
                .collect(),
            seconds: 2,
            color: [100, 210, 195],
            opacity: 0.90,
            warnings: true,
            status_colors: true,
            compact: false,
            pet_mode: false,
            pet_kind: super::desktop::PetKind::Cat,
            pet_animation: true,
            background: true,
            border: true,
            snap_edges: true,
            position: None,
            gpu: String::new(),
            disk: "C:".into(),
            network: String::new(),
            cleanup: super::cleanup::CleanupConfig::default(),
        }
    }
}
impl Config {
    pub fn normalize(&mut self) {
        if self.pet_mode {
            self.compact = false;
        }
        if ![1, 2, 5].contains(&self.seconds) {
            self.seconds = 2;
        }
        self.opacity = if self.opacity.is_finite() {
            self.opacity.clamp(0.15, 1.0)
        } else {
            0.90
        };
        if self
            .position
            .is_some_and(|p| p.iter().any(|v| !v.is_finite()))
        {
            self.position = None;
        }
        let mut seen = std::collections::HashSet::new();
        self.rows.retain(|r| seen.insert(r.metric));
        for metric in Metric::ALL {
            if !seen.contains(&metric) {
                self.rows.push(Row {
                    metric,
                    enabled: false,
                    ..Row::default()
                });
            }
        }
        for row in &mut self.rows {
            if self.version < 2 && matches!(row.metric, Metric::Commit | Metric::Dedicated) {
                row.threshold = 90.0;
            }
            if !row.threshold.is_finite() || row.threshold < 0.0 {
                row.threshold = 90.0;
            }
            if self.version < 3 {
                if row.metric.percent() && !row.metric.alert_supported() {
                    row.threshold = 90.0;
                }
                row.warning_threshold = if row.metric.low_warning() {
                    row.threshold * 2.0
                } else {
                    row.threshold * 7.0 / 9.0
                };
            }
            if row.metric.color_percent() {
                row.threshold = row.threshold.min(100.0);
            }
            if !row.warning_threshold.is_finite() {
                row.warning_threshold = row.threshold;
            }
            if row.metric.low_warning() {
                row.warning_threshold = row.warning_threshold.max(row.threshold);
            } else {
                row.warning_threshold = row.warning_threshold.clamp(0.0, row.threshold);
            }
            if !row.metric.alert_supported() {
                row.alert = false;
            }
        }
        self.version = 3;
        self.cleanup.normalize();
    }
    pub fn enabled(&self, group: &str) -> bool {
        self.rows
            .iter()
            .any(|r| r.enabled && r.metric.group() == group)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::desktop::PetKind;
    #[test]
    fn old_settings_default_pet_options_and_new_modes_roundtrip() {
        let mut old: Config = serde_json::from_str(r#"{"compact":true}"#).unwrap();
        assert!(!old.pet_mode);
        assert!(old.pet_animation);
        assert_eq!(old.pet_kind, PetKind::Cat);
        old.pet_mode = true;
        old.pet_kind = PetKind::Fox;
        old.pet_animation = false;
        old.normalize();
        assert!(!old.compact);
        let new: Config = serde_json::from_str(&serde_json::to_string(&old).unwrap()).unwrap();
        assert!(new.pet_mode);
        assert_eq!(new.pet_kind, PetKind::Fox);
        assert!(!new.pet_animation);
    }
    #[test]
    fn corrupted_settings_are_bounded_and_complete() {
        let mut c = Config {
            seconds: 0,
            opacity: f32::NAN,
            position: Some([f32::INFINITY, 0.0]),
            ..Config::default()
        };
        c.rows.push(c.rows[0].clone());
        c.normalize();
        assert_eq!(c.rows.len(), Metric::ALL.len());
        assert_eq!(c.seconds, 2);
        assert!(c.position.is_none());
        assert_eq!(c.opacity, 0.90);
    }
    #[test]
    fn setting_roundtrip_keeps_order_devices_and_toggles() {
        let mut c = Config::default();
        c.rows.swap(0, 4);
        c.rows[0].trend = true;
        c.gpu = "adapter".into();
        c.compact = true;
        c.background = false;
        c.border = false;
        c.snap_edges = false;
        let decoded: Config = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(decoded.rows[0].metric, Metric::Available);
        assert!(decoded.rows[0].trend);
        assert_eq!(decoded.gpu, "adapter");
        assert!(decoded.compact);
        assert!(!decoded.background);
        assert!(!decoded.border);
        assert!(!decoded.snap_edges);
    }
    #[test]
    fn version_two_migration_preserves_red_thresholds_and_adds_yellow() {
        let mut c = Config {
            version: 2,
            ..Config::default()
        };
        c.rows[0].threshold = 80.0;
        c.rows[0].warning_threshold = f64::NAN;
        c.normalize();
        assert_eq!(c.version, 3);
        assert_eq!(c.rows[0].threshold, 80.0);
        assert!((c.rows[0].warning_threshold - 80.0 * 7.0 / 9.0).abs() < 0.001);
        let row = c
            .rows
            .iter()
            .find(|r| r.metric == Metric::Available)
            .unwrap();
        assert!(row.warning_threshold >= row.threshold);
    }
    #[test]
    fn available_capacity_warns_when_low_not_when_high() {
        assert!(Metric::Available.exceeds(1.0, 2.0));
        assert!(!Metric::Available.exceeds(8.0, 2.0));
        assert!(Metric::Cpu.exceeds(99.0, 90.0));
    }
    #[test]
    fn activity_and_unbounded_values_never_alert() {
        for metric in [
            Metric::Frequency,
            Metric::Used,
            Metric::Paged,
            Metric::Nonpaged,
            Metric::Gpu,
            Metric::Shared,
            Metric::DiskBusy,
            Metric::DiskRead,
            Metric::DiskWrite,
            Metric::Upload,
            Metric::Download,
        ] {
            assert!(!metric.exceeds(100000.0, 0.0), "{metric:?}");
            assert!(
                metric
                    .alert_value(&crate::domain::resources::Reading::percent(100.0, "test"))
                    .is_none()
            );
        }
    }
    #[test]
    fn commit_and_vram_alert_against_capacity_not_gigabytes() {
        let reading = crate::domain::resources::Reading::gib(95 << 30, Some(100 << 30), "test");
        assert!(Metric::Commit.exceeds(Metric::Commit.alert_value(&reading).unwrap(), 90.0));
        let unbounded = crate::domain::resources::Reading::gib(100 << 30, None, "test");
        assert!(Metric::Dedicated.alert_value(&unbounded).is_none());
        let mut legacy = Config {
            version: 1,
            ..Config::default()
        };
        for row in &mut legacy.rows {
            row.threshold = 8.0;
            row.alert = true;
        }
        legacy.normalize();
        assert_eq!(
            legacy
                .rows
                .iter()
                .find(|r| r.metric == Metric::Commit)
                .unwrap()
                .threshold,
            90.0
        );
        assert!(
            !legacy
                .rows
                .iter()
                .find(|r| r.metric == Metric::Download)
                .unwrap()
                .alert
        );
    }
}
