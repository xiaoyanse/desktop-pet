use crate::domain::update::Release;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) const FILES: [&str; 4] = [
    "desktop-meter.exe",
    "README.md",
    "启动设置.cmd",
    "manifest.json",
];
#[derive(Serialize, Deserialize)]
pub(super) struct Job {
    pub target: PathBuf,
    pub old_hash: String,
    pub version: String,
    pub new_hash: String,
    pub nonce: String,
    pub parent: u32,
}
pub(super) fn cache_root() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|_| "无法确定应用位置")?;
    let root: PathBuf = exe.components().take(2).collect();
    if !root.is_absolute() {
        return Err("更新仅支持本地磁盘中的应用".into());
    }
    Ok(root.join("DesktopMeterData").join("updates"))
}
pub(super) fn hash(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|_| "无法读取程序或更新包")?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer).map_err(|_| "无法校验程序或更新包")?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
pub(super) fn managed_target(target: &Path) -> Result<(), String> {
    if !target.is_absolute()
        || target.components().count() < 3
        || fs::symlink_metadata(target)
            .map_err(|_| "应用目录不可访问")?
            .file_type()
            .is_symlink()
    {
        return Err("应用目录不适合安全更新".into());
    }
    let manifest_path = target.join("manifest.json");
    if fs::metadata(&manifest_path)
        .map_err(|_| "请从正式发布包运行后再安装更新")?
        .len()
        > 65536
    {
        return Err("本地包清单过大".into());
    }
    let bytes = fs::read(manifest_path).map_err(|_| "请从正式发布包运行后再安装更新")?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "本地包清单无效")?;
    if manifest["version"].as_str().is_none() {
        return Err("本地包清单缺少版本".into());
    }
    for entry in fs::read_dir(target).map_err(|_| "应用目录不可访问")? {
        let entry = entry.map_err(|_| "无法检查应用目录")?;
        if !entry.file_type().map_err(|_| "无法检查应用文件")?.is_file()
            || !FILES.iter().any(|name| entry.file_name() == *name)
        {
            return Err("应用目录有额外文件；请将用户文件移出后再更新".into());
        }
    }
    if FILES.iter().any(|name| !target.join(name).is_file()) {
        return Err("本地发布包不完整".into());
    }
    Ok(())
}
pub(super) fn extract(
    zip_path: &Path,
    destination: &Path,
    version: &str,
    expected_hash: &str,
) -> Result<(), String> {
    let file = File::open(zip_path).map_err(|_| "更新包不可读取")?;
    let mut archive = zip::ZipArchive::new(file).map_err(|_| "更新包不是有效 ZIP")?;
    if archive.len() > 16 {
        return Err("更新包文件数量超限".into());
    }
    fs::create_dir(destination).map_err(|_| "无法创建更新暂存目录")?;
    let prefix = format!("DesktopMeter-{version}-win-x64/");
    let mut seen = std::collections::HashSet::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|_| "更新包条目无效")?;
        let name = file.name().replace('\\', "/");
        if file.is_dir() && name == prefix {
            continue;
        }
        let leaf = name
            .strip_prefix(&prefix)
            .ok_or("更新包路径无效")?
            .to_owned();
        if !FILES.contains(&leaf.as_str())
            || !seen.insert(leaf.clone())
            || file
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("更新包包含非法、重复或链接文件".into());
        }
        total = total.checked_add(file.size()).ok_or("更新包大小无效")?;
        if total > 100 * 1024 * 1024 {
            return Err("更新解压大小超限".into());
        }
        let mut output = File::options()
            .write(true)
            .create_new(true)
            .open(destination.join(leaf))
            .map_err(|_| "更新文件无法写入")?;
        let size = file.size();
        let copied = std::io::copy(&mut file.by_ref().take(size + 1), &mut output)
            .map_err(|_| "更新解压失败")?;
        if copied != size {
            return Err("更新条目大小不一致".into());
        }
        output.sync_all().map_err(|_| "更新文件同步失败")?;
    }
    if seen.len() != FILES.len() {
        return Err("更新包不完整".into());
    }
    validate_payload(destination, version, expected_hash)
}
pub(super) fn validate_payload(
    destination: &Path,
    version: &str,
    expected_hash: &str,
) -> Result<(), String> {
    if !hash(&destination.join("desktop-meter.exe"))?.eq_ignore_ascii_case(expected_hash) {
        return Err("更新程序校验失败".into());
    }
    managed_target(destination)?;
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(destination.join("manifest.json")).map_err(|_| "更新包清单不可读取")?,
    )
    .map_err(|_| "更新包清单无效")?;
    if manifest["version"].as_str() != Some(version) {
        return Err("更新包内部版本不匹配".into());
    }
    Ok(())
}
pub(super) fn prepare(
    release: &Release,
    cancel: &AtomicBool,
    progress: &AtomicU64,
) -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|_| "无法确定当前程序")?;
    prepare_for(release, &exe, cancel, progress)
}
pub(super) fn prepare_for(
    release: &Release,
    exe: &Path,
    cancel: &AtomicBool,
    progress: &AtomicU64,
) -> Result<PathBuf, String> {
    release.manifest.validate(&release.version)?;
    let exe = exe.canonicalize().map_err(|_| "当前程序不可访问")?;
    let target = exe.parent().ok_or("当前程序路径无效")?.to_owned();
    managed_target(&target)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "系统时间异常")?
        .as_nanos()
        .to_string();
    let root = cache_root()?;
    fs::create_dir_all(&root).map_err(|_| "更新缓存目录不可写")?;
    let stage = root.join(format!("stage-{nonce}"));
    fs::create_dir(&stage).map_err(|_| "更新暂存目录无法创建")?;
    let result = (|| {
        let mut delta_error = None;
        let mut incremental = false;
        if let Some(delta) = &release.manifest.delta
            && delta.size < release.manifest.size
            && super::delta_package::matches(delta, &release.version, &exe)
        {
            let mut incremental_release = release.clone();
            incremental_release.url = delta.url.clone();
            incremental_release.manifest.size = delta.size;
            let incremental_result = (|| {
                let package = stage.join("delta.zip");
                super::download::run(&package, &incremental_release, cancel, progress)?;
                if !hash(&package)?.eq_ignore_ascii_case(&delta.sha256) {
                    return Err("增量包 SHA256 校验失败".to_string());
                }
                super::delta_package::extract(
                    &package,
                    &stage.join("payload"),
                    &exe,
                    release,
                    cancel,
                )
            })();
            match incremental_result {
                Ok(()) => incremental = true,
                Err(error) => {
                    if cancel.load(Ordering::Relaxed) {
                        return Err("下载已取消".into());
                    }
                    delta_error = Some(error);
                    let payload = stage.join("payload");
                    if payload.exists() {
                        fs::remove_dir_all(&payload).map_err(|_| "增量暂存清除失败")?;
                    }
                    progress.store(0, Ordering::Relaxed);
                }
            }
        }
        if !incremental {
            let package = stage.join("package.zip");
            super::download::run(&package, release, cancel, progress)?;
            if !hash(&package)?.eq_ignore_ascii_case(&release.manifest.sha256) {
                return Err("更新包 SHA256 校验失败".into());
            }
            if cancel.load(Ordering::Relaxed) {
                return Err("下载已取消".into());
            }
            extract(
                &package,
                &stage.join("payload"),
                &release.version,
                &release.manifest.executable_sha256,
            )?;
        }
        if cancel.load(Ordering::Relaxed) {
            return Err("下载已取消".into());
        }
        let job = Job {
            target,
            old_hash: hash(&exe)?,
            version: release.version.clone(),
            new_hash: release.manifest.executable_sha256.clone(),
            nonce,
            parent: std::process::id(),
        };
        let path = stage.join("job.json");
        fs::write(stage.join("transfer.json"), serde_json::to_vec(&serde_json::json!({"kind":if incremental { "delta" } else { "full" }, "deltaFallbackReason":delta_error})).map_err(|_| "传输记录无法生成")?).map_err(|_| "传输记录无法写入")?;
        fs::write(
            &path,
            serde_json::to_vec(&job).map_err(|_| "更新任务无法生成")?,
        )
        .map_err(|_| "更新任务无法保存")?;
        Ok(path)
    })();
    if result.is_err()
        && stage.canonicalize().ok().is_some_and(|path| {
            root.canonicalize()
                .ok()
                .is_some_and(|root| path.parent() == Some(root.as_path()))
        })
    {
        let _ = fs::remove_dir_all(&stage);
    }
    result
}
