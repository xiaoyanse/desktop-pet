mod paint;
mod windows;
use crate::domain::screenshot::{Mode, Region, Selection};
use std::{
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
pub(super) use windows::validate;
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{HiDpi::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

struct State<'a> {
    mode: Mode,
    bounds: Region,
    cancel: &'a AtomicBool,
    start: Option<[i32; 2]>,
    points: Vec<[i32; 2]>,
    selected: Option<Selection>,
    window: Option<isize>,
    result: Option<Selection>,
    error: Option<String>,
    notice: Option<String>,
    created: Instant,
}
struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(self.0) != 0 {
                DestroyWindow(self.0);
            }
        }
    }
}
struct Dpi(DPI_AWARENESS_CONTEXT);
impl Drop for Dpi {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                SetThreadDpiAwarenessContext(self.0);
            }
        }
    }
}

pub fn select(
    mode: Mode,
    bounds: Region,
    cancel: &AtomicBool,
) -> Result<Option<Selection>, String> {
    if cancel.load(Ordering::Relaxed) {
        return Ok(None);
    }
    if mode == Mode::Fullscreen {
        return Ok(Some(Selection {
            region: bounds,
            polygon: vec![],
            window: None,
        }));
    }
    unsafe {
        let _dpi = Dpi(SetThreadDpiAwarenessContext(
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        ));
        let class: Vec<u16> = "DesktopMeterSelection\0".encode_utf16().collect();
        static REGISTERED: OnceLock<bool> = OnceLock::new();
        let instance = GetModuleHandleW(std::ptr::null());
        if !*REGISTERED.get_or_init(|| {
            let wc = WNDCLASSW {
                lpfnWndProc: Some(procedure),
                hInstance: instance,
                lpszClassName: class.as_ptr(),
                hCursor: LoadCursorW(std::ptr::null_mut(), IDC_CROSS),
                ..Default::default()
            };
            RegisterClassW(&wc) != 0
        }) {
            return Err("截图选区窗口无法注册".into());
        }
        let mut state = State {
            mode,
            bounds,
            cancel,
            start: None,
            points: Vec::new(),
            selected: None,
            window: None,
            result: None,
            error: None,
            notice: None,
            created: Instant::now(),
        };
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class.as_ptr(),
            std::ptr::null(),
            WS_POPUP,
            bounds.left,
            bounds.top,
            bounds.width as i32,
            bounds.height as i32,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            (&mut state as *mut State<'_>).cast(),
        );
        if hwnd.is_null() {
            return Err("截图选区窗口无法创建".into());
        }
        let _window = Window(hwnd);
        // Color-key transparent pixels would pass clicks through to other applications.
        if SetLayeredWindowAttributes(hwnd, 0, 128, LWA_ALPHA) == 0 {
            return Err("截图透明选区无法初始化".into());
        }
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        if SetTimer(hwnd, 1, 50, None) == 0 {
            return Err("截图取消计时器无法启动".into());
        }
        let mut msg: MSG = Default::default();
        loop {
            let status = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
            if status == 0 {
                break;
            }
            if status < 0 {
                return Err("截图选区消息处理失败".into());
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if let Some(error) = state.error {
            return Err(error);
        }
        if cancel.load(Ordering::Relaxed) {
            return Ok(None);
        }
        Ok(state.result)
    }
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| dispatch(hwnd, msg, w, l)));
    match result {
        Ok(value) => value,
        Err(_) => unsafe {
            let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State<'_>;
            if !state.is_null() {
                (*state).error = Some("选区处理异常，请重新截图".into());
            }
            DestroyWindow(hwnd);
            0
        },
    }
}
fn dispatch(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_NCCREATE {
            let create = &*(l as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State<'_>;
        if pointer.is_null() {
            return DefWindowProcW(hwnd, msg, w, l);
        }
        let state = &mut *pointer;
        match msg {
            WM_ERASEBKGND => 1,
            WM_PAINT => {
                paint::show(hwnd, state);
                0
            }
            WM_TIMER => {
                if state.cancel.load(Ordering::Relaxed)
                    || state.created.elapsed() > Duration::from_secs(120)
                {
                    DestroyWindow(hwnd);
                }
                0
            }
            WM_MOUSEMOVE => {
                let point = cursor(state.bounds);
                if let Some(start) = state.start {
                    if state.mode == Mode::Freehand {
                        if state.points.last() != Some(&point) {
                            if state.points.len() < 1024 {
                                state.points.push(point);
                            } else {
                                *state.points.last_mut().unwrap() = point;
                            }
                        }
                    } else {
                        state.selected = Selection::rectangle(start, point, state.bounds);
                    }
                } else if state.mode == Mode::Window && state.window.is_none() {
                    state.selected =
                        windows::at(point, state.bounds).map(|(_, region)| Selection {
                            region,
                            polygon: vec![],
                            window: None,
                        });
                }
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_LBUTTONDOWN => {
                state.notice = None;
                let point = cursor(state.bounds);
                if state.mode == Mode::Window {
                    if let Some((handle, region)) = windows::at(point, state.bounds) {
                        state.window = Some(handle);
                        state.selected = Some(Selection {
                            region,
                            polygon: vec![],
                            window: None,
                        });
                    }
                } else {
                    state.start = Some(point);
                    state.points.clear();
                    state.selected = None;
                    if state.mode == Mode::Freehand {
                        state.points.push(point);
                    }
                    SetCapture(hwnd);
                }
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_LBUTTONUP => {
                if state.start.take().is_some() && state.mode == Mode::Freehand {
                    match Selection::freehand(std::mem::take(&mut state.points), state.bounds) {
                        Ok(selection) => state.selected = Some(selection),
                        Err(e) => {
                            state.notice = Some(e);
                            state.selected = None;
                            state.points.clear();
                        }
                    }
                }
                ReleaseCapture();
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_RBUTTONDOWN | WM_CLOSE => {
                DestroyWindow(hwnd);
                0
            }
            WM_KEYDOWN => {
                match w as u16 {
                    VK_ESCAPE => {
                        DestroyWindow(hwnd);
                    }
                    VK_RETURN => {
                        if let Some(selection) = &state.selected {
                            if let Some(handle) = state.window
                                && windows::region(handle, state.bounds) != Some(selection.region)
                            {
                                state.error = Some("窗口已经变化或关闭，请重新截图".into());
                                DestroyWindow(hwnd);
                                return 0;
                            }
                            state.result = state.selected.take().map(|mut selection| {
                                if let Some(handle) = state.window {
                                    let mut pid = 0;
                                    GetWindowThreadProcessId(handle as HWND, &mut pid);
                                    selection.window =
                                        Some(crate::domain::screenshot::WindowTarget {
                                            handle: handle as u64,
                                            pid,
                                        });
                                }
                                selection
                            });
                            DestroyWindow(hwnd);
                        }
                    }
                    VK_BACK => {
                        state.notice = None;
                        state.selected = None;
                        state.window = None;
                        state.points.clear();
                    }
                    VK_LEFT | VK_RIGHT | VK_UP | VK_DOWN if state.mode == Mode::Rectangle => {
                        if let Some(selected) = &mut state.selected {
                            adjust(
                                &mut selected.region,
                                state.bounds,
                                w as u16,
                                GetKeyState(VK_SHIFT as i32) < 0,
                            );
                        }
                    }
                    _ => {}
                }
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_DESTROY => {
                KillTimer(hwnd, 1);
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, w, l),
        }
    }
}
fn cursor(bounds: Region) -> [i32; 2] {
    let mut p: POINT = Default::default();
    unsafe {
        GetCursorPos(&mut p);
    }
    [
        p.x.clamp(
            bounds.left,
            (bounds.left as i64 + bounds.width as i64 - 1) as i32,
        ),
        p.y.clamp(
            bounds.top,
            (bounds.top as i64 + bounds.height as i64 - 1) as i32,
        ),
    ]
}
fn adjust(region: &mut Region, bounds: Region, key: u16, resize: bool) {
    let [dx, dy] = match key {
        VK_LEFT => [-1, 0],
        VK_RIGHT => [1, 0],
        VK_UP => [0, -1],
        VK_DOWN => [0, 1],
        _ => [0, 0],
    };
    if resize {
        region.width = (region.width as i64 + dx).clamp(
            1,
            bounds.left as i64 + bounds.width as i64 - region.left as i64,
        ) as u32;
        region.height = (region.height as i64 + dy).clamp(
            1,
            bounds.top as i64 + bounds.height as i64 - region.top as i64,
        ) as u32;
    } else {
        region.left = (region.left as i64 + dx).clamp(
            bounds.left as i64,
            bounds.left as i64 + bounds.width as i64 - region.width as i64,
        ) as i32;
        region.top = (region.top as i64 + dy).clamp(
            bounds.top as i64,
            bounds.top as i64 + bounds.height as i64 - region.height as i64,
        ) as i32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keyboard_adjustment_stays_inside_negative_monitor_bounds() {
        let bounds = Region {
            left: -100,
            top: -20,
            width: 100,
            height: 80,
        };
        let mut region = bounds;
        adjust(&mut region, bounds, VK_RIGHT, false);
        assert_eq!(region, bounds);
        adjust(&mut region, bounds, VK_LEFT, true);
        assert_eq!(region.width, 99);
        adjust(&mut region, bounds, VK_RIGHT, false);
        assert_eq!(region.left, -99);
        adjust(&mut region, bounds, VK_RIGHT, true);
        assert_eq!(region.width, 99);
    }
}
