mod delta;
mod delta_package;
mod download;
mod files;
mod http;
mod install;
#[cfg(test)]
mod tests;

use crate::{
    domain::update::{Preferences, Release},
    interfaces::update::UpdateBackend,
};
use std::{
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU64},
    },
};

pub struct RepositoryUpdater {
    job: Mutex<Option<PathBuf>>,
    acknowledgement: Mutex<Option<PathBuf>>,
}
impl RepositoryUpdater {
    pub fn new(acknowledgement: Option<PathBuf>) -> Self {
        Self {
            job: Mutex::new(None),
            acknowledgement: Mutex::new(acknowledgement),
        }
    }
}
impl UpdateBackend for RepositoryUpdater {
    fn check(&self, preferences: &Preferences) -> Result<Option<Release>, String> {
        let sources = if preferences.prefer_gitee {
            [http::GITEE, http::GITHUB]
        } else {
            [http::GITHUB, http::GITEE]
        };
        let mut errors = Vec::new();
        for source in sources {
            match http::check(source) {
                Ok(Some(release)) => return Ok(Some(release)),
                Ok(None) => {}
                Err(error) => errors.push(error),
            }
        }
        if errors.len() == 2 {
            Err(errors.join("；"))
        } else {
            Ok(None)
        }
    }
    fn download(
        &self,
        release: &Release,
        cancel: &AtomicBool,
        progress: &AtomicU64,
    ) -> Result<(), String> {
        let job = files::prepare(release, cancel, progress)?;
        *self.job.lock().map_err(|_| "更新状态无法读取")? = Some(job);
        Ok(())
    }
    fn launch_install(&self) -> Result<(), String> {
        let job = self
            .job
            .lock()
            .map_err(|_| "更新状态无法读取")?
            .clone()
            .ok_or("请先下载更新")?;
        install::launch(&job)
    }
    fn schedule_install(&self) -> Result<(), String> {
        let job = self
            .job
            .lock()
            .map_err(|_| "更新状态无法读取")?
            .clone()
            .ok_or("请先下载更新")?;
        install::defer(&job)
    }
    fn confirm_running(&self) -> Result<(), String> {
        if let Some(path) = self
            .acknowledgement
            .lock()
            .map_err(|_| "更新确认状态无法读取")?
            .take()
        {
            install::acknowledge(&path)?;
        }
        Ok(())
    }
}

pub fn helper(args: &[String]) -> Result<(), String> {
    install::helper(args)
}
pub fn pending() -> Result<bool, String> {
    install::pending()
}

pub fn probe(source: &str, download: bool) -> Result<serde_json::Value, String> {
    if !matches!(source, http::GITHUB | http::GITEE) {
        return Err("更新探针来源无效".into());
    }
    let release = http::check(source)?;
    let job = if download {
        release
            .as_ref()
            .map(|release| files::prepare(release, &AtomicBool::new(false), &AtomicU64::new(0)))
            .transpose()?
    } else {
        None
    };
    Ok(serde_json::json!({"release":release,"downloadedJob":job}))
}

pub fn make_delta(
    base: &std::path::Path,
    new: &std::path::Path,
    patch: &std::path::Path,
) -> Result<(), String> {
    delta::create(base, new, patch)
}

pub fn delta_probe(
    source: &str,
    base: &std::path::Path,
    force_fallback: bool,
) -> Result<serde_json::Value, String> {
    if !matches!(source, http::GITHUB | http::GITEE) {
        return Err("更新探针来源无效".into());
    }
    let mut release = http::check(source)?.ok_or("无公开更新版本")?;
    if force_fallback && let Some(delta) = release.manifest.delta.as_mut() {
        delta.sha256 = "0".repeat(64);
    }
    let job = files::prepare_for(&release, base, &AtomicBool::new(false), &AtomicU64::new(0))?;
    Ok(serde_json::json!({"release":release,"downloadedJob":job}))
}

pub fn apply_delta(
    base: &std::path::Path,
    patch: &std::path::Path,
    output: &std::path::Path,
) -> Result<(), String> {
    use std::io::Read;
    let mut file = std::fs::File::open(patch).map_err(|_| "差分文件不可读")?;
    let mut header = [0u8; 16];
    file.read_exact(&mut header).map_err(|_| "差分头不可读")?;
    delta::apply(
        base,
        patch,
        output,
        u64::from_le_bytes(header[8..].try_into().unwrap()),
        &AtomicBool::new(false),
    )
}
