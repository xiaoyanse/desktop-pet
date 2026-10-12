use super::work_timer::{Reminder, State};
use serde::{Deserialize, Serialize};
use std::time::Duration;
#[cfg(test)]
mod tests;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MeetingConfig {
    pub minutes: u64,
    pub remind_mute: bool,
    pub mute_minutes: u64,
    pub reminder: Reminder,
}
impl Default for MeetingConfig {
    fn default() -> Self {
        Self {
            minutes: 30,
            remind_mute: false,
            mute_minutes: 15,
            reminder: Reminder::Badge,
        }
    }
}
impl MeetingConfig {
    pub fn normalize(&mut self) {
        self.minutes = self.minutes.clamp(1, 1440);
        self.mute_minutes = self.mute_minutes.clamp(1, 120);
    }
}
#[derive(Clone, Debug)]
pub struct MicrophoneStatus {
    pub name: String,
    pub muted: bool,
    pub volume: Option<u8>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notice {
    Time,
    Microphone,
}
impl Notice {
    pub fn label(self) -> &'static str {
        match self {
            Self::Time => "会议已到预设时间",
            Self::Microphone => "系统麦克风仍未静音",
        }
    }
    pub fn detail(self) -> &'static str {
        match self {
            Self::Time => "计时仍在继续；可以结束会议或继续记录。",
            Self::Microphone => "若暂时不发言，请检查会议软件的静音状态。",
        }
    }
}
#[derive(Default)]
pub struct Session {
    pub state: State,
    pub elapsed: Duration,
    pub target: Duration,
    interval: Option<Duration>,
    next_mute: Duration,
    time_notified: bool,
}
impl Session {
    pub fn start(&mut self, config: &MeetingConfig) {
        *self = Self {
            state: State::Running,
            elapsed: Duration::ZERO,
            target: Duration::from_secs(config.minutes.clamp(1, 1440) * 60),
            interval: config
                .remind_mute
                .then(|| Duration::from_secs(config.mute_minutes.clamp(1, 120) * 60)),
            next_mute: Duration::from_secs(config.mute_minutes.clamp(1, 120) * 60),
            time_notified: false,
        };
    }
    pub fn advance(&mut self, delta: Duration) -> (bool, bool) {
        if self.state != State::Running {
            return (false, false);
        }
        self.elapsed = self.elapsed.saturating_add(delta);
        let time = !self.time_notified && self.elapsed >= self.target;
        self.time_notified |= time;
        let microphone = self.interval.is_some() && self.elapsed >= self.next_mute;
        if microphone {
            self.next_mute = self.elapsed.saturating_add(self.interval.unwrap());
        }
        (time, microphone)
    }
    pub fn overtime(&self) -> Duration {
        self.elapsed.saturating_sub(self.target)
    }
    pub fn active(&self) -> bool {
        matches!(self.state, State::Running | State::Paused)
    }
}
