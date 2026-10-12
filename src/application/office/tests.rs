use super::*;
use crate::domain::office::Kind;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
pub struct Memory {
    pub book: Mutex<Book>,
    pub fail_save: AtomicBool,
    pub fail_load: AtomicBool,
}
impl OfficeStore for Memory {
    fn load(&self) -> Result<Book, String> {
        if self.fail_load.load(Ordering::SeqCst) {
            Err("corrupt".into())
        } else {
            Ok(self.book.lock().unwrap().clone())
        }
    }
    fn save(&self, book: &Book, previous: &Book) -> Result<(), String> {
        if self.fail_save.load(Ordering::SeqCst) {
            Err("disk full".into())
        } else {
            let mut original = self.book.lock().unwrap();
            if *original != *previous {
                return Err("outside change".into());
            }
            *original = book.clone();
            Ok(())
        }
    }
    fn location(&self) -> String {
        "E:\\合成验证\\workspace.json".into()
    }
}
pub struct Clock;
impl LocalTime for Clock {
    fn format(&self, ts: u64) -> String {
        ts.to_string()
    }
    fn parse(&self, text: &str) -> Result<u64, String> {
        text.parse().map_err(|_| "invalid".into())
    }
}
pub fn settle(service: &mut OfficeService, now: u64) {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while service.busy() && std::time::Instant::now() < deadline {
        service.tick(now);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!service.busy());
    service.tick(now);
}
pub fn fake_service() -> (OfficeService, Arc<Memory>) {
    let memory = Arc::new(Memory {
        book: Mutex::new(Book::default()),
        fail_save: AtomicBool::new(false),
        fail_load: AtomicBool::new(false),
    });
    let mut service = OfficeService::new(memory.clone(), Arc::new(Clock), Arc::new(|| {}));
    settle(&mut service, 0);
    (service, memory)
}
fn task(title: &str, due: u64) -> Command {
    Command::Save {
        id: None,
        kind: Kind::Task,
        title: title.into(),
        body: String::new(),
        pinned: false,
        due: Some(due),
    }
}
#[test]
fn failed_save_preserves_book_then_retry_and_reload_restore_it() {
    let (mut service, memory) = fake_service();
    memory.fail_save.store(true, Ordering::SeqCst);
    assert!(service.apply_at(task("中文待办", 10), 1));
    settle(&mut service, 1);
    assert!(service.book.items.is_empty());
    assert!(service.message.contains("保存失败"));
    memory.fail_save.store(false, Ordering::SeqCst);
    assert!(service.apply_at(task("中文待办", 10), 1));
    settle(&mut service, 1);
    let mut restored = OfficeService::new(memory, Arc::new(Clock), Arc::new(|| {}));
    settle(&mut restored, 1);
    assert_eq!(restored.book, service.book);
}
#[test]
fn failed_load_is_readonly_and_retry_does_not_overwrite() {
    let (_, memory) = fake_service();
    memory.fail_load.store(true, Ordering::SeqCst);
    let mut service = OfficeService::new(memory.clone(), Arc::new(Clock), Arc::new(|| {}));
    settle(&mut service, 1);
    assert!(!service.ready);
    assert!(!service.apply_at(task("不可写", 10), 1));
    memory.fail_load.store(false, Ordering::SeqCst);
    service.reload();
    settle(&mut service, 1);
    assert!(service.ready);
    assert!(memory.book.lock().unwrap().items.is_empty());
}
#[test]
fn queue_multiple_overdue_no_repeats_snooze_complete_and_restart_ack() {
    let (mut service, memory) = fake_service();
    for title in ["第一项", "第二项"] {
        assert!(service.apply_at(task(title, 10), 1));
        settle(&mut service, 1);
    }
    assert_eq!(service.next_wakeup(1), Some(Duration::from_secs(9)));
    service.tick(20);
    assert_eq!(service.reminder_count(), 2);
    for _ in 0..20 {
        service.tick(20);
    }
    assert_eq!(service.reminder_count(), 2);
    assert!(service.apply_at(Command::Snooze(1, 600), 20));
    settle(&mut service, 20);
    assert_eq!(service.notice().unwrap().id, 2);
    assert!(service.apply_at(Command::Dismiss(2), 20));
    settle(&mut service, 20);
    assert!(service.notice().is_none());
    let mut restored = OfficeService::new(memory, Arc::new(Clock), Arc::new(|| {}));
    settle(&mut restored, 620);
    assert_eq!(restored.reminder_count(), 1);
    assert!(restored.apply_at(Command::Complete(1), 620));
    settle(&mut restored, 620);
    assert!(restored.notice().is_none());
    assert!(restored.apply_at(Command::Complete(1), 621));
    settle(&mut restored, 621);
    assert_eq!(restored.reminder_count(), 1);
}
#[test]
fn dropping_service_finishes_explicit_save() {
    let (mut service, memory) = fake_service();
    assert!(service.apply_at(task("退出前保存", 100), 1));
    drop(service);
    assert_eq!(memory.book.lock().unwrap().items[0].title, "退出前保存");
}
