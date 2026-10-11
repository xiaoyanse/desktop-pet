use crate::{config::data_dir, native::{Handle, wide}};
use serde::{Deserialize, Serialize};
use std::{sync::mpsc, time::{SystemTime, UNIX_EPOCH}};
use windows_sys::Win32::{Foundation::*, Security::*, System::{Threading::*, SystemInformation::*}, UI::{Shell::*, WindowsAndMessaging::SW_HIDE}};

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtSetSystemInformation(class: i32, data: *const std::ffi::c_void, length: u32) -> i32;
    fn NtQuerySystemInformation(class: i32, data: *mut std::ffi::c_void, length: u32, returned: *mut u32) -> i32;
}
#[repr(C)]
#[derive(Default)]
struct MemoryLists { zero: usize, free: usize, modified: usize, no_write: usize, bad: usize,
    standby: [usize; 8], repurposed: [usize; 8], modified_pagefile: usize }
#[derive(Serialize, Deserialize)]
struct Outcome { before: u64, after: u64, standby_before: Option<u64>, standby_after: Option<u64>, error: Option<String> }
fn available() -> Result<u64, String> {
    let mut m = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
    if unsafe { GlobalMemoryStatusEx(&mut m) } == 0 { Err(std::io::Error::last_os_error().to_string()) } else { Ok(m.ullAvailPhys) }
}
fn low_priority() -> Option<u64> {
    unsafe {
        let mut lists = MemoryLists::default();
        if NtQuerySystemInformation(80, (&mut lists as *mut MemoryLists).cast(), std::mem::size_of::<MemoryLists>() as u32, std::ptr::null_mut()) < 0 { return None; }
        let mut sys = SYSTEM_INFO::default(); GetSystemInfo(&mut sys);
        Some(lists.standby[0] as u64 * sys.dwPageSize as u64)
    }
}
fn enable_privilege() -> Result<Handle, String> {
    unsafe {
        let mut raw = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut raw) == 0 { return Err("无法打开进程权限令牌".into()); }
        let token = Handle::new(raw).ok_or("权限令牌无效")?;
        let mut privileges = TOKEN_PRIVILEGES { PrivilegeCount: 1, ..Default::default() };
        if LookupPrivilegeValueW(std::ptr::null(), wide("SeProfileSingleProcessPrivilege").as_ptr(), &mut privileges.Privileges[0].Luid) == 0 { return Err("系统不支持所需权限".into()); }
        privileges.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
        SetLastError(0);
        let ok = AdjustTokenPrivileges(token.0, 0, &privileges, 0, std::ptr::null_mut(), std::ptr::null_mut());
        if ok == 0 || GetLastError() != 0 { return Err("没有清理权限，请使用管理员账号授权".into()); }
        Ok(token)
    }
}
fn execute() -> Result<Outcome, String> {
    let _token = enable_privilege()?;
    let before = available()?; let standby_before = low_priority();
    // Native API class 80 / command 5 affects priority-0 standby pages only.
    let command = 5i32;
    let status = unsafe { NtSetSystemInformation(80, (&command as *const i32).cast(), 4) };
    if status < 0 { return Err(format!("清理失败，NTSTATUS 0x{:08X}；未尝试其他清理方式", status as u32)); }
    Ok(Outcome { before, after: available()?, standby_before, standby_after: low_priority(), error: None })
}
pub fn helper(args: &[String]) {
    // Elevated helper accepts a fixed-format token, never an arbitrary output path.
    let Some(token) = args.first().filter(|s| valid_token(s)) else { return; };
    let outcome = execute().unwrap_or_else(|error| Outcome { before: 0, after: 0, standby_before: None, standby_after: None, error: Some(error) });
    let folder = data_dir().join("cleanup");
    if std::fs::create_dir_all(&folder).is_ok() {
        let _ = std::fs::write(folder.join(format!("{token}.json")), serde_json::to_vec(&outcome).unwrap());
    }
}
fn valid_token(token: &str) -> bool { !token.is_empty() && token.len() < 64 && token.bytes().all(|c| c.is_ascii_digit() || c == b'-') }
pub fn request(ctx: eframe::egui::Context) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = launch();
        let _ = tx.send(result.unwrap_or_else(|e| e)); ctx.request_repaint();
    }); rx
}
fn launch() -> Result<String, String> {
    let token = format!("{}-{}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos());
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let file = wide(exe.to_string_lossy()); let verb = wide("runas");
    let params = wide(format!("--clean-low-priority {token}"));
    unsafe {
        let mut info = SHELLEXECUTEINFOW { cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC, lpVerb: verb.as_ptr(), lpFile: file.as_ptr(),
            lpParameters: params.as_ptr(), nShow: SW_HIDE, ..Default::default() };
        if ShellExecuteExW(&mut info) == 0 { return Err(format!("未执行清理（授权取消或启动失败）：{}", std::io::Error::last_os_error())); }
        let process = Handle::new(info.hProcess).ok_or("清理辅助进程未返回句柄")?;
        if WaitForSingleObject(process.0, 120_000) != WAIT_OBJECT_0 { return Err("辅助进程尚未完成。为避免重复操作，请稍后查看内存状态".into()); }
    }
    let path = data_dir().join("cleanup").join(format!("{token}.json"));
    let result: Outcome = serde_json::from_slice(&std::fs::read(&path).map_err(|e| format!("无法读取清理结果：{e}"))?).map_err(|e| e.to_string())?;
    if let Some(error) = result.error { return Err(error); }
    let mut text = format!("操作完成。可用内存 {:.2} → {:.2} GB（变化 {:+.2} GB）", result.before as f64 / 1073741824.0,
        result.after as f64 / 1073741824.0, (result.after as f64 - result.before as f64) / 1073741824.0);
    if let (Some(a), Some(b)) = (result.standby_before, result.standby_after) { text.push_str(&format!("；低优先级缓存 {:.1} → {:.1} MB", a as f64 / 1048576.0, b as f64 / 1048576.0)); }
    text.push_str("。待机缓存本来就可复用，可用内存可能不增加；变化也受其他工作影响。");
    Ok(text)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn helper_rejects_paths_and_only_accepts_tokens() {
        assert!(valid_token("123-987654")); assert!(!valid_token("..\\file"));
        assert!(!valid_token("C:\\file")); assert!(!valid_token(""));
    }
}
