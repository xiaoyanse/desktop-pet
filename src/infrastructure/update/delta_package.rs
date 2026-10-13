use super::files;
use crate::domain::update::{Delta, Release};
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub(super) fn matches(delta: &Delta, version: &str, base: &Path) -> bool {
    delta.validate(version)
        && !delta.url.is_empty()
        && files::hash(base)
            .is_ok_and(|hash| hash.eq_ignore_ascii_case(&delta.base_executable_sha256))
        && fs::read(base.parent().unwrap_or(Path::new("")).join("manifest.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .is_some_and(|manifest| manifest["version"].as_str() == Some(&delta.base_version))
}
pub(super) fn extract(
    package: &Path,
    destination: &Path,
    base: &Path,
    release: &Release,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let delta = release.manifest.delta.as_ref().ok_or("缺少增量清单")?;
    if !matches(delta, &release.version, base) {
        return Err("增量基线不匹配".into());
    }
    let mut archive = zip::ZipArchive::new(File::open(package).map_err(|_| "增量包不可读")?)
        .map_err(|_| "增量 ZIP 无效")?;
    if archive.len() > 8 {
        return Err("增量条目超限".into());
    }
    fs::create_dir(destination).map_err(|_| "增量目录不可写")?;
    let prefix = format!(
        "DesktopMeter-{}-from-{}-delta-win-x64/",
        release.version, delta.base_version
    );
    let mut seen = std::collections::HashSet::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        if cancel.load(Ordering::Relaxed) {
            return Err("下载已取消".into());
        }
        let mut entry = archive.by_index(index).map_err(|_| "增量条目无效")?;
        let name = entry.name().replace('\\', "/");
        if entry.is_dir() && name == prefix {
            continue;
        }
        let leaf = name.strip_prefix(&prefix).ok_or("增量路径无效")?;
        if ![
            "desktop-meter.exe.patch",
            "README.md",
            "启动设置.cmd",
            "manifest.json",
        ]
        .contains(&leaf)
            || !seen.insert(leaf.to_owned())
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("增量包含非法、链接或重复条目".into());
        }
        total = total.checked_add(entry.size()).ok_or("增量大小无效")?;
        if total > 100 * 1024 * 1024 {
            return Err("增量解压大小超限".into());
        }
        let mut output = File::options()
            .write(true)
            .create_new(true)
            .open(destination.join(leaf))
            .map_err(|_| "增量文件不可写")?;
        let size = entry.size();
        if std::io::copy(&mut entry.by_ref().take(size + 1), &mut output)
            .map_err(|_| "增量解压失败")?
            != size
        {
            return Err("增量文件大小不符".into());
        }
        output.sync_all().map_err(|_| "增量同步失败")?;
    }
    if seen.len() != 4 {
        return Err("增量包不完整".into());
    }
    super::delta::apply(
        base,
        &destination.join("desktop-meter.exe.patch"),
        &destination.join("desktop-meter.exe"),
        delta.executable_size,
        cancel,
    )?;
    fs::remove_file(destination.join("desktop-meter.exe.patch"))
        .map_err(|_| "差分临时文件不可移除")?;
    files::validate_payload(
        destination,
        &release.version,
        &release.manifest.executable_sha256,
    )
}
