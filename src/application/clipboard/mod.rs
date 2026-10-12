#[cfg(test)]
pub(crate) mod tests;
use crate::{
    domain::clipboard::{ClipboardConfig, History},
    interfaces::{ClipboardBackend, ClipboardEvent, ClipboardWatch, Wake},
};
use std::sync::{
    Arc,
    mpsc::{self, Receiver},
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum State {
    Off,
    Starting,
    Listening,
    Stopping,
}
struct CopyJob {
    rx: Receiver<Result<(), String>>,
    worker: std::thread::JoinHandle<()>,
}
pub struct ClipboardService {
    backend: Arc<dyn ClipboardBackend>,
    wake: Wake,
    watch: Option<Box<dyn ClipboardWatch>>,
    retiring: bool,
    resume: bool,
    copy: Option<CopyJob>,
    pub history: History,
    pub config: ClipboardConfig,
    pub state: State,
    pub message: String,
    pub failed: bool,
}
impl ClipboardService {
    pub fn new(
        backend: Arc<dyn ClipboardBackend>,
        wake: Wake,
        mut config: ClipboardConfig,
    ) -> Self {
        config.normalize();
        Self {
            backend,
            wake,
            watch: None,
            retiring: false,
            resume: false,
            copy: None,
            history: History::default(),
            config,
            state: State::Off,
            message: String::new(),
            failed: false,
        }
    }
    pub fn copying(&self) -> bool {
        self.copy.is_some()
    }
    pub fn start(&mut self) {
        if self.retiring {
            self.resume = true;
            return;
        }
        if self.watch.is_some() {
            return;
        }
        match self.backend.watch(self.config.bytes(), self.wake.clone()) {
            Ok(watch) => {
                self.watch = Some(watch);
                self.state = State::Starting;
                self.message = "正在开启，新复制的文字和图片才会被记录".into();
                self.failed = false;
            }
            Err(e) => {
                self.message = e;
                self.failed = true;
                self.state = State::Off;
            }
        }
    }
    pub fn pause(&mut self) {
        self.resume = false;
        if let Some(watch) = &mut self.watch {
            watch.stop();
            self.retiring = true;
            self.state = State::Stopping;
            self.message = "正在停止记录，已有历史保留".into();
        } else {
            self.state = State::Off;
        }
    }
    pub fn configure(&mut self, config: &mut ClipboardConfig) {
        config.normalize();
        if *config == self.config {
            return;
        }
        if let Err(e) = self.history.trim(config) {
            *config = self.config.clone();
            self.message = e;
            self.failed = true;
            return;
        }
        let restart = self.watch.is_some() && !self.retiring && config.mib != self.config.mib;
        self.config = config.clone();
        self.message = "容量设置已应用，固定内容保留".into();
        self.failed = false;
        if restart {
            self.pause();
            self.resume = true;
        }
    }
    pub fn copy(&mut self, id: u64) {
        if self.copying() {
            return;
        }
        let Some(item) = self.history.items.iter().find(|i| i.id == id) else {
            self.message = "条目已不存在".into();
            self.failed = true;
            return;
        };
        let content = item.packet.content.clone();
        let backend = self.backend.clone();
        let wake = self.wake.clone();
        let (tx, rx) = mpsc::channel();
        match std::thread::Builder::new()
            .name("clipboard-copy".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    backend.copy(&content)
                }))
                .unwrap_or_else(|_| Err("复制任务异常，请重试".into()));
                let _ = tx.send(result);
                wake();
            }) {
            Ok(worker) => {
                self.copy = Some(CopyJob { rx, worker });
                self.message = "正在复制原始内容…".into();
                self.failed = false;
            }
            Err(e) => {
                self.message = format!("复制无法启动：{e}");
                self.failed = true;
            }
        }
    }
    pub fn tick(&mut self) {
        let mut event = None;
        let mut finished = false;
        if let Some(watch) = &mut self.watch {
            if !self.retiring {
                event = watch.next();
            } else {
                while watch.next().is_some() {}
            }
            finished = watch.finished();
            if finished && !self.retiring && event.is_none() {
                event = watch.next();
            }
        }
        if let Some(event) = event {
            self.accept(event);
        }
        if finished {
            self.watch = None;
            self.state = State::Off;
            if self.retiring {
                self.message = "记录已暂停，历史保留".into();
                self.failed = false;
            } else if !self.failed {
                self.message = "记录已停止，请查看错误或重新开启".into();
                self.failed = true;
            } else {
                self.message = format!("记录已停止：{}", self.message);
            }
            self.retiring = false;
            if std::mem::take(&mut self.resume) {
                self.start();
            }
        }
        if let Some(job) = &self.copy
            && job.worker.is_finished()
        {
            let result = match job.rx.try_recv() {
                Ok(r) => Some(r),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => Some(Err("复制结果通道中断".into())),
            };
            if let Some(result) = result {
                let job = self.copy.take().unwrap();
                let _ = job.worker.join();
                self.failed = result.is_err();
                self.message =
                    result.map_or_else(|e| e, |()| "已复制原始内容，可在目标程序中粘贴".into());
            }
        }
    }
    pub fn stopping(&self) -> bool {
        self.retiring
    }
    fn accept(&mut self, event: ClipboardEvent) {
        match event {
            ClipboardEvent::Ready => {
                self.state = State::Listening;
                self.message = "记录已开启，只保存在本次运行的内存中".into();
                self.failed = false;
            }
            ClipboardEvent::Item(packet) => {
                self.state = State::Listening;
                let result =
                    self.history
                        .insert(packet, crate::application::office::now(), &self.config);
                self.failed = result.is_err();
                self.message = result.map_or_else(|e| e, |()| "已记录新复制内容".into());
            }
            ClipboardEvent::Failed(error) => {
                self.message = error;
                self.failed = true;
            }
        }
    }
}
impl Drop for ClipboardService {
    fn drop(&mut self) {
        if let Some(w) = &mut self.watch {
            w.stop();
        }
        if let Some(j) = self.copy.take()
            && j.worker.is_finished()
        {
            let _ = j.worker.join();
        }
    }
}
