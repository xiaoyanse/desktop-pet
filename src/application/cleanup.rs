use crate::{
    domain::cleanup::{CleanupConfig, CleanupReport, MemoryStatus},
    interfaces::{CleanupHistory, MemoryCleaner, Wake},
};
use std::{sync::mpsc, time::Instant};

pub struct CleanupService {
    backend: Box<dyn MemoryCleaner>,
    history: Box<dyn CleanupHistory>,
    rx: Option<mpsc::Receiver<Result<CleanupReport, String>>>,
    pub reports: Vec<CleanupReport>,
    pub message: String,
    pub status: Option<MemoryStatus>,
    last_attempt: Instant,
    last_inspect: Option<Instant>,
    wake: Wake,
}

impl CleanupService {
    pub fn new(
        backend: Box<dyn MemoryCleaner>,
        history: Box<dyn CleanupHistory>,
        wake: Wake,
    ) -> Self {
        let (reports, message) = match history.load() {
            Ok(v) => (v, String::new()),
            Err(e) => (Vec::new(), format!("清理记录读取失败：{e}")),
        };
        Self {
            backend,
            history,
            rx: None,
            reports,
            message,
            status: None,
            last_attempt: Instant::now(),
            last_inspect: None,
            wake,
        }
    }
    pub fn busy(&self) -> bool {
        self.rx.is_some()
    }
    pub fn elevated(&self) -> bool {
        self.backend.elevated()
    }
    pub fn start(&mut self, config: &CleanupConfig, automatic: bool) {
        if self.busy() {
            return;
        }
        let operations = config.selected(automatic);
        if operations.is_empty() {
            self.message = "请至少选择一项清理方式；自动模式可能排除了全部已选项目".into();
            return;
        }
        if automatic && !self.elevated() {
            self.message = "自动清理需要以管理员身份运行，本次未执行".into();
            return;
        }
        self.last_attempt = Instant::now();
        self.rx = Some(
            self.backend
                .request(operations, automatic, self.wake.clone()),
        );
        self.message = if automatic {
            "自动清理执行中…"
        } else {
            "等待 Windows 授权及清理结果…"
        }
        .into();
    }
    pub fn tick(&mut self, config: &CleanupConfig, show_status: bool) {
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(Ok(report)) => {
                    self.message = report.summary();
                    self.status = report.after.clone();
                    self.reports.push(report);
                    if self.reports.len() > 50 {
                        self.reports.drain(..self.reports.len() - 50);
                    }
                    if let Err(e) = self.history.save(&self.reports) {
                        self.message.push_str(&format!("；记录保存失败：{e}"));
                    }
                    self.rx = None;
                }
                Ok(Err(e)) => {
                    self.message = e;
                    self.rx = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.message = "清理任务异常退出，未确认成功".into();
                    self.rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let auto = config.auto_load || config.auto_interval;
        if (show_status || auto) && self.last_inspect.is_none_or(|t| t.elapsed().as_secs() >= 2) {
            self.last_inspect = Some(Instant::now());
            match self.backend.inspect() {
                Ok(v) => self.status = Some(v),
                Err(e) => {
                    self.message = e;
                    return;
                }
            }
            if auto
                && self.elevated()
                && !self.busy()
                && self
                    .status
                    .as_ref()
                    .is_some_and(|s| config.due(s.load, self.last_attempt.elapsed().as_secs()))
            {
                self.start(config, true);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cleanup::{Operation, OperationResult};
    use std::{cell::RefCell, rc::Rc, sync::Arc};
    type ResultSender = mpsc::Sender<Result<CleanupReport, String>>;
    struct FakeCleaner {
        calls: Rc<RefCell<Vec<ResultSender>>>,
        fail_inspect: bool,
    }
    impl MemoryCleaner for FakeCleaner {
        fn request(
            &self,
            _: Vec<Operation>,
            _: bool,
            _: Wake,
        ) -> mpsc::Receiver<Result<CleanupReport, String>> {
            let (tx, rx) = mpsc::channel();
            self.calls.borrow_mut().push(tx);
            rx
        }
        fn elevated(&self) -> bool {
            true
        }
        fn inspect(&self) -> Result<MemoryStatus, String> {
            if self.fail_inspect {
                Err("读数失败".into())
            } else {
                Ok(MemoryStatus::default())
            }
        }
    }
    struct FakeHistory(Rc<RefCell<usize>>);
    impl CleanupHistory for FakeHistory {
        fn load(&self) -> Result<Vec<CleanupReport>, String> {
            Ok(Vec::new())
        }
        fn save(&self, reports: &[CleanupReport]) -> Result<(), String> {
            *self.0.borrow_mut() = reports.len();
            Ok(())
        }
    }
    #[test]
    fn cleanup_is_single_flight_and_retains_partial_success_without_after_reading() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let saved = Rc::new(RefCell::new(0));
        let mut service = CleanupService::new(
            Box::new(FakeCleaner {
                calls: calls.clone(),
                fail_inspect: false,
            }),
            Box::new(FakeHistory(saved.clone())),
            Arc::new(|| {}),
        );
        let config = CleanupConfig::default();
        service.start(&config, false);
        service.start(&config, false);
        assert_eq!(calls.borrow().len(), 1);
        calls.borrow()[0]
            .send(Ok(CleanupReport {
                time_label: "测试".into(),
                at: 1,
                automatic: false,
                before: MemoryStatus::default(),
                after: None,
                measurement_error: Some("读取失败".into()),
                results: vec![OperationResult {
                    operation: Operation::LowStandby,
                    success: true,
                    detail: "已执行".into(),
                }],
            }))
            .unwrap();
        service.tick(&config, false);
        assert!(!service.busy());
        assert_eq!(*saved.borrow(), 1);
        assert!(service.status.is_none());
        assert!(service.message.contains("1 / 1 项成功"));
        assert!(service.message.contains("读数失败"));
        assert!(!service.message.contains("变化"));
    }
    #[test]
    fn failed_inspection_does_not_trigger_auto_from_stale_status() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut service = CleanupService::new(
            Box::new(FakeCleaner {
                calls: calls.clone(),
                fail_inspect: true,
            }),
            Box::new(FakeHistory(Rc::new(RefCell::new(0)))),
            Arc::new(|| {}),
        );
        service.status = Some(MemoryStatus {
            load: 100,
            ..Default::default()
        });
        service.last_attempt = Instant::now() - std::time::Duration::from_secs(600);
        service.tick(
            &CleanupConfig {
                auto_load: true,
                ..Default::default()
            },
            false,
        );
        assert!(!service.busy());
        assert!(calls.borrow().is_empty());
        assert_eq!(service.message, "读数失败");
    }
}
