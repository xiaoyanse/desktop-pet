use super::*;
use std::{
    sync::atomic::AtomicUsize,
    time::{Duration, Instant},
};
struct Fake {
    calls: AtomicUsize,
    entered: mpsc::Sender<()>,
    release: std::sync::Mutex<Receiver<()>>,
}
impl ReportCollector for Fake {
    fn collect(&self, _: &Config, _: &AtomicBool) -> Result<Report, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.entered.send(()).unwrap();
        self.release.lock().unwrap().recv().unwrap();
        Ok(Report {
            captured_at: "fake".into(),
            ..Default::default()
        })
    }
}
struct Store(std::sync::Mutex<Vec<String>>);
impl ReportStore for Store {
    fn save(&self, text: &str) -> Result<String, String> {
        self.0.lock().unwrap().push(text.to_owned());
        Err("模拟不可写目录".into())
    }
}
fn finish(service: &mut ReportService) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while service.busy() {
        service.tick();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn single_flight_cancellation_and_failed_export_keep_the_exact_preview() {
    let (entered, arrived) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let backend = Arc::new(Fake {
        calls: AtomicUsize::new(0),
        entered,
        release: std::sync::Mutex::new(gate),
    });
    let store = Arc::new(Store(std::sync::Mutex::new(vec![])));
    let mut service = ReportService::new(backend.clone(), store.clone(), Arc::new(|| {}));
    service.collect(&Config::default());
    arrived.recv_timeout(Duration::from_secs(2)).unwrap();
    service.collect(&Config::default());
    service.save();
    assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
    release.send(()).unwrap();
    finish(&mut service);
    let preview = service.document.as_ref().unwrap().markdown.clone();
    service.collect(&Config::default());
    arrived.recv_timeout(Duration::from_secs(2)).unwrap();
    service.cancel();
    release.send(()).unwrap();
    finish(&mut service);
    assert!(service.message.contains("取消"));
    assert_eq!(service.document.as_ref().unwrap().markdown, preview);
    service.save();
    finish(&mut service);
    assert!(service.message.contains("保存失败"));
    assert_eq!(service.document.as_ref().unwrap().markdown, preview);
    assert_eq!(store.0.lock().unwrap().as_slice(), &[preview]);
}
