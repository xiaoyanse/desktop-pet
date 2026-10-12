use crate::domain::screenshot::Region;
use windows_sys::Win32::{
    Foundation::*, Graphics::Dwm::*, System::Threading::GetCurrentProcessId,
    UI::WindowsAndMessaging::*,
};

pub fn region(handle: isize, bounds: Region) -> Option<Region> {
    unsafe {
        let hwnd = handle as HWND;
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 || pid == GetCurrentProcessId() {
            return None;
        }
        let mut cloaked = 0u32;
        if DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED as u32,
            (&mut cloaked as *mut u32).cast(),
            4,
        ) >= 0
            && cloaked != 0
        {
            return None;
        }
        let mut rect: RECT = Default::default();
        if DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            (&mut rect as *mut RECT).cast(),
            std::mem::size_of::<RECT>() as u32,
        ) < 0
            && GetWindowRect(hwnd, &mut rect) == 0
        {
            return None;
        }
        if rect.right <= rect.left || rect.bottom <= rect.top {
            return None;
        }
        bounds.intersect(Region {
            left: rect.left,
            top: rect.top,
            width: (rect.right as i64 - rect.left as i64) as u32,
            height: (rect.bottom as i64 - rect.top as i64) as u32,
        })
    }
}
pub(in crate::infrastructure::windows::screenshot) fn validate(
    selection: &crate::domain::screenshot::Selection,
    bounds: Region,
) -> Result<(), String> {
    if let Some(window) = &selection.window {
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(window.handle as HWND, &mut pid);
        }
        if pid == 0
            || pid != window.pid
            || region(window.handle as isize, bounds) != Some(selection.region)
        {
            return Err("选定窗口已关闭或变化，请重新截图".into());
        }
    }
    Ok(())
}
struct Search {
    point: [i32; 2],
    bounds: Region,
    found: Option<(isize, Region)>,
}
pub fn at(point: [i32; 2], bounds: Region) -> Option<(isize, Region)> {
    let mut search = Search {
        point,
        bounds,
        found: None,
    };
    unsafe {
        EnumWindows(Some(enumerate), (&mut search as *mut Search) as isize);
    }
    search.found
}
unsafe extern "system" fn enumerate(hwnd: HWND, data: LPARAM) -> i32 {
    unsafe {
        let search = &mut *(data as *mut Search);
        if let Some(region) = region(hwnd as isize, search.bounds) {
            let [x, y] = search.point;
            if x >= region.left
                && y >= region.top
                && (x as i64) < region.left as i64 + region.width as i64
                && (y as i64) < region.top as i64 + region.height as i64
            {
                search.found = Some((hwnd as isize, region));
                return 0;
            }
        }
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_window_selection_excludes_the_tools_own_window() {
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        unsafe {
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class.as_ptr(),
                std::ptr::null(),
                WS_POPUP | WS_VISIBLE,
                -30000,
                -30000,
                2,
                2,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );
            assert!(!hwnd.is_null());
            assert_ne!(IsWindowVisible(hwnd), 0);
            let selected = region(
                hwnd as isize,
                Region {
                    left: -32000,
                    top: -32000,
                    width: 64000,
                    height: 64000,
                },
            );
            DestroyWindow(hwnd);
            assert!(selected.is_none());
        }
    }
}
