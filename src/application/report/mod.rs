#[cfg(test)]
mod tests;
use crate::{
    domain::{config::Config, report::Report},
    interfaces::{ReportCollector, ReportStore, Wake},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, TryRecvError},
};

pub struct Document {
    pub report: Report,
    pub markdown: String,
}
enum ResultEvent {
    Generated(Document),
    Saved(String),
    Failed(String),
    Cancelled,
}
struct Job {
    rx: Receiver<ResultEvent>,
    worker: std::thread::JoinHandle<()>,
    cancel: Arc<AtomicBool>,
    collecting: bool,
}
pub struct ReportService {
    collector: Arc<dyn ReportCollector>,
    store: Arc<dyn ReportStore>,
    wake: Wake,
    job: Option<Job>,
    pub document: Option<Document>,
    pub message: String,
}
impl ReportService {
    pub fn new(
        collector: Arc<dyn ReportCollector>,
        store: Arc<dyn ReportStore>,
        wake: Wake,
    ) -> Self {
        Self {
            collector,
            store,
            wake,
            job: None,
            document: None,
            message: String::new(),
        }
    }
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
    pub fn can_cancel(&self) -> bool {
        self.job.as_ref().is_some_and(|j| j.collecting)
    }
    pub fn collect(&mut self, config: &Config) {
        if self.busy() {
            return;
        }
        let backend = self.collector.clone();
        let config = config.clone();
        self.message = "正在采集约 2 秒的资源与进程快照…".into();
        self.start(true, move |cancel| match backend.collect(&config, cancel) {
            Ok(report) => ResultEvent::Generated(Document {
                markdown: report.markdown(),
                report,
            }),
            Err(e) => ResultEvent::Failed(format!("采集失败：{e}")),
        });
    }
    pub fn save(&mut self) {
        if self.busy() {
            return;
        }
        let Some(document) = &self.document else {
            return;
        };
        let text = document.markdown.clone();
        let store = self.store.clone();
        self.message = "正在保存当前预览…".into();
        self.start(false, move |_| match store.save(&text) {
            Ok(path) => ResultEvent::Saved(format!("已保存：{path}")),
            Err(e) => ResultEvent::Failed(format!("保存失败：{e}")),
        });
    }
    fn start(
        &mut self,
        collecting: bool,
        task: impl FnOnce(&AtomicBool) -> ResultEvent + Send + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let wake = self.wake.clone();
        let worker = std::thread::Builder::new()
            .name("diagnostic-report".into())
            .spawn(move || {
                let event =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| task(&worker_cancel)))
                        .unwrap_or_else(|_| {
                            ResultEvent::Failed("任务异常退出，请重试；已有预览仍保留".into())
                        });
                let event = if collecting && worker_cancel.load(Ordering::Relaxed) {
                    ResultEvent::Cancelled
                } else {
                    event
                };
                let _ = tx.send(event);
                wake();
            });
        match worker {
            Ok(worker) => {
                self.job = Some(Job {
                    rx,
                    worker,
                    cancel,
                    collecting,
                })
            }
            Err(_) => self.message = "任务无法启动，请稍后重试；已有预览仍保留".into(),
        }
    }
    pub fn cancel(&mut self) {
        if let Some(job) = &self.job
            && job.collecting
        {
            job.cancel.store(true, Ordering::Relaxed);
            self.message = "正在取消采集…".into();
        }
    }
    pub fn tick(&mut self) {
        let Some(job) = &self.job else {
            return;
        };
        let mut result = match job.rx.try_recv() {
            Ok(event) => event,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                ResultEvent::Failed("任务中断，请重试；已有预览仍保留".into())
            }
        };
        if job.collecting && job.cancel.load(Ordering::Relaxed) {
            result = ResultEvent::Cancelled;
        }
        if let Some(job) = self.job.take() {
            let _ = job.worker.join();
        }
        match result {
            ResultEvent::Generated(document) => {
                self.message = "报告已生成；可以预览、复制或保存".into();
                self.document = Some(document);
            }
            ResultEvent::Saved(message) | ResultEvent::Failed(message) => self.message = message,
            ResultEvent::Cancelled => self.message = "采集已取消，已有预览仍保留".into(),
        }
    }
}
impl Drop for ReportService {
    fn drop(&mut self) {
        if let Some(job) = self.job.take() {
            job.cancel.store(true, Ordering::Relaxed);
            let _ = job.worker.join();
        }
    }
}
