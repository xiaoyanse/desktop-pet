use crate::infrastructure::windows::native;
use windows_sys::Win32::{
    Foundation::*,
    UI::{Shell::*, WindowsAndMessaging::*},
};

const ID: usize = 0x043F;
pub fn attach(hwnd: HWND) {
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let desired = floating_style(style);
        if style != desired {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired as isize);
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
        SetWindowSubclass(hwnd, Some(callback), ID, 0);
    }
}
pub fn auxiliary() {
    unsafe {
        let settings = FindWindowW(std::ptr::null(), native::wide("资源监视器 · 设置").as_ptr());
        let mut settings_pid = 0;
        if !settings.is_null() {
            GetWindowThreadProcessId(settings, &mut settings_pid);
            if settings_pid == std::process::id() {
                super::settings_surface::attach(settings);
            }
        }
        for title in [
            "资源助手 · 快捷操作",
            "资源助手 · 计时提醒",
            "资源助手 · 待办提醒",
            "资源助手 · 会议提醒",
            "桌面仪表 · 置顶贴图",
        ] {
            let hwnd = FindWindowW(std::ptr::null(), native::wide(title).as_ptr());
            let mut pid = 0;
            if !hwnd.is_null() {
                GetWindowThreadProcessId(hwnd, &mut pid);
                if pid == std::process::id() {
                    attach(hwnd);
                    if title == "桌面仪表 · 置顶贴图" {
                        super::settings_surface::attach_bounded(hwnd);
                    }
                }
            }
        }
    }
}
fn floating_style(style: u32) -> u32 {
    (style & !WS_EX_APPWINDOW) | WS_EX_TOOLWINDOW
}
unsafe extern "system" fn callback(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    unsafe {
        if msg == WM_STYLECHANGING && w as i32 == GWL_EXSTYLE && l != 0 {
            let style = &mut *(l as *mut STYLESTRUCT);
            style.styleNew = floating_style(style.styleNew);
        }
        if msg == WM_NCDESTROY {
            RemoveWindowSubclass(hwnd, Some(callback), ID);
        }
        DefSubclassProc(hwnd, msg, w, l)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn floating_windows_keep_topmost_and_passthrough_but_leave_app_switcher() {
        let style = floating_style(WS_EX_APPWINDOW | WS_EX_TOPMOST | WS_EX_TRANSPARENT);
        assert_eq!(style & WS_EX_APPWINDOW, 0);
        assert_ne!(style & WS_EX_TOOLWINDOW, 0);
        assert_ne!(style & WS_EX_TOPMOST, 0);
        assert_ne!(style & WS_EX_TRANSPARENT, 0);
        assert_eq!(floating_style(style), style);
    }
}
