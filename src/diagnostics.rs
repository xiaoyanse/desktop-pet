use crate::native::{Handle, from_wide, wide};
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, System::Threading::*, UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*}};

pub fn inspect(path: &std::path::Path) -> Result<(), String> {
    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), wide("桌面资源监视器").as_ptr());
        if hwnd.is_null() { return Err("未找到监视器窗口".into()); }
        let mut pid = 0; GetWindowThreadProcessId(hwnd, &mut pid);
        let handle = Handle::new(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid)).ok_or("无法读取监视器进程")?;
        let mut exe = vec![0u16; 32768]; let mut length = exe.len() as u32;
        if QueryFullProcessImageNameW(handle.0, 0, exe.as_mut_ptr(), &mut length) == 0 { return Err("无法读取可执行文件路径".into()); }
        let expected = std::env::current_exe().map_err(|e| e.to_string())?;
        let actual = from_wide(&exe[..length as usize]);
        if !actual.eq_ignore_ascii_case(&expected.to_string_lossy()) { return Err("窗口来自另一个安装位置，拒绝读取".into()); }
        let mut rect = RECT::default(); if GetWindowRect(hwnd, &mut rect) == 0 { return Err("窗口边界读取失败".into()); }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if GetMonitorInfoW(monitor, &mut info) == 0 { return Err("显示器工作区域读取失败".into()); }
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let report = serde_json::json!({"pid": pid, "executable": actual, "dpi": GetDpiForWindow(hwnd),
            "rect": [rect.left, rect.top, rect.right, rect.bottom], "width": rect.right-rect.left, "height": rect.bottom-rect.top,
            "workArea": [info.rcWork.left, info.rcWork.top, info.rcWork.right, info.rcWork.bottom],
            "topmost": style & WS_EX_TOPMOST != 0, "passthrough": style & WS_EX_TRANSPARENT != 0,
            "layered": style & WS_EX_LAYERED != 0 });
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
        std::fs::write(path, serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }
}
