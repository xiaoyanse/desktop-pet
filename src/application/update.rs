use crate::{
    domain::update::{Preferences, Release, newer},
    interfaces::{Wake, update::UpdateBackend},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{self, Receiver},
};
use std::time::{Duration, Instant};

pub enum State {
    Idle,
    Checking,
    Latest,
    Available(Release),
    Downloading(Release),
    Ready(Release),
    Failed(String),
}
enum ResultMessage {
    Checked(Result<Option<Release>, String>),
    Downloaded(Result<(), String>, Release),
}
pub struct UpdateService {
    backend: Arc<dyn UpdateBackend>,
    wake: Wake,
    rx: Option<Receiver<ResultMessage>>,
    cancel: Arc<AtomicBool>,
    progress: Arc<AtomicU64>,
    started: Instant,
    checked_at_startup: bool,
    acknowledged: bool,
    pub state: State,
    pub install_requested: bool,
    pub scheduled: bool,
}
impl UpdateService {
    pub fn new(backend: Arc<dyn UpdateBackend>, wake: Wake) -> Self {
        Self {
            backend,
            wake,
            rx: None,
            cancel: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(AtomicU64::new(0)),
            started: Instant::now(),
            checked_at_startup: false,
            acknowledged: false,
            state: State::Idle,
            install_requested: false,
            scheduled: false,
        }
    }
    pub fn busy(&self) -> bool {
        self.rx.is_some()
    }
    pub fn progress(&self) -> u64 {
        self.progress.load(Ordering::Relaxed)
    }
    pub fn tick(&mut self, prefs: &Preferences) {
        if !self.acknowledged && self.started.elapsed() >= Duration::from_secs(3) {
            self.acknowledged = true;
            if let Err(error) = self.backend.confirm_running() {
                self.state = State::Failed(error);
            }
        }
        let result = self.rx.as_ref().map(|rx| rx.try_recv());
        if result
            .as_ref()
            .is_some_and(|result| matches!(result, Err(mpsc::TryRecvError::Disconnected)))
        {
            self.rx = None;
            self.state = State::Failed("更新工作线程意外停止，请重新检查".into());
        }
        if let Some(Ok(msg)) = result {
            self.rx = None;
            self.state = match msg {
                ResultMessage::Checked(Ok(Some(r)))
                    if newer(&r.version, crate::domain::update::VERSION)
                        && (prefs.skipped.is_empty() || newer(&r.version, &prefs.skipped)) =>
                {
                    State::Available(r)
                }
                ResultMessage::Checked(Ok(_)) => State::Latest,
                ResultMessage::Checked(Err(e)) => State::Failed(e),
                ResultMessage::Downloaded(Ok(()), r) if self.cancel.load(Ordering::Relaxed) => {
                    State::Available(r)
                }
                ResultMessage::Downloaded(Ok(()), r) => State::Ready(r),
                ResultMessage::Downloaded(Err(_), r) if self.cancel.load(Ordering::Relaxed) => {
                    State::Available(r)
                }
                ResultMessage::Downloaded(Err(e), _) => State::Failed(e),
            };
        }
        if !self.checked_at_startup && self.started.elapsed() >= Duration::from_secs(5) {
            self.checked_at_startup = true;
            if prefs.automatic && !prefs.pinned {
                self.check(prefs);
            }
        }
    }
    pub fn check(&mut self, prefs: &Preferences) {
        if self.busy() || prefs.pinned {
            return;
        }
        self.state = State::Checking;
        let backend = self.backend.clone();
        let wake = self.wake.clone();
        let prefs = prefs.clone();
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        std::thread::spawn(move || {
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| backend.check(&prefs)))
                    .unwrap_or_else(|_| Err("更新检查意外停止，请重试".into()));
            let _ = tx.send(ResultMessage::Checked(result));
            wake();
        });
    }
    pub fn download(&mut self) {
        if self.busy() {
            return;
        }
        let State::Available(release) = &self.state else {
            return;
        };
        let release = release.clone();
        self.state = State::Downloading(release.clone());
        self.cancel.store(false, Ordering::Relaxed);
        self.progress.store(0, Ordering::Relaxed);
        let backend = self.backend.clone();
        let cancel = self.cancel.clone();
        let progress = self.progress.clone();
        let wake = self.wake.clone();
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                backend.download(&release, &cancel, &progress)
            }))
            .unwrap_or_else(|_| Err("更新下载意外停止，请重试".into()));
            let _ = tx.send(ResultMessage::Downloaded(result, release));
            wake();
        });
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
    pub fn next_wakeup(&self) -> Option<Duration> {
        if self.busy() {
            Some(Duration::from_millis(250))
        } else if !self.checked_at_startup || !self.acknowledged {
            Some(Duration::from_secs(1))
        } else {
            None
        }
    }
    pub fn install(&mut self) -> Result<(), String> {
        self.backend.launch_install()?;
        self.install_requested = false;
        Ok(())
    }
    pub fn schedule(&mut self) -> Result<(), String> {
        self.backend.schedule_install()?;
        self.scheduled = true;
        Ok(())
    }
}
impl Drop for UpdateService {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests;
