use super::*;
use crate::domain::update::Manifest;
struct Fake {
    panic: bool,
}
fn release() -> Release {
    Release {
        version: "99.0.0".into(),
        notes: "release".into(),
        url: "https://github.com/xiaoyanse/desktop-pet/releases/download/v99.0.0/file.zip".into(),
        page: String::new(),
        source: "GitHub".into(),
        manifest: Manifest {
            schema: 1,
            version: "99.0.0".into(),
            file_name: "DesktopMeter-99.0.0-win-x64.zip".into(),
            size: 100,
            sha256: "a".repeat(64),
            executable_sha256: "b".repeat(64),
            delta: None,
        },
    }
}
impl UpdateBackend for Fake {
    fn check(&self, _: &Preferences) -> Result<Option<Release>, String> {
        if self.panic {
            panic!("fixture");
        }
        Ok(Some(release()))
    }
    fn download(&self, _: &Release, cancel: &AtomicBool, _: &AtomicU64) -> Result<(), String> {
        let started = Instant::now();
        while !cancel.load(Ordering::Relaxed) && started.elapsed() < Duration::from_secs(2) {
            std::thread::sleep(Duration::from_millis(1));
        }
        Err("cancelled".into())
    }
    fn launch_install(&self) -> Result<(), String> {
        Ok(())
    }
    fn schedule_install(&self) -> Result<(), String> {
        Ok(())
    }
}
fn wait(service: &mut UpdateService, prefs: &Preferences) {
    let start = Instant::now();
    while service.busy() && start.elapsed() < Duration::from_secs(3) {
        service.tick(prefs);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!service.busy());
}
#[test]
fn channel_result_is_consumed_once_and_cancel_returns_to_available() {
    let prefs = Preferences {
        automatic: false,
        ..Default::default()
    };
    let mut service = UpdateService::new(Arc::new(Fake { panic: false }), Arc::new(|| {}));
    service.check(&prefs);
    wait(&mut service, &prefs);
    assert!(matches!(service.state, State::Available(_)));
    service.download();
    assert!(service.busy());
    service.cancel();
    wait(&mut service, &prefs);
    assert!(matches!(service.state, State::Available(_)));
}
#[test]
fn panic_is_recoverable_and_pin_and_skip_do_not_download_silently() {
    let mut prefs = Preferences {
        automatic: false,
        pinned: true,
        ..Default::default()
    };
    let mut service = UpdateService::new(Arc::new(Fake { panic: true }), Arc::new(|| {}));
    service.check(&prefs);
    assert!(!service.busy());
    prefs.pinned = false;
    service.check(&prefs);
    wait(&mut service, &prefs);
    assert!(matches!(service.state, State::Failed(_)));
    let mut service = UpdateService::new(Arc::new(Fake { panic: false }), Arc::new(|| {}));
    prefs.skipped = "99.0.0".into();
    service.check(&prefs);
    wait(&mut service, &prefs);
    assert!(matches!(service.state, State::Latest));
}
#[test]
fn idle_service_stops_extra_repaint_scheduling_after_startup() {
    let prefs = Preferences {
        automatic: false,
        ..Default::default()
    };
    let mut service = UpdateService::new(Arc::new(Fake { panic: false }), Arc::new(|| {}));
    service.started = Instant::now() - Duration::from_secs(10);
    service.tick(&prefs);
    assert!(service.next_wakeup().is_none());
    assert!(!service.busy());
}
