use super::*;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
pub struct Fake {
    pub calls: AtomicUsize,
    pub result: Mutex<Result<MicrophoneStatus, String>>,
}
impl MicrophoneSource for Fake {
    fn sample(&self) -> Result<MicrophoneStatus, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.result.lock().unwrap().clone()
    }
}
pub fn fake_service() -> (MeetingService, Arc<Fake>) {
    let source = Arc::new(Fake {
        calls: AtomicUsize::new(0),
        result: Mutex::new(Ok(MicrophoneStatus {
            name: "合成麦克风".into(),
            muted: false,
            volume: Some(70),
        })),
    });
    (MeetingService::new(source.clone(), Arc::new(|| {})), source)
}
pub fn settle(service: &mut MeetingService, config: &MeetingConfig) {
    for _ in 0..100 {
        service.tick(Instant::now(), false, config);
        if !service.busy() {
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("microphone query did not settle");
}
#[test]
fn dormant_service_does_not_read_audio_status_and_page_poll_is_throttled() {
    let (mut service, source) = fake_service();
    let config = MeetingConfig::default();
    let now = Instant::now();
    service.tick(now, false, &config);
    assert_eq!(source.calls.load(Ordering::SeqCst), 0);
    assert!(service.next_wakeup(now, false).is_none());
    service.tick(now, true, &config);
    settle(&mut service, &config);
    for _ in 0..5 {
        service.tick(Instant::now(), true, &config);
    }
    assert_eq!(source.calls.load(Ordering::SeqCst), 1);
    assert!(service.microphone.as_ref().unwrap().is_ok());
}
#[test]
fn pause_resume_finish_and_failed_microphone_do_not_fake_mute_state() {
    let (mut service, source) = fake_service();
    let config = MeetingConfig {
        minutes: 1,
        ..Default::default()
    };
    let now = Instant::now();
    service.start(&config, now - Duration::from_secs(90));
    service.pause(now, &config);
    assert_eq!(service.session.elapsed.as_secs(), 90);
    assert_eq!(service.session.state, State::Paused);
    service.tick(now + Duration::from_secs(20), false, &config);
    assert_eq!(service.session.elapsed.as_secs(), 90);
    service.resume(now + Duration::from_secs(20));
    service.finish(now + Duration::from_secs(30), &config);
    assert_eq!(service.session.elapsed.as_secs(), 100);
    assert!(service.notices.is_empty());
    *source.result.lock().unwrap() = Err("no endpoint".into());
    service.refresh();
    service.tick(Instant::now(), true, &config);
    settle(&mut service, &config);
    assert!(service.microphone.as_ref().unwrap().is_err());
    service.reset();
    assert_eq!(service.session.state, State::Idle);
}
#[test]
fn mute_prompt_uses_fresh_status_and_silent_mode_clears_notifications() {
    for muted in [false, true] {
        let (mut service, source) = fake_service();
        source.result.lock().unwrap().as_mut().unwrap().muted = muted;
        let mut config = MeetingConfig {
            remind_mute: true,
            mute_minutes: 1,
            ..Default::default()
        };
        let now = Instant::now();
        service.start(&config, now - Duration::from_secs(61));
        service.tick(now, false, &config);
        settle(&mut service, &config);
        assert_eq!(service.notices.contains(&Notice::Microphone), !muted);
        service.tick(Instant::now(), false, &config);
        assert!(service.notices.len() <= 1);
        config.reminder = Reminder::Off;
        service.tick(Instant::now(), false, &config);
        assert!(service.notices.is_empty());
        service.finish(Instant::now(), &config);
        assert!(service.next_wakeup(Instant::now(), false).is_none());
    }
}
