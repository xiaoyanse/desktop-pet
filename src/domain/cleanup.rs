use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Operation {
    LowStandby,
    WorkingSets,
    FileCache,
    VolumeCache,
    Modified,
    Standby,
    Registry,
    Combine,
}
impl Operation {
    pub const ALL: [Self; 8] = [
        Self::LowStandby,
        Self::WorkingSets,
        Self::FileCache,
        Self::VolumeCache,
        Self::Modified,
        Self::Standby,
        Self::Registry,
        Self::Combine,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::LowStandby => "低优先级待机缓存",
            Self::WorkingSets => "应用工作集",
            Self::FileCache => "系统文件缓存",
            Self::VolumeCache => "卷写入缓存",
            Self::Modified => "已修改页面列表",
            Self::Standby => "全部待机缓存",
            Self::Registry => "注册表缓存",
            Self::Combine => "合并相同内存页",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::LowStandby => {
                "只清除优先级 0 的待机页，作为默认选择。待机缓存本来就属于可用内存。"
            }
            Self::WorkingSets => "收缩应用驻留内存；应用继续工作时会重新调页，可能短暂变慢。",
            Self::FileCache => "收缩系统文件缓存工作集；后续文件访问可能重新读取磁盘。",
            Self::VolumeCache => "将各本地卷的待写数据刷到磁盘；属于落盘操作，可能增加磁盘活动。",
            Self::Modified => "将修改过的内存页写回存储，可能出现较明显的磁盘活动或停顿。",
            Self::Standby => "清除各优先级待机页；已有缓存会丢失，后续访问可能重新读取。",
            Self::Registry => "协调并刷新注册表缓存；结果由 Windows 原生接口决定。",
            Self::Combine => "请求 Windows 合并内容相同的物理内存页；效果取决于当前内存内容。",
        }
    }
    pub fn bit(self) -> u32 {
        1 << Self::ALL.iter().position(|op| *op == self).unwrap()
    }
    pub fn auto_sensitive(self) -> bool {
        matches!(self, Self::Modified | Self::Standby)
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CleanupConfig {
    pub operations: Vec<Operation>,
    pub auto_load: bool,
    pub load_percent: u32,
    pub auto_interval: bool,
    pub interval_minutes: u64,
    pub cooldown_minutes: u64,
    pub allow_sensitive_auto: bool,
    pub hotkey: bool,
}
impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            operations: vec![Operation::LowStandby],
            auto_load: false,
            load_percent: 90,
            auto_interval: false,
            interval_minutes: 30,
            cooldown_minutes: 5,
            allow_sensitive_auto: false,
            hotkey: false,
        }
    }
}
impl CleanupConfig {
    pub fn normalize(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.operations.retain(|o| seen.insert(*o));
        self.load_percent = self.load_percent.clamp(1, 100);
        self.interval_minutes = self.interval_minutes.clamp(1, 1440);
        self.cooldown_minutes = self.cooldown_minutes.clamp(1, 1440);
    }
    pub fn selected(&self, automatic: bool) -> Vec<Operation> {
        self.operations
            .iter()
            .copied()
            .filter(|o| !automatic || self.allow_sensitive_auto || !o.auto_sensitive())
            .collect()
    }
    pub fn due(&self, load: u32, elapsed_seconds: u64) -> bool {
        elapsed_seconds >= self.cooldown_minutes * 60
            && ((self.auto_load && load >= self.load_percent)
                || (self.auto_interval && elapsed_seconds >= self.interval_minutes * 60))
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct MemoryStatus {
    pub total: u64,
    pub available: u64,
    pub load: u32,
    pub commit: u64,
    pub commit_limit: u64,
    pub cache: u64,
    pub standby_low: Option<u64>,
    pub standby_all: Option<u64>,
    pub modified: Option<u64>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct OperationResult {
    pub operation: Operation,
    pub success: bool,
    pub detail: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct CleanupReport {
    #[serde(default)]
    pub time_label: String,
    pub at: u64,
    pub automatic: bool,
    pub before: MemoryStatus,
    pub after: Option<MemoryStatus>,
    #[serde(default)]
    pub measurement_error: Option<String>,
    pub results: Vec<OperationResult>,
}
impl CleanupReport {
    pub fn summary(&self) -> String {
        let successes = self.results.iter().filter(|r| r.success).count();
        let Some(after) = &self.after else {
            return format!(
                "{} / {} 项成功 · 操作后读数失败：{}",
                successes,
                self.results.len(),
                self.measurement_error.as_deref().unwrap_or("未知错误")
            );
        };
        format!(
            "{} / {} 项成功 · 可用 {:.2} → {:.2} GB · 变化 {:+.2} GB",
            successes,
            self.results.len(),
            self.before.available as f64 / 1073741824.0,
            after.available as f64 / 1073741824.0,
            (after.available as f64 - self.before.available as f64) / 1073741824.0
        )
    }
}
pub fn decode_mask(mask: u32) -> Option<Vec<Operation>> {
    (mask != 0 && mask & !255 == 0).then(|| {
        Operation::ALL
            .into_iter()
            .filter(|o| mask & o.bit() != 0)
            .collect()
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_do_not_clean_automatically_and_full_mask_contains_all_eight_operations() {
        let config = CleanupConfig::default();
        assert!(!config.due(100, 86400));
        assert_eq!(decode_mask(255).unwrap(), Operation::ALL);
        assert!(decode_mask(256).is_none());
        assert!(decode_mask(0).is_none());
    }
    #[test]
    fn threshold_and_interval_respect_cooldown_and_sensitive_auto_opt_in() {
        let mut config = CleanupConfig {
            operations: Operation::ALL.to_vec(),
            auto_load: true,
            ..Default::default()
        };
        assert!(!config.due(100, 299));
        assert!(config.due(90, 300));
        assert!(!config.due(89, 300));
        assert_eq!(config.selected(true).len(), 6);
        assert_eq!(config.selected(false).len(), 8);
        config.allow_sensitive_auto = true;
        assert_eq!(config.selected(true).len(), 8);
        config.auto_load = false;
        config.auto_interval = true;
        assert!(!config.due(100, 1799));
        assert!(config.due(1, 1800));
    }
}
