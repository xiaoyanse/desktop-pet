use super::*;
#[test]
fn deadline_notifies_once_but_meeting_keeps_counting_overtime() {
    let mut s = Session::default();
    s.start(&MeetingConfig {
        minutes: 1,
        ..Default::default()
    });
    assert_eq!(s.advance(Duration::from_secs(90)), (true, false));
    assert_eq!(s.state, State::Running);
    assert_eq!(s.overtime(), Duration::from_secs(30));
    assert_eq!(s.advance(Duration::from_secs(60)), (false, false));
    assert_eq!(s.elapsed.as_secs(), 150);
    s.state = State::Paused;
    s.advance(Duration::from_secs(500));
    assert_eq!(s.elapsed.as_secs(), 150);
}
#[test]
fn missed_mute_intervals_do_not_create_a_notification_backlog() {
    let mut s = Session::default();
    s.start(&MeetingConfig {
        remind_mute: true,
        mute_minutes: 1,
        ..Default::default()
    });
    assert!(s.advance(Duration::from_secs(500)).1);
    assert!(!s.advance(Duration::ZERO).1);
    assert!(!s.advance(Duration::from_secs(59)).1);
    assert!(s.advance(Duration::from_secs(1)).1);
}
#[test]
fn old_config_is_safe_and_running_lengths_are_frozen() {
    let mut config: MeetingConfig = serde_json::from_str("{}").unwrap();
    assert!(!config.remind_mute);
    config.minutes = u64::MAX;
    config.mute_minutes = 0;
    config.normalize();
    assert_eq!((config.minutes, config.mute_minutes), (1440, 1));
    let mut s = Session::default();
    s.start(&config);
    config.minutes = 10;
    assert_eq!(s.target.as_secs(), 86400);
    let c: MeetingConfig = serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
    assert_eq!(c.minutes, 10);
}
