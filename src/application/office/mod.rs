#[cfg(test)]
pub(crate) mod tests;
use crate::{
    domain::office::{Book, Command, Item},
    interfaces::{LocalTime, OfficeStore, Wake},
};
use std::{
    collections::{HashSet, VecDeque},
    sync::{
        Arc,
        mpsc::{self, Receiver, TryRecvError},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
struct Job {
    rx: Receiver<Result<Book, String>>,
    worker: std::thread::JoinHandle<()>,
    loading: bool,
}
pub struct OfficeService {
    store: Arc<dyn OfficeStore>,
    pub clock: Arc<dyn LocalTime>,
    wake: Wake,
    job: Option<Job>,
    pub book: Book,
    pub revision: u64,
    pub saved_revision: u64,
    pub ready: bool,
    pub message: String,
    pub failed: bool,
    presented: HashSet<(u64, u64)>,
    notices: VecDeque<u64>,
}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
impl OfficeService {
    pub fn new(store: Arc<dyn OfficeStore>, clock: Arc<dyn LocalTime>, wake: Wake) -> Self {
        let mut service = Self {
            store,
            clock,
            wake,
            job: None,
            book: Book::default(),
            revision: 0,
            saved_revision: 0,
            ready: false,
            message: String::new(),
            failed: false,
            presented: HashSet::new(),
            notices: VecDeque::new(),
        };
        service.reload();
        service
    }
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
    pub fn location(&self) -> String {
        self.store.location()
    }
    pub fn reload(&mut self) {
        if self.busy() {
            return;
        }
        self.ready = false;
        self.message = "正在读取本地便签…".into();
        let store = self.store.clone();
        self.start(true, move || {
            let book = store.load()?;
            book.validate()?;
            Ok(book)
        });
    }
    pub fn apply(&mut self, command: Command) -> bool {
        self.apply_at(command, now())
    }
    pub fn apply_at(&mut self, command: Command, now: u64) -> bool {
        if self.busy() || !self.ready {
            return false;
        }
        let book = match self.book.changed(command, now) {
            Ok(book) => book,
            Err(error) => {
                self.message = error;
                self.failed = true;
                return false;
            }
        };
        let store = self.store.clone();
        self.message = "正在保存…".into();
        self.failed = false;
        let previous = self.book.clone();
        self.start(false, move || {
            store.save(&book, &previous)?;
            Ok(book)
        })
    }
    fn start(
        &mut self,
        loading: bool,
        task: impl FnOnce() -> Result<Book, String> + Send + 'static,
    ) -> bool {
        let (tx, rx) = mpsc::channel();
        let wake = self.wake.clone();
        match std::thread::Builder::new()
            .name("office-storage".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(task))
                    .unwrap_or_else(|_| Err("本地便签任务异常，原数据保留".into()));
                let _ = tx.send(result);
                wake();
            }) {
            Ok(worker) => {
                self.job = Some(Job {
                    rx,
                    worker,
                    loading,
                });
                true
            }
            Err(error) => {
                self.message = format!("本地任务无法启动：{error}");
                self.failed = true;
                false
            }
        }
    }
    pub fn tick(&mut self, now: u64) {
        if let Some(job) = &self.job {
            let result = match job.rx.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err("本地便签任务中断".into())),
            };
            if let Some(result) = result {
                let job = self.job.take().unwrap();
                let _ = job.worker.join();
                match result {
                    Ok(book) => {
                        self.book = book;
                        self.ready = true;
                        self.failed = false;
                        self.revision += 1;
                        if !job.loading {
                            self.saved_revision += 1;
                        }
                        self.message = if job.loading {
                            "本地便签已载入"
                        } else {
                            "已保存到本地"
                        }
                        .into();
                    }
                    Err(error) => {
                        self.failed = true;
                        self.message = format!(
                            "{}失败，原数据保留：{error}",
                            if job.loading { "读取" } else { "保存" }
                        )
                    }
                }
            }
        }
        if !self.ready {
            return;
        }
        self.notices.retain(|id| {
            self.book
                .items
                .iter()
                .any(|i| i.id == *id && i.needs_reminder(now))
        });
        self.presented.retain(|(id, due)| {
            self.book.items.iter().any(|i| {
                i.id == *id && !i.done && i.due == Some(*due) && i.acknowledged_due != Some(*due)
            })
        });
        for item in &self.book.items {
            if self.notices.len() >= 64 {
                break;
            }
            if item.needs_reminder(now) && self.presented.insert((item.id, item.due.unwrap())) {
                self.notices.push_back(item.id);
            }
        }
    }
    pub fn notice(&self) -> Option<&Item> {
        let id = self.notices.front()?;
        self.book.items.iter().find(|i| i.id == *id)
    }
    pub fn reminder_count(&self) -> usize {
        self.notices.len()
    }
    pub fn next_wakeup(&self, now: u64) -> Option<Duration> {
        if !self.ready {
            return None;
        }
        self.book
            .items
            .iter()
            .filter(|i| {
                !i.done
                    && i.due.is_some()
                    && i.acknowledged_due != i.due
                    && !self.presented.contains(&(i.id, i.due.unwrap()))
            })
            .map(|i| Duration::from_secs(i.due.unwrap().saturating_sub(now).clamp(1, 30)))
            .min()
    }
}
impl Drop for OfficeService {
    fn drop(&mut self) {
        if let Some(job) = self.job.take() {
            let _ = job.worker.join();
        }
    }
}
