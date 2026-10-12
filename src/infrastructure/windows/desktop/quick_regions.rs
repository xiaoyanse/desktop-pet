use crate::infrastructure::windows::native;
use std::cell::RefCell;
use windows_sys::Win32::{
    Foundation::*,
    Graphics::{Dwm::DwmExtendFrameIntoClientArea, Gdi::*},
    UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
};

type Bounds = Vec<[i32; 4]>;
thread_local! { static LAST: RefCell<Option<(isize,Bounds)>> = const { RefCell::new(None) }; }
struct Region(HRGN);
impl Drop for Region {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                DeleteObject(self.0);
            }
        }
    }
}

pub fn apply(regions: &[[f32; 4]], preview: bool) -> Result<(), String> {
    unsafe {
        let title = if preview {
            "资源监视器 · 快捷入口试用"
        } else {
            "资源助手 · 快捷操作"
        };
        let hwnd = FindWindowW(std::ptr::null(), native::wide(title).as_ptr());
        if hwnd.is_null() {
            return Ok(());
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid != std::process::id() {
            return Err("快捷窗口来源不匹配".into());
        }
        let scale = GetDpiForWindow(hwnd).max(96) as f32 / 96.0;
        let bounds: Bounds = regions
            .iter()
            .filter(|r| r.iter().all(|v| v.is_finite()) && r[2] > r[0] && r[3] > r[1])
            .map(|r| r.map(|v| (v * scale).round() as i32))
            .collect();
        let mut old_box = RECT::default();
        if GetWindowRgnBox(hwnd, &mut old_box) != ERROR
            && LAST.with(|last| {
                last.borrow()
                    .as_ref()
                    .is_some_and(|(h, b)| *h == hwnd as isize && *b == bounds)
            })
        {
            return Ok(());
        }
        let mut union = Region(CreateRectRgn(0, 0, 0, 0));
        if union.0.is_null() {
            return Err("无法建立快捷入口区域".into());
        }
        for [x0, y0, x1, y1] in &bounds {
            let radius = ((y1 - y0) as f32).min(32.0 * scale).round() as i32;
            let part = Region(if (x1 - x0 - y1 + y0).abs() <= 1 {
                CreateEllipticRgn(*x0, *y0, *x1 + 1, *y1 + 1)
            } else {
                CreateRoundRectRgn(*x0, *y0, *x1 + 1, *y1 + 1, radius, radius)
            });
            if part.0.is_null() || CombineRgn(union.0, union.0, part.0, RGN_OR) == ERROR {
                return Err("无法组合快捷入口区域".into());
            }
        }
        if SetWindowRgn(hwnd, union.0, 0) == 0 {
            return Err("无法应用快捷入口区域；已收起入口".into());
        }
        union.0 = std::ptr::null_mut();
        // A window region can make untouched OpenGL pixels opaque without full-client DWM composition.
        let margins = windows_sys::Win32::UI::Controls::MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
        let result = DwmExtendFrameIntoClientArea(hwnd, &margins);
        if result < 0 {
            return Err(format!("快捷入口透明合成失败：0x{:08X}", result as u32));
        }
        LAST.with(|last| *last.borrow_mut() = Some((hwnd as isize, bounds)));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disjoint_regions_leave_empty_space_outside_and_drop_handles() {
        unsafe {
            let union = Region(CreateRectRgn(0, 0, 0, 0));
            for [x0, y0, x1, y1] in [[10, 10, 110, 100], [220, 180, 420, 300]] {
                let part = Region(CreateRoundRectRgn(x0, y0, x1, y1, 22, 22));
                assert_ne!(CombineRgn(union.0, union.0, part.0, RGN_OR), ERROR);
            }
            assert_ne!(PtInRegion(union.0, 60, 60), 0);
            assert_ne!(PtInRegion(union.0, 300, 220), 0);
            assert_eq!(PtInRegion(union.0, 170, 140), 0);
            assert_eq!(PtInRegion(union.0, 1, 1), 0);
        }
    }
}
