use crate::domain::work_timer::{Mode, Session, State, TimerConfig};
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct WorkTimer {
    pub session: Session,
    pub notice: bool,
    anchor: Option<Instant>,
}
impl WorkTimer {
    pub fn tick(&mut self, now: Instant) {
        if self.session.state != State::Running {
            return;
        }
        if let Some(anchor) = self.anchor.replace(now) {
            self.notice |= self.session.advance(now.saturating_duration_since(anchor));
        }
        if self.session.state == State::Complete {
            self.anchor = None;
        }
    }
    pub fn start(&mut self, mode: Mode, config: &TimerConfig, now: Instant) {
        if matches!(self.session.state, State::Running | State::Paused) {
            return;
        }
        self.session.start(mode, config);
        self.anchor = Some(now);
        self.notice = false;
    }
    pub fn pause(&mut self, now: Instant) {
        self.tick(now);
        if self.session.state == State::Running {
            self.session.state = State::Paused;
        }
        self.anchor = None;
    }
    pub fn resume(&mut self, now: Instant) {
        if self.session.state == State::Paused {
            self.session.state = State::Running;
            self.anchor = Some(now);
        }
    }
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn finish_stopwatch(&mut self, now: Instant) {
        if self.session.mode == Some(Mode::Stopwatch)
            && matches!(self.session.state, State::Running | State::Paused)
        {
            self.tick(now);
            self.session.state = State::Complete;
            self.anchor = None;
            self.notice = false;
        }
    }
    pub fn start_break(&mut self, now: Instant) {
        if self.session.state != State::Complete || self.session.mode != Some(Mode::Focus) {
            return;
        }
        let minutes = self.session.break_length.as_secs() / 60;
        self.start(
            Mode::Break,
            &TimerConfig {
                break_minutes: minutes,
                ..Default::default()
            },
            now,
        );
    }
    pub fn next_wakeup(&self) -> Option<Duration> {
        if self.session.state != State::Running {
            return None;
        }
        let next = Duration::from_secs(1)
            - Duration::from_nanos(self.session.elapsed.subsec_nanos().into());
        Some(
            self.session
                .remaining()
                .map_or(next, |r| r.min(next))
                .max(Duration::from_millis(1)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pause_resume_uses_monotonic_elapsed_time_not_refresh_count() {
        let now = Instant::now();
        let mut timer = WorkTimer::default();
        timer.start(Mode::Focus, &TimerConfig::default(), now);
        timer.pause(now + Duration::from_millis(1250));
        timer.tick(now + Duration::from_secs(3600));
        assert_eq!(timer.session.elapsed, Duration::from_millis(1250));
        assert!(timer.next_wakeup().is_none());
        timer.resume(now + Duration::from_secs(3600));
        timer.tick(now + Duration::from_millis(3600750));
        assert_eq!(timer.session.elapsed, Duration::from_secs(2));
        assert_eq!(timer.next_wakeup(), Some(Duration::from_secs(1)));
    }
    #[test]
    fn rest_is_explicit_and_dismissal_is_not_rearmed_on_every_frame() {
        let now = Instant::now();
        let mut timer = WorkTimer::default();
        timer.start(Mode::Focus, &TimerConfig::default(), now);
        timer.tick(now + Duration::from_secs(1500));
        assert!(timer.notice);
        assert_eq!(timer.session.state, State::Complete);
        assert!(timer.next_wakeup().is_none());
        timer.notice = false;
        timer.tick(now + Duration::from_secs(4000));
        assert!(!timer.notice);
        timer.start_break(now + Duration::from_secs(4000));
        assert_eq!(timer.session.mode, Some(Mode::Break));
        assert_eq!(timer.session.remaining(), Some(Duration::from_secs(300)));
        timer.reset();
        timer.tick(now + Duration::from_secs(5000));
        assert_eq!(timer.session.state, State::Idle);
        assert!(timer.next_wakeup().is_none());
    }
    #[test]
    fn starting_again_cannot_accidentally_replace_a_running_or_paused_session() {
        let now = Instant::now();
        let mut timer = WorkTimer::default();
        timer.start(Mode::Stopwatch, &TimerConfig::default(), now);
        timer.tick(now + Duration::from_secs(5));
        timer.start(
            Mode::Break,
            &TimerConfig::default(),
            now + Duration::from_secs(5),
        );
        assert_eq!(timer.session.mode, Some(Mode::Stopwatch));
        timer.pause(now + Duration::from_secs(6));
        timer.start(
            Mode::Focus,
            &TimerConfig::default(),
            now + Duration::from_secs(10),
        );
        assert_eq!(timer.session.mode, Some(Mode::Stopwatch));
        assert_eq!(timer.session.elapsed, Duration::from_secs(6));
    }
}
