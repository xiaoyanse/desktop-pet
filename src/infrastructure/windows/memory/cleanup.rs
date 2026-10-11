use super::memory_operations;
use crate::infrastructure::windows::native::{self, Handle, wide};
use crate::{
    domain::cleanup::{CleanupReport, MemoryStatus, Operation, decode_mask},
    interfaces::{MemoryCleaner, Wake},
};
use std::{
    sync::mpsc,
    time::{SystemTime, UNIX_EPOCH},
};
use windows_sys::Win32::{
    Foundation::*,
    Storage::FileSystem::*,
    System::{Pipes::*, Threading::*},
    UI::{Shell::*, WindowsAndMessaging::SW_HIDE},
};

const MAX_RESULT: u32 = 64 * 1024;
pub struct WindowsCleaner;
impl MemoryCleaner for WindowsCleaner {
    fn request(
        &self,
        operations: Vec<Operation>,
        automatic: bool,
        wake: Wake,
    ) -> mpsc::Receiver<Result<CleanupReport, String>> {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = if memory_operations::elevated() {
                memory_operations::execute(&operations, automatic)
            } else if automatic {
                Err("自动清理缺少管理员权限，未执行".into())
            } else {
                launch(&operations)
            };
            let _ = tx.send(result);
            wake();
        });
        rx
    }
    fn elevated(&self) -> bool {
        memory_operations::elevated()
    }
    fn inspect(&self) -> Result<MemoryStatus, String> {
        memory_operations::inspect()
    }
}
fn valid_token(token: &str) -> bool {
    !token.is_empty() && token.len() < 64 && token.bytes().all(|c| c.is_ascii_digit() || c == b'-')
}
fn pipe_name(token: &str) -> Vec<u16> {
    wide(format!(r"\\.\pipe\DesktopMeter-{token}"))
}

