use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::ScreenToClient,
    UI::{HiDpi::GetDpiForWindow, Shell::*, WindowsAndMessaging::*},
};

const ID: usize = 0x0440;
const ATTACHED: *const u16 = windows_sys::core::w!("DesktopMeter.SettingsSurface");

pub fn attach(hwnd: HWND) {
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid != std::process::id() || !GetPropW(hwnd, ATTACHED).is_null() {
            return;
        }
        if SetWindowSubclass(hwnd, Some(callback), ID, 0) != 0 {
            SetPropW(hwnd, ATTACHED, ID as HANDLE);
            repair_position(hwnd);
        }
    }
}

fn repair_position(hwnd: HWND) {
    unsafe {
        if IsZoomed(hwnd) != 0 || IsIconic(hwnd) != 0 {
            return;
        }
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return;
        }
        let Some(area) = super::docking::area_for(&rect) else {
            return;
        };
        let bounded = super::docking::constrain(rect, area);
        if (rect.left, rect.top, rect.right, rect.bottom)
            != (bounded.left, bounded.top, bounded.right, bounded.bottom)
        {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                bounded.left,
                bounded.top,
                bounded.right - bounded.left,
                bounded.bottom - bounded.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
}

// Resolve the resize border before rendering: a fast drag can finish before
// egui's deferred BeginResize command reaches the Windows message loop.
fn hit(point: [f32; 2], size: [f32; 2]) -> Option<i32> {
    let [x, y] = point;
    let [width, height] = size;
    if x < 0.0 || y < 0.0 || x >= width || y >= height {
        return None;
    }
    let left = x < 6.0;
    let right = x >= width - 6.0;
    let top = y < 6.0;
    let bottom = y >= height - 6.0;
    match (left, right, top, bottom) {
        (true, _, true, _) => Some(HTTOPLEFT as i32),
        (_, true, true, _) => Some(HTTOPRIGHT as i32),
        (true, _, _, true) => Some(HTBOTTOMLEFT as i32),
        (_, true, _, true) => Some(HTBOTTOMRIGHT as i32),
        (true, _, _, _) => Some(HTLEFT as i32),
        (_, true, _, _) => Some(HTRIGHT as i32),
        (_, _, true, _) => Some(HTTOP as i32),
        (_, _, _, true) => Some(HTBOTTOM as i32),
        _ if y < 44.0 && x < width - 138.0 => Some(HTCAPTION as i32),
        _ => None,
    }
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
        if msg == WM_NCHITTEST && IsZoomed(hwnd) == 0 {
            let mut point = POINT {
                x: l as i16 as i32,
                y: (l >> 16) as i16 as i32,
            };
            let mut rect = RECT::default();
            if ScreenToClient(hwnd, &mut point) != 0 && GetClientRect(hwnd, &mut rect) != 0 {
                let scale = GetDpiForWindow(hwnd).max(96) as f32 / 96.0;
                if let Some(area) = hit(
                    [point.x as f32 / scale, point.y as f32 / scale],
                    [rect.right as f32 / scale, rect.bottom as f32 / scale],
                ) {
                    return area as isize;
                }
            }
        }
        if msg == WM_NCDESTROY {
            RemoveWindowSubclass(hwnd, Some(callback), ID);
            RemovePropW(hwnd, ATTACHED);
        }
        let result = DefSubclassProc(hwnd, msg, w, l);
        if matches!(
            msg,
            WM_WINDOWPOSCHANGED | WM_EXITSIZEMOVE | WM_DISPLAYCHANGE
        ) {
            // Restore/resize must settle before measuring the actual physical window bounds.
            repair_position(hwnd);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_edges_resize_but_leave_window_buttons_and_content_as_client() {
        let size = [840.0, 600.0];
        for (point, expected) in [
            ([2.0, 2.0], HTTOPLEFT),
            ([838.0, 2.0], HTTOPRIGHT),
            ([2.0, 598.0], HTBOTTOMLEFT),
            ([838.0, 598.0], HTBOTTOMRIGHT),
            ([420.0, 2.0], HTTOP),
            ([420.0, 598.0], HTBOTTOM),
            ([2.0, 300.0], HTLEFT),
            ([838.0, 300.0], HTRIGHT),
            ([420.0, 24.0], HTCAPTION),
        ] {
            assert_eq!(hit(point, size), Some(expected as i32));
        }
        for point in [
            [812.0, 24.0],
            [766.0, 24.0],
            [720.0, 24.0],
            [420.0, 300.0],
            [-1.0, 0.0],
        ] {
            assert_eq!(hit(point, size), None);
        }
    }

    #[test]
    fn hidden_window_bounds_are_repaired_without_activation_or_z_order_change() {
        struct TestWindow(HWND);
        impl Drop for TestWindow {
            fn drop(&mut self) {
                unsafe {
                    DestroyWindow(self.0);
                }
            }
        }
        unsafe {
            let class: Vec<_> = "STATIC\0".encode_utf16().collect();
            let title: Vec<_> = "DesktopMeter hidden bounds regression\0"
                .encode_utf16()
                .collect();
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                WS_POPUP,
                -55,
                247,
                1040,
                820,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );
            assert!(!hwnd.is_null());
            let owned = TestWindow(hwnd);
            repair_position(owned.0);
            for iteration in 0..3 {
                let mut rect = RECT::default();
                assert_ne!(GetWindowRect(hwnd, &mut rect), 0);
                let area = super::super::docking::area_for(&rect).unwrap();
                assert!(
                    rect.left >= area.left
                        && rect.top >= area.top
                        && rect.right <= area.right
                        && rect.bottom <= area.bottom
                );
                if iteration == 0 {
                    // Reproduce the old -55px restore bounds on an invisible test-owned window.
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        area.left - 55,
                        area.top + 247,
                        1040,
                        820,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                    repair_position(hwnd);
                } else if iteration == 1 {
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        area.right - 100,
                        area.bottom - 100,
                        1040,
                        820,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                    repair_position(hwnd);
                }
                assert_eq!(IsWindowVisible(hwnd), 0);
            }
        }
    }
}
