use crate::domain::meeting::MicrophoneStatus;
/// Only endpoint metadata and controls are read; no audio stream is acquired.
pub trait MicrophoneSource: Send + Sync {
    fn sample(&self) -> Result<MicrophoneStatus, String>;
}
