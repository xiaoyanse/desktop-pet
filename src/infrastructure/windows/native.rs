use std::borrow::Cow;
use windows_sys::Win32::{
    Foundation::*,
    System::{Registry::*, Threading::*},
    UI::WindowsAndMessaging::*,
};

pub fn wide(value: impl AsRef<str>) -> Vec<u16> {
    value.as_ref().encode_utf16().chain(Some(0)).collect()
}
pub fn from_wide(value: &[u16]) -> String {
    String::from_utf16_lossy(&value[..value.iter().position(|c| *c == 0).unwrap_or(value.len())])
}
pub unsafe fn from_wide_ptr(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
    }
}
pub struct Handle(pub HANDLE);
impl Handle {
    pub fn new(handle: HANDLE) -> Option<Self> {
        (!handle.is_null() && handle != INVALID_HANDLE_VALUE).then_some(Self(handle))
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
pub fn request_instance(settings: bool, quit: bool) {
    unsafe {
        let hwnd = FindWindowW(wide("DesktopMeterTray").as_ptr(), std::ptr::null());
        if !hwnd.is_null() {
            PostMessageW(
                hwnd,
                WM_APP
                    + if quit {
                        5
                    } else if settings {
                        4
                    } else {
                        2
                    },
                0,
                0,
            );
        }
    }
}
pub fn single_instance(settings: bool) -> Option<Handle> {
    unsafe {
        let mutex = Handle::new(CreateMutexW(
            std::ptr::null(),
            0,
            wide("Local\\DesktopMeter.Rust.043").as_ptr(),
        ))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            request_instance(settings, false);
            None
        } else {
            Some(mutex)
        }
    }
}
pub fn message(text: &str) {
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide(text).as_ptr(),
            wide("桌面资源监视器").as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}
pub fn card_menu(hwnd: HWND, compact: bool) -> i32 {
    use super::desktop::menus::Glyph;
    super::desktop::menus::popup(
        hwnd,
        &[
            (
                1,
                if compact {
                    "恢复完整视图"
                } else {
                    "切换精简视图"
                },
                Glyph::Restore,
                true,
            ),
            (2, "设置", Glyph::Settings, true),
            (5, "切换桌面宠物", Glyph::Pet, true),
            (6, "快速清理", Glyph::Memory, true),
            (3, "切换背景显示", Glyph::Background, true),
            (4, "切换边框显示", Glyph::Border, true),
        ],
    )
}
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_NAME: &str = "DesktopMeter";
pub fn autostart() -> bool {
    unsafe {
        let mut key = std::ptr::null_mut();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide(RUN_KEY).as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &mut key,
        ) != 0
        {
            return false;
        }
        let mut bytes = 0;
        let status = RegQueryValueExW(
            key,
            wide(RUN_NAME).as_ptr(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut bytes,
        );
        RegCloseKey(key);
        status == 0 && bytes > 0
    }
}
pub fn set_autostart(enable: bool) -> Result<(), String> {
    let command = if enable {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        Some(wide(format!("\"{}\"", exe.display())))
    } else {
        None
    };
    unsafe {
        let mut key = std::ptr::null_mut();
        let result = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide(RUN_KEY).as_ptr(),
            0,
            std::ptr::null(),
            0,
            KEY_SET_VALUE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        );
        if result != 0 {
            return Err(format!("开机启动设置失败：{result}"));
        }
        let result = if let Some(command) = &command {
            RegSetValueExW(
                key,
                wide(RUN_NAME).as_ptr(),
                0,
                REG_SZ,
                command.as_ptr().cast(),
                (command.len() * 2) as u32,
            )
        } else {
            RegDeleteValueW(key, wide(RUN_NAME).as_ptr())
        };
        RegCloseKey(key);
        if result == 0 || (!enable && result == ERROR_FILE_NOT_FOUND) {
            Ok(())
        } else {
            Err(format!("开机启动设置失败：{result}"))
        }
    }
}
pub fn safe_position(hwnd: HWND) {
    unsafe {
        use windows_sys::Win32::Graphics::Gdi::*;
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return;
        }
        let monitor = MonitorFromRect(&rect, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return;
        }
        let area = info.rcWork;
        let x = rect.left.clamp(
            area.left,
            (area.right - (rect.right - rect.left)).max(area.left),
        );
        let y = rect.top.clamp(
            area.top,
            (area.bottom - (rect.bottom - rect.top)).max(area.top),
        );
        SetWindowPos(hwnd, HWND_TOPMOST, x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
    }
}
pub fn font_bytes() -> Option<(Cow<'static, str>, Vec<u8>)> {
    let root = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    ["msyh.ttc", "msyh.ttf", "simhei.ttf", "simsun.ttc"]
        .into_iter()
        .find_map(|name| {
            std::fs::read(format!("{root}\\Fonts\\{name}"))
                .ok()
                .map(|data| (Cow::Borrowed(name), data))
        })
}
