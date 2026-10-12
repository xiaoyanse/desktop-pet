#[cfg(test)]
mod tests;
use crate::{domain::office::Book, interfaces::OfficeStore};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::windows::ffi::OsStrExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use windows_sys::Win32::Storage::FileSystem::{
    MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
};
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
static SERIAL: AtomicU64 = AtomicU64::new(0);
pub struct LocalOfficeStore {
    path: PathBuf,
}
impl Default for LocalOfficeStore {
    fn default() -> Self {
        let executable =
            std::env::current_exe().unwrap_or_else(|_| PathBuf::from("E:\\desktop-meter.exe"));
        let root = executable.ancestors().last().unwrap_or(&executable);
        Self {
            path: root
                .join("DesktopMeterData")
                .join("Office")
                .join("workspace.json"),
        }
    }
}
impl LocalOfficeStore {
    #[cfg(test)]
    fn at(path: PathBuf) -> Self {
        Self { path }
    }
}
fn replace(from: &std::path::Path, to: &std::path::Path) -> Result<(), String> {
    let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}
impl OfficeStore for LocalOfficeStore {
    fn location(&self) -> String {
        self.path.display().to_string()
    }
    fn load(&self) -> Result<Book, String> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Book::default());
            }
            Err(error) => return Err(error.to_string()),
        };
        if file.metadata().map_err(|e| e.to_string())?.len() > MAX_FILE_BYTES {
            return Err("便签文件超过16 MiB上限".into());
        }
        let mut bytes = vec![];
        file.take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err("便签文件超过16 MiB上限".into());
        }
        let book: Book = serde_json::from_slice(&bytes)
            .map_err(|e| format!("便签文件损坏，未覆盖；可检查 workspace.backup.json：{e}"))?;
        book.validate()?;
        Ok(book)
    }
    fn save(&self, book: &Book, previous: &Book) -> Result<(), String> {
        book.validate()?;
        let bytes = serde_json::to_vec_pretty(book).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err("便签文件超过16 MiB上限".into());
        }
        let parent = self.path.parent().ok_or("便签目录无效")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let suffix = format!(
            "{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        );
        let temp = parent.join(format!("workspace-{suffix}.partial"));
        let backup_temp = parent.join(format!("backup-{suffix}.partial"));
        let mut own_temp = false;
        let mut own_backup = false;
        let result = (|| {
            if self.load()? != *previous {
                return Err(
                    "便签文件已被其他操作修改；保留当前编辑，请重新载入后核对原数据".into(),
                );
            }
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|e| e.to_string())?;
            own_temp = true;
            file.write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|e| e.to_string())?;
            drop(file);
            if self.path.try_exists().map_err(|e| e.to_string())? {
                self.load()?;
                let source = File::open(&self.path).map_err(|e| e.to_string())?;
                let mut backup = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&backup_temp)
                    .map_err(|e| e.to_string())?;
                own_backup = true;
                let copied = std::io::copy(&mut source.take(MAX_FILE_BYTES + 1), &mut backup)
                    .map_err(|e| e.to_string())?;
                if copied > MAX_FILE_BYTES {
                    return Err("便签文件在保存时增长，原文件未覆盖".into());
                }
                backup.sync_all().map_err(|e| e.to_string())?;
                drop(backup);
                replace(&backup_temp, &parent.join("workspace.backup.json"))?;
            }
            replace(&temp, &self.path)
        })();
        if result.is_err() {
            if own_temp {
                let _ = fs::remove_file(&temp);
            }
            if own_backup {
                let _ = fs::remove_file(&backup_temp);
            }
        }
        result
    }
}
