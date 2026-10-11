use std::{cell::Cell, ptr::NonNull};
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, UI::{Shell::*, HiDpi::GetDpiForWindow, WindowsAndMessaging::*}};

const SUBCLASS_ID: usize = 0x043D;
const LEFT: u8 = 1;
const RIGHT: u8 = 2;
const TOP: u8 = 4;
const BOTTOM: u8 = 8;
struct State { enabled: Cell<bool>, anchors: Cell<u8>, resizing: Cell<bool>, moving: Cell<bool> }
pub struct Docking { hwnd: HWND, state: NonNull<State> }
impl Docking {
    pub fn attach(hwnd: HWND, enabled: bool) -> Option<Self> {
        let state = Box::new(State { enabled: Cell::new(enabled), anchors: Cell::new(0), resizing: Cell::new(false), moving: Cell::new(false) });
        let state = NonNull::new(Box::into_raw(state))?;
        unsafe {
            if SetWindowSubclass(hwnd, Some(callback), SUBCLASS_ID, state.as_ptr() as usize) == 0 {
                drop(Box::from_raw(state.as_ptr())); return None;
            }
        }
        let result = Self { hwnd, state };
        result.capture_anchors(); Some(result)
    }
    pub fn set_enabled(&self, enabled: bool) {
        unsafe {
            let was_enabled = self.state.as_ref().enabled.replace(enabled);
            if !enabled { self.state.as_ref().anchors.set(0); }
            else if !was_enabled { self.capture_anchors(); }
        }
    }
    fn capture_anchors(&self) {
        if let Some((rect, area)) = bounds(self.hwnd) {
            let (_, _, anchors) = snap_rect(rect, area, 1);
            unsafe { self.state.as_ref().anchors.set(anchors); }
        }
    }
}
impl Drop for Docking {
    fn drop(&mut self) { unsafe { RemoveWindowSubclass(self.hwnd, Some(callback), SUBCLASS_ID); drop(Box::from_raw(self.state.as_ptr())); } }
}
fn bounds(hwnd: HWND) -> Option<(RECT, RECT)> {
    unsafe {
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect) == 0 { return None; }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if GetMonitorInfoW(monitor, &mut info) == 0 { return None; }
        Some((rect, info.rcWork))
    }
}
fn snap_rect(rect: RECT, area: RECT, distance: i32) -> (i32, i32, u8) {
    let mut x = rect.left; let mut y = rect.top; let mut anchors = 0;
    let dl = (rect.left - area.left).abs(); let dr = (rect.right - area.right).abs();
    let dt = (rect.top - area.top).abs(); let db = (rect.bottom - area.bottom).abs();
    if dl <= distance && dl <= dr { x = area.left; anchors |= LEFT; }
    else if dr <= distance { x = area.right - (rect.right - rect.left); anchors |= RIGHT; }
    if dt <= distance && dt <= db { y = area.top; anchors |= TOP; }
    else if db <= distance { y = area.bottom - (rect.bottom - rect.top); anchors |= BOTTOM; }
    (x, y, anchors)
}
unsafe extern "system" fn callback(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM, _id: usize, data: usize) -> LRESULT {
    unsafe {
        let state = &*(data as *const State);
        if msg == WM_ENTERSIZEMOVE { state.moving.set(true); state.anchors.set(0); }
        if msg == WM_EXITSIZEMOVE {
            state.moving.set(false);
            if state.enabled.get() && let Some((rect, area)) = bounds(hwnd) {
                let distance = (GetDpiForWindow(hwnd).max(96) as i32 * 16) / 96;
                let (x, y, anchors) = snap_rect(rect, area, distance);
                state.anchors.set(anchors);
                if anchors != 0 { SetWindowPos(hwnd, std::ptr::null_mut(), x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE); }
            }
        }
        if msg == WM_SIZE && state.enabled.get() && !state.resizing.get() && !state.moving.get()
            && state.anchors.get() != 0 && let Some((rect, area)) = bounds(hwnd) {
            let anchors = state.anchors.get();
            let x = if anchors & RIGHT != 0 { area.right - (rect.right - rect.left) } else if anchors & LEFT != 0 { area.left } else { rect.left };
            let y = if anchors & BOTTOM != 0 { area.bottom - (rect.bottom - rect.top) } else if anchors & TOP != 0 { area.top } else { rect.top };
            state.resizing.set(true);
            SetWindowPos(hwnd, std::ptr::null_mut(), x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
            state.resizing.set(false);
        }
        if msg == WM_NCDESTROY { RemoveWindowSubclass(hwnd, Some(callback), SUBCLASS_ID); }
        DefSubclassProc(hwnd, msg, w, l)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn rect(x: i32, y: i32, w: i32, h: i32) -> RECT { RECT { left: x, top: y, right: x+w, bottom: y+h } }
    #[test]
    fn snaps_each_edge_and_corners_without_pulling_distant_windows() {
        let area = rect(0, 0, 1920, 1040);
        assert_eq!(snap_rect(rect(12, 100, 240, 280), area, 16), (0, 100, LEFT));
        assert_eq!(snap_rect(rect(1672, 100, 240, 280), area, 16), (1680, 100, RIGHT));
        assert_eq!(snap_rect(rect(800, 8, 240, 280), area, 16), (800, 0, TOP));
        assert_eq!(snap_rect(rect(800, 752, 240, 280), area, 16), (800, 760, BOTTOM));
        assert_eq!(snap_rect(rect(1672, 8, 240, 280), area, 16), (1680, 0, RIGHT|TOP));
        assert_eq!(snap_rect(rect(800, 200, 240, 280), area, 16), (800, 200, 0));
    }
    #[test]
    fn negative_monitor_coordinates_and_taskbar_work_area_are_supported() {
        assert_eq!(snap_rect(rect(-1910, -5, 240, 280), rect(-1920, 0, 1920, 1040), 16), (-1920, 0, LEFT|TOP));
    }
}
