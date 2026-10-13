use windows_sys::Win32::{
    Foundation::{HANDLE, HWND, RECT},
    Graphics::Gdi::{CreateRoundRectRgn, DeleteObject, SetWindowRgn},
    UI::{
        HiDpi::GetDpiForWindow,
        WindowsAndMessaging::{GetPropW, GetWindowRect, IsZoomed, RemovePropW, SetPropW},
    },
};
const SHAPE: *const u16 = windows_sys::core::w!("DesktopMeter.SettingsShape");

pub(super) fn detach(hwnd: HWND) {
    unsafe {
        RemovePropW(hwnd, SHAPE);
    }
}

pub(super) fn update(hwnd: HWND) {
    unsafe {
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect) != 0 {
            apply(
                hwnd,
                rect.right - rect.left,
                rect.bottom - rect.top,
                GetDpiForWindow(hwnd).max(96),
                IsZoomed(hwnd) != 0,
            );
        }
    }
}

fn apply(hwnd: HWND, width: i32, height: i32, dpi: u32, maximized: bool) {
    unsafe {
        if width <= 0 || height <= 0 || width > 65535 || height > 65535 || dpi > 65535 {
            return;
        }
        let signature = width as usize
            | (height as usize) << 16
            | (dpi as usize) << 32
            | usize::from(maximized) << 48;
        if GetPropW(hwnd, SHAPE) as usize == signature {
            return;
        }
        // SetWindowRgn can synchronously emit resize messages; mark first to prevent recursion.
        if SetPropW(hwnd, SHAPE, signature as HANDLE) == 0 {
            return;
        }
        if maximized {
            if SetWindowRgn(hwnd, std::ptr::null_mut(), 1) == 0 {
                detach(hwnd);
            }
            return;
        }
        // Match the 18 logical-pixel painted corner, clipping the native surface itself.
        let diameter = (36.0 * dpi as f64 / 96.0).round() as i32;
        let region = CreateRoundRectRgn(0, 0, width + 1, height + 1, diameter, diameter);
        if region.is_null() {
            detach(hwnd);
        } else if SetWindowRgn(hwnd, region, 1) == 0 {
            // On success Windows owns the region. Only delete it on failure.
            DeleteObject(region);
            detach(hwnd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::{Graphics::Gdi::*, UI::WindowsAndMessaging::*};

    struct Window(HWND);
    impl Drop for Window {
        fn drop(&mut self) {
            unsafe {
                DestroyWindow(self.0);
            }
        }
    }
    struct Region(HRGN);
    impl Drop for Region {
        fn drop(&mut self) {
            unsafe {
                DeleteObject(self.0);
            }
        }
    }
    #[test]
    fn native_clip_removes_all_four_corners_and_tracks_resize_and_dpi() {
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                windows_sys::core::w!("STATIC"),
                windows_sys::core::w!("DesktopMeter hidden corner regression"),
                WS_POPUP,
                100,
                100,
                840,
                600,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );
            assert!(!hwnd.is_null());
            let owned = Window(hwnd);
            super::super::settings_surface::attach(hwnd);
            let region = Region(CreateRectRgn(0, 0, 0, 0));
            for (width, height, dpi) in [(840, 600, 96), (1000, 700, 144), (1040, 820, 192)] {
                SetWindowPos(
                    owned.0,
                    std::ptr::null_mut(),
                    0,
                    0,
                    width,
                    height,
                    SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
                // Resize is wired to the subclass before any explicit DPI fixture is applied.
                assert_eq!(GetWindowRgn(hwnd, region.0), COMPLEXREGION);
                assert_eq!(PtInRegion(region.0, width / 2, height - 2), 1);
                apply(hwnd, width, height, dpi, false);
                assert_eq!(GetWindowRgn(hwnd, region.0), COMPLEXREGION);
                for (x, y) in [
                    (0, 0),
                    (width - 1, 0),
                    (0, height - 1),
                    (width - 1, height - 1),
                ] {
                    assert_eq!(PtInRegion(region.0, x, y), 0);
                }
                assert_eq!(PtInRegion(region.0, width / 2, height / 2), 1);
                assert_eq!(PtInRegion(region.0, width / 2, 0), 1);
                assert_eq!(IsWindowVisible(hwnd), 0);
            }
            apply(hwnd, 1040, 820, 192, true);
            assert_eq!(GetWindowRgn(hwnd, region.0), 0);
            apply(hwnd, 1040, 820, 192, false);
            assert_eq!(GetWindowRgn(hwnd, region.0), COMPLEXREGION);
        }
    }
}
