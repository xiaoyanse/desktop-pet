use crate::interfaces::{DesktopAction, Wake};
use std::{
    cell::{Cell, RefCell},
    ptr::NonNull,
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::ScreenToClient,
    UI::{
        Controls::WM_MOUSELEAVE, HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::*, Shell::*,
        WindowsAndMessaging::*,
    },
};

const ID: usize = 0x043E;
struct State {
    compact: Cell<bool>,
    draggable: Cell<bool>,
    drag_height: Cell<f32>,
    toolbar_width: Cell<f32>,
    actions: RefCell<Vec<DesktopAction>>,
    wake: Wake,
}
pub struct DragSurface {
    hwnd: HWND,
    state: NonNull<State>,
}
impl DragSurface {
    pub fn attach(hwnd: HWND, wake: Wake) -> Option<Self> {
        let state = NonNull::new(Box::into_raw(Box::new(State {
            compact: Cell::new(false),
            draggable: Cell::new(true),
            drag_height: Cell::new(280.0),
            toolbar_width: Cell::new(156.0),
            actions: RefCell::new(Vec::new()),
            wake,
        })))?;
        if unsafe { SetWindowSubclass(hwnd, Some(callback), ID, state.as_ptr() as usize) } == 0 {
            unsafe {
                drop(Box::from_raw(state.as_ptr()));
            }
            return None;
        }
        Some(Self { hwnd, state })
    }
    pub fn configure(&self, compact: bool, draggable: bool, drag_height: f32, toolbar_width: f32) {
        unsafe {
            self.state.as_ref().compact.set(compact);
            self.state.as_ref().draggable.set(draggable);
            self.state.as_ref().drag_height.set(drag_height);
            self.state.as_ref().toolbar_width.set(toolbar_width);
        }
    }
    pub fn actions(&self) -> Vec<DesktopAction> {
        unsafe { std::mem::take(&mut *self.state.as_ref().actions.borrow_mut()) }
    }
}
impl Drop for DragSurface {
    fn drop(&mut self) {
        unsafe {
            RemoveWindowSubclass(self.hwnd, Some(callback), ID);
            drop(Box::from_raw(self.state.as_ptr()));
        }
    }
}
fn caption(point: POINT, scale: f32, toolbar_width: f32, draggable: bool, height: i32) -> bool {
    // Leave the whole toolbar and the expanded detail actions as client controls.
    draggable
        && point.x >= 0
        && point.y >= 0
        && point.y < height
        && (point.y >= (40.0 * scale) as i32 || point.x >= (toolbar_width * scale) as i32)
}
unsafe extern "system" fn callback(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    unsafe {
        let state = &*(data as *const State);
        let client_point = || {
            let mut point = POINT {
                x: l as i16 as i32,
                y: (l >> 16) as i16 as i32,
            };
            ScreenToClient(hwnd, &mut point);
            point
        };
        if msg == WM_NCHITTEST {
            let result = DefSubclassProc(hwnd, msg, w, l);
            let mut rect = RECT::default();
            GetClientRect(hwnd, &mut rect);
            if result == HTCLIENT as isize
                && caption(
                    client_point(),
                    GetDpiForWindow(hwnd).max(96) as f32 / 96.0,
                    state.toolbar_width.get(),
                    state.draggable.get(),
                    rect.bottom.min(
                        (state.drag_height.get() * GetDpiForWindow(hwnd).max(96) as f32 / 96.0)
                            as i32,
                    ),
                )
            {
                return HTCAPTION as isize;
            }
            return result;
        }
        if msg == WM_NCMOUSEMOVE && w == HTCAPTION as usize {
            let mut tracking = TRACKMOUSEEVENT {
                cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE | TME_NONCLIENT,
                hwndTrack: hwnd,
                dwHoverTime: 0,
            };
            TrackMouseEvent(&mut tracking);
            let p = client_point();
            return DefSubclassProc(
                hwnd,
                WM_MOUSEMOVE,
                0,
                ((p.y as u16 as usize) << 16 | p.x as u16 as usize) as isize,
            );
        }
        if matches!(msg, WM_NCMOUSELEAVE | WM_MOUSELEAVE) {
            // Moving between our caption and toolbar is not leaving the window.
            let mut point = POINT::default();
            let mut rect = RECT::default();
            if GetCursorPos(&mut point) != 0
                && ScreenToClient(hwnd, &mut point) != 0
                && GetClientRect(hwnd, &mut rect) != 0
                && point.x >= 0
                && point.y >= 0
                && point.x < rect.right
                && point.y < rect.bottom
            {
                return 0;
            }
            return DefSubclassProc(hwnd, WM_MOUSELEAVE, 0, 0);
        }
        if matches!(msg, WM_NCRBUTTONDOWN | WM_NCRBUTTONUP) && w == HTCAPTION as usize {
            let p = client_point();
            return DefSubclassProc(
                hwnd,
                if msg == WM_NCRBUTTONDOWN {
                    WM_RBUTTONDOWN
                } else {
                    WM_RBUTTONUP
                },
                0,
                ((p.y as u16 as usize) << 16 | p.x as u16 as usize) as isize,
            );
        }
        if msg == WM_NCLBUTTONDBLCLK && w == HTCAPTION as usize {
            if state.compact.get() {
                state.actions.borrow_mut().push(DesktopAction::ToggleMode);
                (state.wake)();
            }
            return 0;
        }
        if state.toolbar_width.get() == 0.0 {
            let action = if msg == WM_NCLBUTTONDOWN && w == HTCAPTION as usize {
                Some(DesktopAction::PetReact)
            } else if msg == WM_EXITSIZEMOVE {
                Some(DesktopAction::PetDrop)
            } else {
                None
            };
            if let Some(action) = action {
                state.actions.borrow_mut().push(action);
                (state.wake)();
            }
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
    fn native_drag_reserves_toolbar_and_supports_dpi_and_lock() {
        assert!(!caption(POINT { x: 20, y: 20 }, 1.0, 185.0, true, 142));
        assert!(caption(POINT { x: 180, y: 80 }, 1.0, 185.0, true, 142));
        assert!(!caption(POINT { x: 180, y: 80 }, 1.0, 185.0, false, 142));
        assert!(!caption(POINT { x: 180, y: 40 }, 2.0, 185.0, true, 284));
        assert!(caption(POINT { x: 360, y: 160 }, 2.0, 185.0, true, 284));
        assert!(caption(POINT { x: 170, y: 20 }, 1.0, 156.0, true, 62));
        assert!(caption(POINT { x: 20, y: 20 }, 1.0, 0.0, true, 84));
        assert!(!caption(POINT { x: 180, y: 330 }, 1.0, 185.0, true, 300));
    }
}
