use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reminder {
    Off,
    #[default]
    Badge,
    Card,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TimerConfig {
    pub focus_minutes: u64,
    pub break_minutes: u64,
    pub stopwatch: bool,
    pub custom: bool,
    pub reminder: Reminder,
}
impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            focus_minutes: 25,
            break_minutes: 5,
            stopwatch: false,
            custom: false,
            reminder: Reminder::Badge,
        }
    }
}
impl TimerConfig {
    pub fn normalize(&mut self) {
        self.focus_minutes = self.focus_minutes.clamp(1, 1440);
        self.break_minutes = self.break_minutes.clamp(1, 1440);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Focus,
    Break,
    Stopwatch,
}
impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Focus => "专注",
            Self::Break => "休息",
            Self::Stopwatch => "正计时",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Idle,
    Running,
    Paused,
    Complete,
}

#[derive(Default)]
pub struct Session {
    pub state: State,
    pub mode: Option<Mode>,
    pub elapsed: Duration,
    pub target: Option<Duration>,
    pub break_length: Duration,
}
impl Session {
    pub fn start(&mut self, mode: Mode, config: &TimerConfig) {
        self.state = State::Running;
        self.mode = Some(mode);
        self.elapsed = Duration::ZERO;
        self.break_length = Duration::from_secs(config.break_minutes.clamp(1, 1440) * 60);
        self.target = match mode {
            Mode::Focus => Some(Duration::from_secs(
                config.focus_minutes.clamp(1, 1440) * 60,
            )),
            Mode::Break => Some(self.break_length),
            Mode::Stopwatch => None,
        };
    }
    pub fn advance(&mut self, delta: Duration) -> bool {
        if self.state != State::Running {
            return false;
        }
        self.elapsed = self.elapsed.saturating_add(delta);
        if let Some(target) = self.target
            && self.elapsed >= target
        {
            self.elapsed = target;
            self.state = State::Complete;
            return true;
        }
        false
    }
    pub fn remaining(&self) -> Option<Duration> {
        self.target.map(|t| t.saturating_sub(self.elapsed))
    }
    pub fn displayed_seconds(&self) -> u64 {
        if let Some(remaining) = self.remaining() {
            remaining.as_secs() + u64::from(remaining.subsec_nanos() != 0)
        } else {
            self.elapsed.as_secs()
        }
    }
}

pub fn clock(seconds: u64) -> String {
    if seconds >= 3600 {
        format!(
            "{:02}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn countdown_freezes_lengths_and_finishes_once_after_a_late_update() {
        let mut config = TimerConfig::default();
        let mut session = Session::default();
        session.start(Mode::Focus, &config);
        config.focus_minutes = 50;
        config.break_minutes = 10;
        assert_eq!(config.focus_minutes, 50);
        assert_eq!(config.break_minutes, 10);
        session.advance(Duration::from_millis(1));
        assert_eq!(session.displayed_seconds(), 1500);
        assert_eq!(session.break_length, Duration::from_secs(300));
        assert!(session.advance(Duration::from_secs(3600)));
        assert_eq!(session.displayed_seconds(), 0);
        assert_eq!(session.elapsed, Duration::from_secs(1500));
        assert!(!session.advance(Duration::from_secs(3600)));
    }
    #[test]
    fn stopwatch_and_paused_sessions_have_no_hidden_countdown() {
        let mut session = Session::default();
        session.start(Mode::Stopwatch, &TimerConfig::default());
        session.advance(Duration::from_secs(3700));
        assert_eq!(clock(session.displayed_seconds()), "01:01:40");
        assert_eq!(session.state, State::Running);
        session.state = State::Paused;
        session.advance(Duration::from_secs(3600));
        assert_eq!(session.elapsed, Duration::from_secs(3700));
    }
    #[test]
    fn settings_roundtrip_and_old_empty_settings_keep_safe_defaults() {
        let mut config: TimerConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.reminder, Reminder::Badge);
        config.focus_minutes = u64::MAX;
        config.break_minutes = 0;
        config.reminder = Reminder::Off;
        config.normalize();
        let loaded: TimerConfig =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!((loaded.focus_minutes, loaded.break_minutes), (1440, 1));
        assert_eq!(loaded.reminder, Reminder::Off);
    }
}
