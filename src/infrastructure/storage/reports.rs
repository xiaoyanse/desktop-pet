use crate::interfaces::ReportStore;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct LocalReports;
impl ReportStore for LocalReports {
    fn save(&self, text: &str) -> Result<String, String> {
        save_in(super::settings::data_dir().join("reports"), text)
    }
}
fn save_in(folder: PathBuf, text: &str) -> Result<String, String> {
    fs::create_dir_all(&folder)
        .map_err(|_| "报告目录无法创建，请检查磁盘及目录权限".to_string())?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    for sequence in 0..100 {
        let path = folder.join(format!("diagnostic-{stamp}-{sequence}.md"));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(text.as_bytes())
                    .and_then(|_| file.sync_all())
                    .map_err(|_| "报告写入失败，请检查可用空间；目标文件可能不完整".to_string())?;
                return Ok(path.to_string_lossy().into_owned());
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err("报告文件无法创建，请检查磁盘及目录权限".into()),
        }
    }
    Err("报告文件名冲突，请稍后重试".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saving_twice_preserves_each_utf8_preview_and_existing_files() {
        let folder = std::env::temp_dir().join(format!(
            "desktop-meter-report-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let a = save_in(folder.clone(), "# 中文报告\n第一份").unwrap();
        let b = save_in(folder.clone(), "# 中文报告\n第二份").unwrap();
        assert_ne!(a, b);
        assert_eq!(fs::read_to_string(&a).unwrap(), "# 中文报告\n第一份");
        assert_eq!(fs::read_to_string(&b).unwrap(), "# 中文报告\n第二份");
        fs::remove_dir_all(folder).unwrap();
    }
}
