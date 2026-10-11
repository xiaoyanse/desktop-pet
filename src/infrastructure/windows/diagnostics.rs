use super::native::{Handle, from_wide, wide};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::Threading::*,
    UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
};

pub fn inspect(path: &std::path::Path) -> Result<(), String> {
    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), wide("桌面资源监视器").as_ptr());
        if hwnd.is_null() {
            return Err("未找到监视器窗口".into());
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let handle = Handle::new(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid))
            .ok_or("无法读取监视器进程")?;
        let mut exe = vec![0u16; 32768];
        let mut length = exe.len() as u32;
        if QueryFullProcessImageNameW(handle.0, 0, exe.as_mut_ptr(), &mut length) == 0 {
            return Err("无法读取可执行文件路径".into());
        }
        let expected = std::env::current_exe().map_err(|e| e.to_string())?;
        let actual = from_wide(&exe[..length as usize]);
        if !actual.eq_ignore_ascii_case(&expected.to_string_lossy()) {
            return Err("窗口来自另一个安装位置，拒绝读取".into());
        }
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return Err("窗口边界读取失败".into());
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return Err("显示器工作区域读取失败".into());
        }
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let auxiliary = |title: &str| {
            let window = FindWindowW(std::ptr::null(), wide(title).as_ptr());
            let mut owner = 0;
            GetWindowThreadProcessId(window, &mut owner);
            if window.is_null() || owner != pid {
                return None;
            }
            let style = GetWindowLongPtrW(window, GWL_EXSTYLE) as u32;
            let frame_style = GetWindowLongPtrW(window, GWL_STYLE) as u32;
            let mut bounds = RECT::default();
            GetWindowRect(window, &mut bounds);
            let mut client = RECT::default();
            let mut origin = POINT::default();
            if GetClientRect(window, &mut client) == 0 || ClientToScreen(window, &mut origin) == 0 {
                return None;
            }
            // Winit keeps caption style bits for Windows snapping even without decorations.
            // Measure the actual non-client area instead of treating those bits as a visible frame.
            let insets = [
                origin.x - bounds.left,
                origin.y - bounds.top,
                bounds.right - origin.x - client.right,
                bounds.bottom - origin.y - client.bottom,
            ];
            Some(
                serde_json::json!({"toolWindow":style & WS_EX_TOOLWINDOW != 0,"appWindow":style & WS_EX_APPWINDOW != 0,
                    "captionStyleBit":frame_style & WS_CAPTION != 0,
                    "borderStyleBit":frame_style & (WS_BORDER | WS_DLGFRAME) != 0,
                    "nonClientInsets":insets,
                    "maximized":IsZoomed(window) != 0,"minimized":IsIconic(window) != 0,
                    "rect":[bounds.left,bounds.top,bounds.right,bounds.bottom]}),
            )
        };
        let report = serde_json::json!({"pid": pid, "executable": actual, "dpi": GetDpiForWindow(hwnd),
            "rect": [rect.left, rect.top, rect.right, rect.bottom], "width": rect.right-rect.left, "height": rect.bottom-rect.top,
            "workArea": [info.rcWork.left, info.rcWork.top, info.rcWork.right, info.rcWork.bottom],
            "topmost": style & WS_EX_TOPMOST != 0, "passthrough": style & WS_EX_TRANSPARENT != 0,
            "layered": style & WS_EX_LAYERED != 0,
            "toolWindow": style & WS_EX_TOOLWINDOW != 0, "appWindow": style & WS_EX_APPWINDOW != 0,
            "settings": auxiliary("资源监视器 · 设置"), "flyout": auxiliary("资源助手 · 快捷操作") });
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    }
}
