use super::settings::data_dir;
use crate::{domain::cleanup::CleanupReport, interfaces::CleanupHistory};

pub struct JsonCleanupHistory;
impl CleanupHistory for JsonCleanupHistory {
    fn load(&self) -> Result<Vec<CleanupReport>, String> {
        let path = data_dir().join("cleanup-history.json");
        if !path.exists() {
            return Ok(Vec::new());
        }
        if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 1024 * 1024 {
            return Err("记录文件超过 1 MB".into());
        }
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        let mut records: Vec<CleanupReport> =
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if records.len() > 50 {
            records.drain(..records.len() - 50);
        }
        Ok(records)
    }
    fn save(&self, reports: &[CleanupReport]) -> Result<(), String> {
        std::fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
        let temp = data_dir().join("cleanup-history.tmp");
        let path = data_dir().join("cleanup-history.json");
        std::fs::write(
            &temp,
            serde_json::to_vec(reports).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        unsafe {
            use windows_sys::Win32::Storage::FileSystem::*;
            if MoveFileExW(
                crate::infrastructure::windows::native::wide(temp.to_string_lossy()).as_ptr(),
                crate::infrastructure::windows::native::wide(path.to_string_lossy()).as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            ) == 0
            {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
        Ok(())
    }
}
