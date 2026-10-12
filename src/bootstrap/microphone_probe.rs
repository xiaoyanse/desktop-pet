use crate::{infrastructure::windows::microphone::WindowsMicrophone, interfaces::MicrophoneSource};
use std::{io::Write, path::Path};

/// Explicit read-only endpoint probe; never creates an audio stream or changes device state.
pub fn run(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("麦克风探针需要新文件的绝对路径".into());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let result = match WindowsMicrophone.sample() {
        Ok(status) => {
            serde_json::json!({"name":status.name,"muted":status.muted,"inputVolume":status.volume})
        }
        Err(error) => serde_json::json!({"error":error}),
    };
    let report = serde_json::json!({"pid":std::process::id(),"endpointRole":"communications","readOnly":true,"audioSamplesRead":false,"result":result});
    file.write_all(&serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
