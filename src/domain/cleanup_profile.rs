use super::cleanup::{CleanupConfig, Operation};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Profile {
    Light,
    Medium,
    Heavy,
}

impl Profile {
    pub const ALL: [Self; 3] = [Self::Light, Self::Medium, Self::Heavy];
    pub fn label(self) -> &'static str {
        match self {
            Self::Light => "轻度 · 日常推荐",
            Self::Medium => "中度 · 缓存整理",
            Self::Heavy => "重度 · 临时缓解",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Light => "保留应用工作集，只清低优先级待机缓存。适合工作进行中。",
            Self::Medium => "收缩文件缓存并清全部待机页。后续访问可能重新读取磁盘。",
            Self::Heavy => "额外收缩应用工作集、写回修改页。可能停顿，适合工作告一段落时。",
        }
    }
    pub fn operations(self) -> &'static [Operation] {
        use Operation::*;
        match self {
            Self::Light => &[LowStandby],
            Self::Medium => &[FileCache, Standby],
            Self::Heavy => &[WorkingSets, FileCache, Modified, Standby],
        }
    }
    pub fn detect(operations: &[Operation]) -> Option<Self> {
        Self::ALL.into_iter().find(|p| {
            operations.len() == p.operations().len()
                && p.operations().iter().all(|o| operations.contains(o))
        })
    }
}

#[derive(Default)]
pub struct Review {
    pending: Option<Vec<Operation>>,
}
impl Review {
    pub fn begin(&mut self, config: &CleanupConfig) {
        self.pending = (!config.operations.is_empty()).then(|| config.operations.clone());
    }
    pub fn operations(&self) -> Option<&[Operation]> {
        self.pending.as_deref()
    }
    pub fn cancel(&mut self) {
        self.pending = None;
    }
    pub fn take(&mut self) -> Option<CleanupConfig> {
        self.pending.take().map(|operations| CleanupConfig {
            operations,
            ..Default::default()
        })
    }
}

pub fn operation_names(operations: &[Operation]) -> String {
    operations
        .iter()
        .map(|o| o.label())
        .collect::<Vec<_>>()
        .join("、")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profiles_are_distinct_and_keep_flush_only_operations_out_of_recommendations() {
        assert_eq!(
            Profile::detect(&CleanupConfig::default().operations),
            Some(Profile::Light)
        );
        for p in Profile::ALL {
            assert_eq!(Profile::detect(p.operations()), Some(p));
            assert!(!p.operations().contains(&Operation::VolumeCache));
            assert!(!p.operations().contains(&Operation::Registry));
            assert!(!p.operations().contains(&Operation::Combine));
        }
        assert_eq!(
            Profile::Heavy.operations().last(),
            Some(&Operation::Standby)
        );
        assert_eq!(Profile::detect(&[Operation::Combine]), None);
        assert_eq!(Profile::detect(&[]), None);
    }
    #[test]
    fn confirmation_freezes_operations_and_cancel_or_empty_never_creates_a_request() {
        let mut review = Review::default();
        let mut config = CleanupConfig::default();
        review.begin(&config);
        config.operations = Profile::Heavy.operations().to_vec();
        assert_eq!(
            review.take().unwrap().operations,
            Profile::Light.operations()
        );
        assert!(review.take().is_none());
        review.begin(&config);
        review.cancel();
        assert!(review.take().is_none());
        config.operations.clear();
        review.begin(&config);
        assert!(review.take().is_none());
    }
}
