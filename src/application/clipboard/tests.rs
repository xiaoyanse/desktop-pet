use super::*;
use crate::domain::{
    clipboard::{Content, Packet},
    screenshot::Image,
};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
pub struct Fake {
    pub starts: AtomicUsize,
    pub events: Arc<Mutex<Vec<ClipboardEvent>>>,
    pub done: Arc<AtomicBool>,
    pub fail_copy: AtomicBool,
    pub copies: Mutex<Vec<Content>>,
    pub delay_stop: Arc<AtomicBool>,
}
struct Watch {
    events: Arc<Mutex<Vec<ClipboardEvent>>>,
    done: Arc<AtomicBool>,
    delay_stop: Arc<AtomicBool>,
}
impl ClipboardWatch for Watch {
    fn next(&mut self) -> Option<ClipboardEvent> {
        let mut e = self.events.lock().unwrap();
        if e.is_empty() {
            None
        } else {
            Some(e.remove(0))
        }
    }
    fn stop(&mut self) {
        if !self.delay_stop.load(Ordering::Acquire) {
            self.done.store(true, Ordering::Release);
        }
    }
    fn finished(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }
}
impl ClipboardBackend for Fake {
    fn watch(&self, _: usize, _: Wake) -> Result<Box<dyn ClipboardWatch>, String> {
        self.starts.fetch_add(1, Ordering::Relaxed);
        self.done.store(false, Ordering::Release);
        self.events.lock().unwrap().push(ClipboardEvent::Ready);
        Ok(Box::new(Watch {
            events: self.events.clone(),
            done: self.done.clone(),
            delay_stop: self.delay_stop.clone(),
        }))
    }
    fn copy(&self, content: &Content) -> Result<(), String> {
        if self.fail_copy.load(Ordering::Relaxed) {
            return Err("合成写入失败".into());
        }
        self.copies.lock().unwrap().push(content.clone());
        Ok(())
    }
}
pub fn fake_service() -> (ClipboardService, Arc<Fake>) {
    let fake = Arc::new(Fake {
        starts: AtomicUsize::new(0),
        events: Arc::new(Mutex::new(vec![])),
        done: Arc::new(AtomicBool::new(false)),
        fail_copy: AtomicBool::new(false),
        copies: Mutex::new(vec![]),
        delay_stop: Arc::new(AtomicBool::new(false)),
    });
    (
        ClipboardService::new(fake.clone(), Arc::new(|| {}), ClipboardConfig::default()),
        fake,
    )
}
#[test]
fn changing_budget_waits_for_old_listener_and_retains_specific_failure() {
    let (mut service, source) = fake_service();
    service.start();
    service.tick();
    source.delay_stop.store(true, Ordering::Release);
    let mut config = ClipboardConfig { count: 30, mib: 4 };
    service.configure(&mut config);
    service.tick();
    assert!(service.stopping());
    assert_eq!(source.starts.load(Ordering::Relaxed), 1);
    source.done.store(true, Ordering::Release);
    service.tick();
    service.tick();
    assert_eq!(source.starts.load(Ordering::Relaxed), 2);
    source
        .events
        .lock()
        .unwrap()
        .push(ClipboardEvent::Failed("注册窗口失败的具体原因".into()));
    source.done.store(true, Ordering::Release);
    service.tick();
    assert!(service.state == State::Off);
    assert!(service.message.contains("注册窗口失败的具体原因"));
    assert!(service.failed);
}
#[test]
fn reducing_capacity_cannot_drop_pins_or_replace_previous_policy() {
    let (mut service, _) = fake_service();
    for text in ["固定一", "固定二"] {
        service
            .history
            .insert(Packet::text(text.into()).unwrap(), 1, &service.config)
            .unwrap();
    }
    service.history.pin(1);
    service.history.pin(2);
    let mut config = ClipboardConfig { count: 1, mib: 4 };
    service.configure(&mut config);
    assert_eq!(config.count, 30);
    assert_eq!(service.config.mib, 32);
    assert_eq!(service.history.items.len(), 2);
    assert!(service.failed);
}
#[test]
fn default_off_pause_discards_old_results_and_explicit_resume_works() {
    let (mut service, source) = fake_service();
    service.tick();
    assert_eq!(source.starts.load(Ordering::Relaxed), 0);
    service.start();
    service.tick();
    assert!(service.state == State::Listening);
    source.events.lock().unwrap().push(ClipboardEvent::Item(
        Packet::text("不要读入".into()).unwrap(),
    ));
    service.pause();
    service.tick();
    assert!(service.history.items.is_empty());
    assert!(service.state == State::Off);
    service.start();
    service.tick();
    assert_eq!(source.starts.load(Ordering::Relaxed), 2);
    source
        .events
        .lock()
        .unwrap()
        .push(ClipboardEvent::Item(Packet::text("新内容".into()).unwrap()));
    service.tick();
    assert_eq!(service.history.items.len(), 1);
}
#[test]
fn image_copy_uses_original_resolution_and_failure_is_not_success() {
    let (mut service, source) = fake_service();
    let image = Image::new([400, 200], vec![120; 400 * 200 * 4]).unwrap();
    service
        .history
        .insert(Packet::picture(image).unwrap(), 1, &service.config)
        .unwrap();
    for fail in [false, true] {
        source.fail_copy.store(fail, Ordering::Relaxed);
        service.copy(1);
        for _ in 0..100 {
            service.tick();
            if !service.copying() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(!service.copying());
        assert_eq!(service.failed, fail);
    }
    assert!(matches!(&source.copies.lock().unwrap()[0],Content::Picture(i) if i.size()==[400,200]));
}
