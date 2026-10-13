#[cfg(test)]
pub(crate) mod tests;
use crate::{
    domain::screenshot::{Document, Image, Mode},
    interfaces::{ScreenCapture, ScreenshotOutput, Wake},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, TryRecvError},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Output {
    Copy,
    Save,
    Pin,
}
enum Event {
    Captured(Image),
    Copied(Option<Image>),
    CopyFailed(Image, String),
    Pinned(Image),
    Message(String),
    Cancelled,
    Failed(String),
}
struct Job {
    rx: Receiver<Event>,
    worker: std::thread::JoinHandle<()>,
    cancel: Arc<AtomicBool>,
    capturing: bool,
}
pub struct ScreenshotService {
    capture: Arc<dyn ScreenCapture>,
    output: Arc<dyn ScreenshotOutput>,
    wake: Wake,
    job: Option<Job>,
    discard_after_job: bool,
    pub document: Option<Document>,
    pub pinned: Option<Image>,
    pub generation: u64,
    pub pin_generation: u64,
    pub folder: String,
    pub message: String,
}
impl ScreenshotService {
    pub fn new(
        capture: Arc<dyn ScreenCapture>,
        output: Arc<dyn ScreenshotOutput>,
        wake: Wake,
    ) -> Self {
        let folder = output.default_folder();
        Self {
            capture,
            output,
            wake,
            job: None,
            discard_after_job: false,
            document: None,
            pinned: None,
            generation: 0,
            pin_generation: 0,
            folder,
            message: String::new(),
        }
    }
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
    pub fn capturing(&self) -> bool {
        self.job.as_ref().is_some_and(|j| j.capturing)
    }
    #[cfg(test)]
    pub fn capture(&mut self, mode: Mode) {
        self.capture_with_output(mode, None);
    }
    pub fn capture_to_clipboard(&mut self, mode: Mode, keep_preview: bool) {
        self.capture_with_output(mode, Some(keep_preview));
    }
    fn capture_with_output(&mut self, mode: Mode, copy: Option<bool>) {
        if self.busy() {
            return;
        }
        let capture = self.capture.clone();
        let output = self.output.clone();
        self.message = format!("{}：Enter 确认，Esc 取消", mode.label());
        self.start(true, move |cancel| {
            let Some(selection) = capture.select(mode, cancel)? else {
                return Ok(Event::Cancelled);
            };
            if cancel.load(Ordering::Relaxed) {
                return Ok(Event::Cancelled);
            }
            capture.validate(&selection)?;
            let image = capture.capture(selection.region, cancel)?;
            capture.validate(&selection)?;
            let image = selection.apply_mask(image)?;
            if cancel.load(Ordering::Relaxed) {
                return Ok(Event::Cancelled);
            }
            if let Some(keep_preview) = copy {
                return Ok(match output.copy(&image) {
                    Ok(()) => Event::Copied(keep_preview.then_some(image)),
                    Err(error) => Event::CopyFailed(
                        image,
                        format!("截图复制失败：{error}；可在截图页重试复制"),
                    ),
                });
            }
            Ok(Event::Captured(image))
        });
    }
    pub fn export(&mut self, kind: Output) {
        if self.busy() {
            return;
        }
        let Some(document) = self.document.clone() else {
            self.message = "请先完成截图".into();
            return;
        };
        let output = self.output.clone();
        let folder = self.folder.clone();
        self.message = "正在生成批注结果…".into();
        self.start(false, move |_| {
            let image = document.render()?;
            match kind {
                Output::Copy => {
                    output.copy(&image)?;
                    Ok(Event::Message("图片与批注已复制".into()))
                }
                Output::Save => Ok(Event::Message(format!(
                    "已保存：{}",
                    output.save(&image, &folder)?
                ))),
                Output::Pin => Ok(Event::Pinned(image)),
            }
        });
    }
    pub fn discard(&mut self) {
        if !self.busy() {
            self.document = None;
            self.generation += 1;
            self.message = "预览已关闭，截图不会自动保存".into();
        } else if !self.capturing() {
            self.discard_after_job = true;
        }
    }
    pub fn cancel(&mut self) {
        if let Some(job) = &self.job
            && job.capturing
        {
            job.cancel.store(true, Ordering::Relaxed);
            self.message = "正在取消截图…".into();
        }
    }
    fn start(
        &mut self,
        capturing: bool,
        task: impl FnOnce(&AtomicBool) -> Result<Event, String> + Send + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let wake = self.wake.clone();
        match std::thread::Builder::new()
            .name("screenshot-task".into())
            .spawn(move || {
                let event =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| task(&worker_cancel)))
                        .unwrap_or_else(|_| Err("截图任务异常，请重试；原预览仍保留".into()))
                        .unwrap_or_else(Event::Failed);
                // Once clipboard copy commits, a late cancellation must not claim it was undone.
                let event = if capturing
                    && worker_cancel.load(Ordering::Relaxed)
                    && !matches!(event, Event::Copied(_))
                {
                    Event::Cancelled
                } else {
                    event
                };
                let _ = tx.send(event);
                wake();
            }) {
            Ok(worker) => {
                self.job = Some(Job {
                    rx,
                    worker,
                    cancel,
                    capturing,
                })
            }
            Err(_) => self.message = "截图任务无法启动，请重试".into(),
        }
    }
    pub fn tick(&mut self) {
        let Some(job) = &self.job else {
            return;
        };
        let mut event = match job.rx.try_recv() {
            Ok(e) => e,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Event::Failed("截图任务中断，原预览仍保留".into()),
        };
        if job.capturing && job.cancel.load(Ordering::Relaxed) && !matches!(event, Event::Copied(_))
        {
            event = Event::Cancelled;
        }
        if let Some(job) = self.job.take() {
            let _ = job.worker.join();
        }
        match event {
            Event::Copied(image) => {
                if let Some(image) = image {
                    self.document = Some(Document::new(image));
                    self.generation += 1;
                }
                self.message = "截图已复制，可直接粘贴".into();
            }
            Event::CopyFailed(image, message) => {
                self.document = Some(Document::new(image));
                self.generation += 1;
                self.message = message;
            }
            Event::Captured(image) => {
                self.document = Some(Document::new(image));
                self.generation += 1;
                self.message = "截图完成；批注后可复制、保存或贴图".into();
            }
            Event::Pinned(image) => {
                self.pinned = Some(image);
                self.pin_generation += 1;
                self.message = "已置顶贴图；再次贴图会替换当前图片".into();
            }
            Event::Message(message) | Event::Failed(message) => self.message = message,
            Event::Cancelled => self.message = "截图已取消，原预览仍保留".into(),
        }
        if std::mem::take(&mut self.discard_after_job) {
            self.document = None;
            self.generation += 1;
        }
    }
}
impl Drop for ScreenshotService {
    fn drop(&mut self) {
        if let Some(job) = self.job.take() {
            job.cancel.store(true, Ordering::Relaxed);
            let _ = job.worker.join();
        }
    }
}
