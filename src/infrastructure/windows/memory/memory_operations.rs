use crate::domain::cleanup::{CleanupReport, MemoryStatus, Operation, OperationResult};
use crate::infrastructure::windows::native::{Handle, from_wide, wide};
use std::time::{SystemTime, UNIX_EPOCH};
use windows_sys::Win32::{
    Foundation::*,
    Security::*,
    Storage::FileSystem::*,
    System::{
        Memory::SetSystemFileCacheSize, ProcessStatus::*, SystemInformation::*, Threading::*,
    },
};

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtSetSystemInformation(class: i32, data: *mut std::ffi::c_void, length: u32) -> i32;
    fn NtQuerySystemInformation(
        class: i32,
        data: *mut std::ffi::c_void,
        length: u32,
        returned: *mut u32,
    ) -> i32;
}
#[repr(C)]
#[derive(Default)]
struct MemoryLists {
    zero: usize,
    free: usize,
    modified: usize,
    no_write: usize,
    bad: usize,
    standby: [usize; 8],
    repurposed: [usize; 8],
    modified_pagefile: usize,
}
#[repr(C)]
#[derive(Default)]
struct CombineInfo {
    event: HANDLE,
    pages: usize,
    flags: u32,
}

pub fn inspect() -> Result<MemoryStatus, String> {
    unsafe {
        let mut m = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        if GlobalMemoryStatusEx(&mut m) == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let mut p = PERFORMANCE_INFORMATION::default();
        if GetPerformanceInfo(&mut p, std::mem::size_of_val(&p) as u32) == 0 {
            return Err("无法读取提交内存和系统缓存".into());
        }
        let mut lists = MemoryLists::default();
        let valid = NtQuerySystemInformation(
            80,
            (&mut lists as *mut MemoryLists).cast(),
            std::mem::size_of_val(&lists) as u32,
            std::ptr::null_mut(),
        ) >= 0;
        Ok(MemoryStatus {
            total: m.ullTotalPhys,
            available: m.ullAvailPhys,
            load: m.dwMemoryLoad,
            commit: (p.CommitTotal * p.PageSize) as u64,
            commit_limit: (p.CommitLimit * p.PageSize) as u64,
            cache: (p.SystemCache * p.PageSize) as u64,
            standby_low: valid.then_some((lists.standby[0] * p.PageSize) as u64),
            standby_all: valid.then_some((lists.standby.iter().sum::<usize>() * p.PageSize) as u64),
            modified: valid.then_some((lists.modified * p.PageSize) as u64),
        })
    }
}
pub fn elevated() -> bool {
    unsafe {
        let mut raw = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut raw) == 0 {
            return false;
        }
        let Some(token) = Handle::new(raw) else {
            return false;
        };
        let mut elevation = TOKEN_ELEVATION::default();
        let mut length = 0;
        GetTokenInformation(
            token.0,
            TokenElevation,
            (&mut elevation as *mut TOKEN_ELEVATION).cast(),
            std::mem::size_of_val(&elevation) as u32,
            &mut length,
        ) != 0
            && elevation.TokenIsElevated != 0
    }
}
struct Privilege {
    token: Handle,
    previous: TOKEN_PRIVILEGES,
}
impl Privilege {
    fn acquire(name: &str) -> Result<Self, String> {
        unsafe {
            let mut raw = std::ptr::null_mut();
            if OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                &mut raw,
            ) == 0
            {
                return Err("无法读取权限令牌".into());
            }
            let token = Handle::new(raw).ok_or("权限令牌无效")?;
            let mut requested = TOKEN_PRIVILEGES {
                PrivilegeCount: 1,
                ..Default::default()
            };
            if LookupPrivilegeValueW(
                std::ptr::null(),
                wide(name).as_ptr(),
                &mut requested.Privileges[0].Luid,
            ) == 0
            {
                return Err("系统未提供所需权限".into());
            }
            requested.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
            let mut previous = TOKEN_PRIVILEGES::default();
            let mut length = 0;
            SetLastError(0);
            if AdjustTokenPrivileges(
                token.0,
                0,
                &requested,
                std::mem::size_of_val(&previous) as u32,
                &mut previous,
                &mut length,
            ) == 0
                || GetLastError() != 0
            {
                return Err(format!(
                    "无法取得 {name} 权限：{}",
                    std::io::Error::last_os_error()
                ));
            }
            Ok(Self { token, previous })
        }
    }
}
impl Drop for Privilege {
    fn drop(&mut self) {
        unsafe {
            AdjustTokenPrivileges(
                self.token.0,
                0,
                &self.previous,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }
    }
}
fn nt_set<T>(class: i32, value: &mut T) -> Result<(), String> {
    nt_result(unsafe {
        NtSetSystemInformation(
            class,
            (value as *mut T).cast(),
            std::mem::size_of_val(value) as u32,
        )
    })
}
fn nt_result(status: i32) -> Result<(), String> {
    if status >= 0 {
        Ok(())
    } else {
        Err(format!(
            "Windows 未完成操作：NTSTATUS 0x{:08X}（可能缺少权限或当前系统不支持）",
            status as u32
        ))
    }
}
fn run(operation: Operation) -> Result<String, String> {
    match operation {
        Operation::FileCache => {
            let _privilege = Privilege::acquire("SeIncreaseQuotaPrivilege")?;
            if unsafe { SetSystemFileCacheSize(usize::MAX, usize::MAX, 0) } == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            Ok("系统文件缓存工作集收缩请求已完成".into())
        }
        Operation::VolumeCache => flush_volumes(),
        Operation::Registry => {
            nt_result(unsafe { NtSetSystemInformation(156, std::ptr::null_mut(), 0) })?;
            Ok("注册表刷新请求已完成".into())
        }
        Operation::Combine => {
            let _privilege = Privilege::acquire("SeProfileSingleProcessPrivilege")?;
            let mut info = CombineInfo::default();
            nt_set(130, &mut info)?;
            Ok(format!("Windows 报告合并 {} 页", info.pages))
        }
        operation => {
            let _privilege = Privilege::acquire("SeProfileSingleProcessPrivilege")?;
            let mut command = match operation {
                Operation::WorkingSets => 2i32,
                Operation::Modified => 3,
                Operation::Standby => 4,
                Operation::LowStandby => 5,
                _ => unreachable!(),
            };
            nt_set(80, &mut command)?;
            Ok("Windows 已接受并完成该项请求".into())
        }
    }
}
fn flush_volumes() -> Result<String, String> {
    unsafe {
        let mut buffer = vec![0u16; 1024];
        let search = FindFirstVolumeW(buffer.as_mut_ptr(), buffer.len() as u32);
        if search == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let result = {
            let mut successes = 0;
            let mut errors = Vec::new();
            loop {
                let volume = from_wide(&buffer);
                if GetDriveTypeW(wide(&volume).as_ptr()) == 3 {
                    let path = wide(volume.trim_end_matches('\\'));
                    let raw = CreateFileW(
                        path.as_ptr(),
                        GENERIC_WRITE,
                        FILE_SHARE_READ | FILE_SHARE_WRITE,
                        std::ptr::null(),
                        OPEN_EXISTING,
                        0,
                        std::ptr::null_mut(),
                    );
                    if let Some(handle) = Handle::new(raw) {
                        if FlushFileBuffers(handle.0) != 0 {
                            successes += 1;
                        } else {
                            errors.push(format!("{volume}：{}", std::io::Error::last_os_error()));
                        }
                    } else {
                        errors.push(format!("{volume}：{}", std::io::Error::last_os_error()));
                    }
                }
                if FindNextVolumeW(search, buffer.as_mut_ptr(), buffer.len() as u32) == 0 {
                    if GetLastError() != ERROR_NO_MORE_FILES {
                        errors.push(std::io::Error::last_os_error().to_string());
                    }
                    break;
                }
            }
            if errors.is_empty() && successes > 0 {
                Ok(format!("已刷新 {successes} 个本地固定卷"))
            } else {
                Err(format!(
                    "成功 {successes} 个卷；{}",
                    if errors.is_empty() {
                        "没有可处理的本地固定卷".into()
                    } else {
                        errors.join("；")
                    }
                ))
            }
        };
        FindVolumeClose(search);
        result
    }
}
pub fn execute(operations: &[Operation], automatic: bool) -> Result<CleanupReport, String> {
    if !elevated() {
        return Err("需要管理员权限，未执行任何清理".into());
    }
    if operations.is_empty() {
        return Err("未选择清理方式".into());
    }
    let before = inspect()?;
    let results = operations
        .iter()
        .map(|operation| match run(*operation) {
            Ok(detail) => OperationResult {
                operation: *operation,
                success: true,
                detail,
            },
            Err(detail) => OperationResult {
                operation: *operation,
                success: false,
                detail,
            },
        })
        .collect();
    let (after, measurement_error) = match inspect() {
        Ok(status) => (Some(status), None),
        Err(error) => (None, Some(error)),
    };
    Ok(CleanupReport {
        time_label: {
            let mut local = SYSTEMTIME::default();
            unsafe { GetLocalTime(&mut local) };
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                local.wYear, local.wMonth, local.wDay, local.wHour, local.wMinute, local.wSecond
            )
        },
        at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        automatic,
        before,
        after,
        measurement_error,
        results,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_combine_structure_has_expected_abi_size() {
        assert_eq!(
            std::mem::size_of::<CombineInfo>(),
            if cfg!(target_pointer_width = "64") {
                24
            } else {
                12
            }
        );
        assert!(nt_result(0).is_ok());
        assert!(nt_result(0xC0000061u32 as i32).is_err());
    }
    #[test]
    fn readonly_status_reports_real_memory_without_cleaning() {
        let status = inspect().unwrap();
        assert!(status.total > 0);
        assert!(status.available <= status.total);
        assert!(status.load <= 100);
    }
}
