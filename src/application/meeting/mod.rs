use crate::{
    domain::{
        meeting::{MeetingConfig, MicrophoneStatus, Notice, Session},
        work_timer::{Reminder, State},
    },
    interfaces::{MicrophoneSource, Wake},
};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        mpsc::{self, Receiver},
    },
    time::{Duration, Instant},
};
#[cfg(test)]
pub(crate) mod tests;
struct Job {
    rx: Receiver<(Result<MicrophoneStatus, String>, Instant)>,
    worker: std::thread::JoinHandle<()>,
}
pub struct MeetingService {
    pub session: Session,
    pub microphone: Option<Result<MicrophoneStatus, String>>,
    pub microphone_at: Option<Instant>,
    pub notices: VecDeque<Notice>,
    source: Arc<dyn MicrophoneSource>,
    wake: Wake,
    job: Option<Job>,
    last_started: Option<Instant>,
    anchor: Option<Instant>,
    pending_mute: Option<Instant>,
}
impl MeetingService {
    pub fn new(source: Arc<dyn MicrophoneSource>, wake: Wake) -> Self {
        Self {
            session: Session::default(),
            microphone: None,
            microphone_at: None,
            notices: VecDeque::new(),
            source,
            wake,
            job: None,
            last_started: None,
            anchor: None,
            pending_mute: None,
        }
    }
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
    pub fn start(&mut self, config: &MeetingConfig, now: Instant) {
        if !self.session.active() {
            self.session.start(config);
            self.anchor = Some(now);
            self.notices.clear();
            self.pending_mute = None;
            self.refresh();
        }
    }
    pub fn pause(&mut self, now: Instant, config: &MeetingConfig) {
        self.advance(now, config);
        if self.session.state == State::Running {
            self.session.state = State::Paused;
            self.anchor = None;
            self.pending_mute = None;
        }
    }
    pub fn resume(&mut self, now: Instant) {
        if self.session.state == State::Paused {
            self.session.state = State::Running;
            self.anchor = Some(now);
        }
    }
    pub fn finish(&mut self, now: Instant, config: &MeetingConfig) {
        if self.session.active() {
            self.advance(now, config);
            self.session.state = State::Complete;
            self.anchor = None;
            self.notices.clear();
            self.pending_mute = None;
        }
    }
    pub fn reset(&mut self) {
        if !self.session.active() {
            self.session = Session::default();
            self.notices.clear();
            self.pending_mute = None;
        }
    }
    pub fn dismiss(&mut self) {
        self.notices.pop_front();
    }
    pub fn refresh(&mut self) {
        if !self.busy() {
            self.last_started = None;
        }
    }
    fn push(&mut self, notice: Notice) {
        if !self.notices.contains(&notice) && self.notices.len() < 2 {
            self.notices.push_back(notice);
        }
    }
    fn advance(&mut self, now: Instant, config: &MeetingConfig) {
        if self.session.state == State::Running
            && let Some(anchor) = self.anchor.replace(now)
        {
            let (time, microphone) = self.session.advance(now.saturating_duration_since(anchor));
            if config.reminder != Reminder::Off {
                if time {
                    self.push(Notice::Time);
                }
                if microphone {
                    self.pending_mute = Some(now);
                }
            }
        }
        if config.reminder == Reminder::Off {
            self.notices.clear();
            self.pending_mute = None;
        }
    }
    pub fn tick(&mut self, now: Instant, page_open: bool, config: &MeetingConfig) {
        self.advance(now, config);
        if let Some(job) = &self.job {
            let result = match job.rx.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some((Err("麦克风查询任务中断".into()), now))
                }
            };
            if let Some((result, captured)) = result {
                let job = self.job.take().unwrap();
                let _ = job.worker.join();
                if self.pending_mute.is_some_and(|at| captured >= at) {
                    self.pending_mute = None;
                    if self.session.state == State::Running
                        && config.reminder != Reminder::Off
                        && result.as_ref().is_ok_and(|mic| !mic.muted)
                    {
                        self.push(Notice::Microphone);
                    }
                }
                self.microphone = Some(result);
                self.microphone_at = Some(captured);
            }
        }
        let wanted = page_open || self.session.state == State::Running;
        if wanted
            && !self.busy()
            && (self.pending_mute.is_some()
                || self
                    .last_started
                    .is_none_or(|at| now.saturating_duration_since(at) >= Duration::from_secs(2)))
        {
            self.query(now);
        }
    }
    fn query(&mut self, now: Instant) {
        let (tx, rx) = mpsc::channel();
        let source = self.source.clone();
        let wake = self.wake.clone();
        self.last_started = Some(now);
        match std::thread::Builder::new()
            .name("meeting-status".into())
            .spawn(move || {
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| source.sample()))
                        .unwrap_or_else(|_| Err("麦克风状态读取异常".into()));
                let _ = tx.send((result, Instant::now()));
                wake();
            }) {
            Ok(worker) => self.job = Some(Job { rx, worker }),
            Err(error) => {
                self.microphone = Some(Err(format!("查询无法启动：{error}")));
                self.microphone_at = Some(now);
                self.pending_mute = None;
            }
        }
    }
    pub fn next_wakeup(&self, now: Instant, page_open: bool) -> Option<Duration> {
        if self.session.state == State::Running {
            Some(Duration::from_secs(1))
        } else if page_open {
            if self.busy() {
                Some(Duration::from_secs(2))
            } else {
                Some(
                    self.last_started
                        .map_or(Duration::from_millis(1), |at| {
                            Duration::from_secs(2).saturating_sub(now.saturating_duration_since(at))
                        })
                        .max(Duration::from_millis(20)),
                )
            }
        } else {
            None
        }
    }
}
impl Drop for MeetingService {
    fn drop(&mut self) {
        if let Some(job) = self.job.take() {
            let _ = job.worker.join();
        }
    }
}
