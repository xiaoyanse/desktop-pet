use crate::{domain::config::Config, interfaces::SettingsStore};
use std::{fs, path::PathBuf};

pub struct JsonSettings;
impl SettingsStore for JsonSettings {
    fn load(&self) -> (Config, Option<String>) {
        let path = data_dir().join("settings.json");
        if !path.exists() {
            return (Config::default(), None);
        }
        match fs::read(&path).map_err(|e| e.to_string()).and_then(|b| {
            let value: serde_json::Value = serde_json::from_slice(&b).map_err(|e| e.to_string())?;
            let legacy = value.get("version").is_none();
            let mut config: Config = serde_json::from_value(value).map_err(|e| e.to_string())?;
            if legacy {
                config.version = 1;
            }
            config.normalize();
            Ok(config)
        }) {
            Ok(config) => (config, None),
            Err(e) => (
                Config::default(),
                Some(format!("设置读取失败，已使用默认值：{e}")),
            ),
        }
    }
    fn save(&self, config: &Config) -> Result<(), String> {
        fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
        let path = data_dir().join("settings.json");
        let temp = data_dir().join("settings.tmp");
        fs::write(
            &temp,
            serde_json::to_vec_pretty(config).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let from = super::super::windows::native::wide(temp.to_string_lossy());
        let to = super::super::windows::native::wide(path.to_string_lossy());
        unsafe {
            use windows_sys::Win32::Storage::FileSystem::{
                MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
            };
            if MoveFileExW(
                from.as_ptr(),
                to.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            ) == 0
            {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
        Ok(())
    }
}
pub fn data_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_else(|| ".".into()))
        .join("DesktopMeter")
}