pub fn helper(args: &[String]) {
    if args.len() != 2 {
        return;
    }
    let Some(token) = args.first().filter(|s| valid_token(s)) else {
        return;
    };
    let Some(operations) = args[1].parse::<u32>().ok().and_then(decode_mask) else {
        return;
    };
    unsafe {
        let Some(pipe) = Handle::new(CreateFileW(
            pipe_name(token).as_ptr(),
            GENERIC_WRITE,
            0,
            std::ptr::null(),
            OPEN_EXISTING,
            SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
            std::ptr::null_mut(),
        )) else {
            return;
        };
        let mut server = 0;
        if GetNamedPipeServerProcessId(pipe.0, &mut server) == 0
            || token.split('-').next().and_then(|s| s.parse::<u32>().ok()) != Some(server)
            || !same_executable(server)
        {
            return;
        }
        let result = memory_operations::execute(&operations, false);
        let Ok(bytes) = serde_json::to_vec(&result) else {
            return;
        };
        if bytes.len() > MAX_RESULT as usize {
            return;
        }
        let mut written = 0;
        WriteFile(
            pipe.0,
            bytes.as_ptr(),
            bytes.len() as u32,
            &mut written,
            std::ptr::null_mut(),
        );
    }
}
fn launch(operations: &[Operation]) -> Result<CleanupReport, String> {
    let mask = operations
        .iter()
        .fold(0, |bits, operation| bits | operation.bit());
    if decode_mask(mask).is_none() {
        return Err("无效的清理选择".into());
    }
    let token = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let file = wide(exe.to_string_lossy());
    let verb = wide("runas");
    let params = wide(format!("--clean-helper {token} {mask}"));
    unsafe {
        let pipe = Handle::new(CreateNamedPipeW(
            pipe_name(&token).as_ptr(),
            PIPE_ACCESS_INBOUND | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            MAX_RESULT,
            MAX_RESULT,
            0,
            std::ptr::null(),
        ))
        .ok_or("无法创建清理结果通道")?;
        ConnectNamedPipe(pipe.0, std::ptr::null_mut());
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
            lpVerb: verb.as_ptr(),
            lpFile: file.as_ptr(),
            lpParameters: params.as_ptr(),
            nShow: SW_HIDE,
            ..Default::default()
        };
        if ShellExecuteExW(&mut info) == 0 {
            return Err(format!(
                "未执行清理（授权取消或启动失败）：{}",
                std::io::Error::last_os_error()
            ));
        }
        let process = Handle::new(info.hProcess).ok_or("辅助进程没有返回句柄")?;
        let mut bytes = vec![0u8; MAX_RESULT as usize];
        loop {
            let mut read = 0;
            if ReadFile(
                pipe.0,
                bytes.as_mut_ptr(),
                MAX_RESULT,
                &mut read,
                std::ptr::null_mut(),
            ) != 0
                && read > 0
            {
                let mut client = 0;
                if GetNamedPipeClientProcessId(pipe.0, &mut client) == 0
                    || client != GetProcessId(process.0)
                {
                    return Err("清理结果来源校验失败".into());
                }
                return serde_json::from_slice::<Result<CleanupReport, String>>(
                    &bytes[..read as usize],
                )
                .map_err(|e| format!("清理结果格式错误：{e}"))?;
            }
            if WaitForSingleObject(process.0, 0) == WAIT_OBJECT_0 {
                if ReadFile(
                    pipe.0,
                    bytes.as_mut_ptr(),
                    MAX_RESULT,
                    &mut read,
                    std::ptr::null_mut(),
                ) != 0
                    && read > 0
                {
                    let mut client = 0;
                    if GetNamedPipeClientProcessId(pipe.0, &mut client) == 0
                        || client != GetProcessId(process.0)
                    {
                        return Err("清理结果来源校验失败".into());
                    }
                    return serde_json::from_slice::<Result<CleanupReport, String>>(
                        &bytes[..read as usize],
                    )
                    .map_err(|e| format!("清理结果格式错误：{e}"))?;
                }
                return Err("辅助进程已退出，没有返回有效结果；未确认清理成功".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
fn same_executable(pid: u32) -> bool {
    unsafe {
        let Some(process) = Handle::new(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid))
        else {
            return false;
        };
        let mut path = vec![0u16; 32768];
        let mut size = path.len() as u32;
        if QueryFullProcessImageNameW(process.0, 0, path.as_mut_ptr(), &mut size) == 0 {
            return false;
        }
        let Ok(exe) = std::env::current_exe() else {
            return false;
        };
        native::from_wide(&path[..size as usize]).eq_ignore_ascii_case(&exe.to_string_lossy())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn named_pipe_exchanges_authenticated_result_without_cleaning() {
        let token = format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let name = pipe_name(&token);
        unsafe {
            let pipe = Handle::new(CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_INBOUND | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_MESSAGE
                    | PIPE_READMODE_MESSAGE
                    | PIPE_NOWAIT
                    | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                MAX_RESULT,
                MAX_RESULT,
                0,
                std::ptr::null(),
            ))
            .unwrap();
            ConnectNamedPipe(pipe.0, std::ptr::null_mut());
            let expected: Result<CleanupReport, String> = Err("测试返回：未执行清理".into());
            let bytes = serde_json::to_vec(&expected).unwrap();
            let client = Handle::new(CreateFileW(
                name.as_ptr(),
                GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
                std::ptr::null_mut(),
            ))
            .unwrap();
            let mut server_pid = 0;
            assert_ne!(GetNamedPipeServerProcessId(client.0, &mut server_pid), 0);
            assert_eq!(server_pid, std::process::id());
            assert!(same_executable(server_pid));
            let mut written = 0;
            assert_ne!(
                WriteFile(
                    client.0,
                    bytes.as_ptr(),
                    bytes.len() as u32,
                    &mut written,
                    std::ptr::null_mut()
                ),
                0
            );
            assert_eq!(written as usize, bytes.len());
            drop(client);
            let mut output = vec![0; MAX_RESULT as usize];
            let mut read = 0;
            assert_ne!(
                ReadFile(
                    pipe.0,
                    output.as_mut_ptr(),
                    MAX_RESULT,
                    &mut read,
                    std::ptr::null_mut()
                ),
                0
            );
            let mut client_pid = 0;
            assert_ne!(GetNamedPipeClientProcessId(pipe.0, &mut client_pid), 0);
            assert_eq!(client_pid, std::process::id());
            let result: Result<CleanupReport, String> =
                serde_json::from_slice(&output[..read as usize]).unwrap();
            assert_eq!(result.err(), expected.err());
        }
    }
    #[test]
    fn elevated_helper_rejects_paths_and_invalid_operation_masks() {
        assert!(valid_token("123-987654"));
        assert!(!valid_token("..\\file"));
        assert!(!valid_token("C:\\file"));
        assert!(!valid_token(""));
        assert!(decode_mask(256).is_none());
        assert!(decode_mask(0).is_none());
    }
}
