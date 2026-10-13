use super::files::{self, Job};
use std::{
    fs,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, WAIT_OBJECT_0},
    System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
        QueryFullProcessImageNameW, WaitForSingleObject,
    },
};

fn read_job(path: &Path) -> Result<(PathBuf, Job), String> {
    let path = path.canonicalize().map_err(|_| "更新任务不可读取")?;
    if path.file_name().is_none_or(|name| name != "job.json")
        || fs::metadata(&path).map_err(|_| "更新任务不可读取")?.len() > 8192
    {
        return Err("更新任务路径或大小无效".into());
    }
    let mut job: Job = serde_json::from_slice(&fs::read(&path).map_err(|_| "更新任务不可读取")?)
        .map_err(|_| "更新任务格式无效")?;
    job.target = job
        .target
        .canonicalize()
        .map_err(|_| "更新目标目录不可访问")?;
    if job.nonce.is_empty()
        || job.nonce.len() > 40
        || !job.nonce.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err("更新任务身份无效".into());
    }
    let stage = files::cache_root()?
        .join(format!("stage-{}", job.nonce))
        .canonicalize()
        .map_err(|_| "更新暂存目录不可访问")?;
    if path.parent() != Some(stage.as_path()) || job.target.parent().is_none() {
        return Err("更新任务不属于本机暂存目录".into());
    }
    Ok((stage, job))
}
fn executable(target: &Path) -> PathBuf {
    target.join("desktop-meter.exe")
}
fn hidden(exe: &Path) -> Command {
    let mut command = Command::new(exe);
    command.creation_flags(0x08000000);
    command
}
pub(super) fn launch(path: &Path) -> Result<(), String> {
    let (stage, mut job) = read_job(path)?;
    let current = std::env::current_exe()
        .map_err(|_| "当前程序不可访问")?
        .canonicalize()
        .map_err(|_| "当前程序不可访问")?;
    if current != executable(&job.target) || files::hash(&current)? != job.old_hash {
        return Err("更新任务与当前程序不匹配，请重新下载".into());
    }
    files::managed_target(&job.target)?;
    files::managed_target(&stage.join("payload"))?;
    if !files::hash(&executable(&stage.join("payload")))?.eq_ignore_ascii_case(&job.new_hash) {
        return Err("暂存程序校验失败，请重新下载".into());
    }
    job.parent = std::process::id();
    fs::write(
        path,
        serde_json::to_vec(&job).map_err(|_| "更新任务无法生成")?,
    )
    .map_err(|_| "更新任务无法保存")?;
    let helper = stage.join("helper.exe");
    fs::copy(&current, &helper).map_err(|_| "无法创建临时更新辅助程序")?;
    hidden(&helper)
        .arg("--apply-update")
        .arg(path)
        .arg(job.parent.to_string())
        .spawn()
        .map_err(|_| "更新辅助程序无法启动")?;
    Ok(())
}
pub(super) fn defer(path: &Path) -> Result<(), String> {
    let (_, job) = read_job(path)?;
    if std::env::current_exe()
        .map_err(|_| "当前程序不可访问")?
        .canonicalize()
        .map_err(|_| "当前程序不可访问")?
        != executable(&job.target)
    {
        return Err("更新任务与当前程序不匹配".into());
    }
    fs::write(
        files::cache_root()?.join("pending.json"),
        serde_json::to_vec(&path).map_err(|_| "更新安排无法生成")?,
    )
    .map_err(|_| "更新安排无法保存".to_string())
}
pub(super) fn pending() -> Result<bool, String> {
    let pointer = files::cache_root()?.join("pending.json");
    if !pointer.exists() {
        return Ok(false);
    }
    if fs::metadata(&pointer)
        .map_err(|_| "更新安排不可读取")?
        .len()
        > 8192
    {
        return Err("更新安排格式无效".into());
    }
    let path: PathBuf =
        serde_json::from_slice(&fs::read(&pointer).map_err(|_| "更新安排不可读取")?)
            .map_err(|_| "更新安排格式无效")?;
    let (_, job) = read_job(&path)?;
    let current = std::env::current_exe()
        .map_err(|_| "当前程序不可访问")?
        .canonicalize()
        .map_err(|_| "当前程序不可访问")?;
    if current != executable(&job.target) {
        return Ok(false);
    }
    launch(&path)?;
    fs::remove_file(&pointer).map_err(|_| "无法完成更新安排")?;
    Ok(true)
}
pub(super) fn acknowledge(path: &Path) -> Result<(), String> {
    let (stage, job) = read_job(path)?;
    let current = std::env::current_exe()
        .map_err(|_| "新版程序不可访问")?
        .canonicalize()
        .map_err(|_| "新版程序不可访问")?;
    if current != executable(&job.target)
        || job.version != crate::domain::update::VERSION
        || !files::hash(&current)?.eq_ignore_ascii_case(&job.new_hash)
    {
        return Err("新版程序启动确认不匹配".into());
    }
    fs::write(
        stage.join("healthy.txt"),
        format!("{}:{}", job.nonce, job.version),
    )
    .map_err(|_| "新版程序启动确认无法保存".to_string())
}
fn wait_for_parent(job: &Job) -> Result<(), String> {
    let handle = unsafe {
        OpenProcess(
            PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            job.parent,
        )
    };
    if handle.is_null() {
        return Ok(());
    }
    let mut buffer = vec![0u16; 32768];
    let mut length = buffer.len() as u32;
    let valid = unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut length) }
        != 0
        && PathBuf::from(String::from_utf16_lossy(&buffer[..length as usize]))
            .canonicalize()
            .ok()
            .as_ref()
            == Some(&executable(&job.target));
    let result = if valid {
        unsafe { WaitForSingleObject(handle, 60000) }
    } else {
        u32::MAX
    };
    unsafe {
        CloseHandle(handle);
    }
    if result == WAIT_OBJECT_0 {
        Ok(())
    } else {
        Err("原程序尚未安全退出，更新未替换任何文件".into())
    }
}
pub(super) fn helper(args: &[String]) -> Result<(), String> {
    let path = args.first().map(PathBuf::from).ok_or("缺少更新任务")?;
    let (stage, job) = read_job(&path)?;
    if args.get(1).and_then(|pid| pid.parse::<u32>().ok()) != Some(job.parent) || job.parent == 0 {
        return Err("更新父进程无效".into());
    }
    if std::env::current_exe()
        .map_err(|_| "更新辅助程序不可访问")?
        .canonicalize()
        .map_err(|_| "更新辅助程序不可访问")?
        != stage.join("helper.exe")
    {
        return Err("更新辅助程序位置无效".into());
    }
    wait_for_parent(&job)?;
    let backup = replace(&stage, &job)?;
    let mut child = match hidden(&executable(&job.target))
        .arg("--update-ack")
        .arg(&path)
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            rollback(&stage, &job, &backup)?;
            return Err("新版无法启动，已恢复原版".into());
        }
    };
    let started = Instant::now();
    loop {
        if fs::read_to_string(stage.join("healthy.txt"))
            .ok()
            .as_deref()
            == Some(&format!("{}:{}", job.nonce, job.version))
        {
            fs::write(
                stage.join("result.txt"),
                "更新完成；旧版程序保留在相邻 previous 目录，可手动恢复。",
            )
            .map_err(|_| "更新结果无法保存")?;
            return Ok(());
        }
        if child
            .try_wait()
            .map_err(|_| "无法确认新版进程状态")?
            .is_some()
            || started.elapsed() > Duration::from_secs(30)
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let _ = child.kill();
    let _ = child.wait();
    rollback(&stage, &job, &backup)?;
    Err("新版未完成启动确认，已恢复原版；用户配置未替换".into())
}
pub(super) fn replace(stage: &Path, job: &Job) -> Result<PathBuf, String> {
    files::managed_target(&job.target)?;
    let payload = stage.join("payload");
    files::managed_target(&payload)?;
    if files::hash(&executable(&job.target))? != job.old_hash
        || !files::hash(&executable(&payload))?.eq_ignore_ascii_case(&job.new_hash)
    {
        return Err("更新前的程序校验失败，未替换文件".into());
    }
    let name = job
        .target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("应用目录名称无效")?;
    let backup = job
        .target
        .with_file_name(format!("{name}.previous-{}", job.nonce));
    if backup.exists() {
        return Err("旧版备份目录已存在，未替换文件".into());
    }
    fs::rename(&job.target, &backup).map_err(|_| "原程序目录被占用或没有写入权限，未替换文件")?;
    if fs::rename(&payload, &job.target).is_err() {
        fs::rename(&backup, &job.target)
            .map_err(|_| "更新被中断，请使用 previous 备份恢复应用目录")?;
        return Err("新版替换失败，已恢复原程序目录".into());
    }
    Ok(backup)
}
fn rollback(stage: &Path, job: &Job, backup: &Path) -> Result<(), String> {
    fs::rename(&job.target, stage.join("failed-program"))
        .map_err(|_| "新版目录无法移开，请从 previous 备份恢复")?;
    fs::rename(backup, &job.target).map_err(|_| "原版备份无法恢复，请检查 previous 目录")?;
    fs::write(stage.join("result.txt"), "新版启动失败，已回退原版。")
        .map_err(|_| "更新结果无法保存")?;
    hidden(&executable(&job.target))
        .spawn()
        .map_err(|_| "旧版已恢复，请手动启动")?;
    Ok(())
}
