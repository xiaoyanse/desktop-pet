use super::surface::Surface;
use crate::infrastructure::windows::native::wide;
use std::{
    cell::RefCell,
    panic::{AssertUnwindSafe, catch_unwind},
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Controls::WM_MOUSELEAVE, HiDpi::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

#[derive(Clone, Copy)]
pub enum PointerEvent {
    Move([f32; 2]),
    Button([f32; 2], bool),
    Leave,
}

pub struct FrameInput {
    pub size: [usize; 2],
    pub scale: f32,
    pub events: Vec<PointerEvent>,
    pub maximized: bool,
}

#[derive(Default)]
pub struct FrameAction {
    pub maximize: Option<bool>,
    pub close: bool,
    pub open_settings: bool,
    pub drag: bool,
    pub repaint_after: Option<std::time::Duration>,
}

pub type Draw = Box<dyn FnMut(&mut [u8], FrameInput) -> Result<FrameAction, String>>;

struct State {
    surface: Option<Surface>,
    events: Vec<PointerEvent>,
    draw: Draw,
    root: bool,
    settings: HWND,
    failure: Option<String>,
}

impl State {
    fn paint(&mut self, hwnd: HWND) -> Result<FrameAction, String> {
        let mut rect = RECT::default();
        if unsafe { GetClientRect(hwnd, &mut rect) } == 0 {
            return Err("原生软件实验：读取窗口尺寸失败".into());
        }
        let size = [
            (rect.right - rect.left).max(0) as usize,
            (rect.bottom - rect.top).max(0) as usize,
        ];
        if size.contains(&0) || unsafe { IsIconic(hwnd) } != 0 {
            return Ok(FrameAction::default());
        }
        if self.surface.as_ref().is_none_or(|s| s.size != size) {
            // Release the previous allocation before allocating the new size, including restore.
            self.surface = None;
            self.surface = Some(Surface::new(size)?);
        }
        let input = FrameInput {
            size,
            scale: unsafe { GetDpiForWindow(hwnd) }.max(96) as f32 / 96.0,
            events: std::mem::take(&mut self.events),
            maximized: unsafe { IsZoomed(hwnd) } != 0,
        };
        let surface = self.surface.as_mut().expect("当前尺寸缓冲已创建");
        let action = (self.draw)(surface.pixels(), input)?;
        surface.present(hwnd)?;
        Ok(action)
    }
}

/// Own-process-only developer host. The two state allocations outlive all native callbacks.
pub fn run(root_draw: Draw, settings_draw: Draw) -> Result<(), String> {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let class = wide("DesktopMeterSoftwareExperiment");
        let instance = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            return Err(format!("原生软件实验：注册失败 {}", GetLastError()));
        }
        let states = [
            RefCell::new(State {
                surface: None,
                events: Vec::new(),
                draw: root_draw,
                root: true,
                settings: std::ptr::null_mut(),
                failure: None,
            }),
            RefCell::new(State {
                surface: None,
                events: Vec::new(),
                draw: settings_draw,
                root: false,
                settings: std::ptr::null_mut(),
                failure: None,
            }),
        ];
        let mut windows = [std::ptr::null_mut(); 2];
        let mut result = Ok(());
        for index in 0..2 {
            let title = wide(if index == 0 {
                "软件绘制实验 · 透明窗口"
            } else {
                "软件绘制实验 · 设置"
            });
            let (x, y, w, h) = if index == 0 {
                (1120, 560, 240, 142)
            } else {
                (80, 80, 1040, 820)
            };
            windows[index] = CreateWindowExW(
                // Developer surfaces stay directly discoverable for real input/close verification.
                // Production floating windows retain their existing tool-window policy.
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_APPWINDOW,
                class.as_ptr(),
                title.as_ptr(),
                WS_POPUP | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX,
                x,
                y,
                w,
                h,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                (&states[index] as *const RefCell<State>).cast(),
            );
            if windows[index].is_null() {
                result = Err(format!("原生软件实验：窗口创建失败 {}", GetLastError()));
                break;
            }
        }
        if result.is_ok() {
            states[0].borrow_mut().settings = windows[1];
            for hwnd in windows {
                ShowWindow(hwnd, SW_SHOWNORMAL);
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
            let mut msg = MSG::default();
            loop {
                let status = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
                if status <= 0 {
                    if status < 0 {
                        result = Err(format!("原生软件实验：消息循环失败 {}", GetLastError()));
                    } else if msg.wParam != 0 {
                        result = Err("原生软件实验：绘制回调或消息处理失败".into());
                    }
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        for hwnd in windows {
            if !hwnd.is_null() && IsWindow(hwnd) != 0 {
                DestroyWindow(hwnd);
            }
        }
        for state in states {
            if let Some(error) = state.into_inner().failure {
                result = Err(error);
            }
        }
        UnregisterClassW(class.as_ptr(), instance);
        result
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match catch_unwind(AssertUnwindSafe(|| dispatch(hwnd, msg, w, l))) {
        Ok(result) => result,
        Err(_) => {
            unsafe {
                PostQuitMessage(1);
            }
            0
        }
    }
}

fn dispatch(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_NCCREATE {
            let create = &*(l as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
            return 1;
        }
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<State>;
        if ptr.is_null() {
            return DefWindowProcW(hwnd, msg, w, l);
        }
        let cell = &*ptr;
        match msg {
            WM_NCDESTROY => {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            WM_ERASEBKGND => return 1,
            WM_PAINT => {
                let mut paint = PAINTSTRUCT::default();
                BeginPaint(hwnd, &mut paint);
                let action = if let Ok(mut state) = cell.try_borrow_mut() {
                    match state.paint(hwnd) {
                        Ok(action) => Some(action),
                        Err(error) => {
                            state.failure = Some(error);
                            PostQuitMessage(1);
                            None
                        }
                    }
                } else {
                    None
                };
                EndPaint(hwnd, &paint);
                if let Some(action) = action {
                    apply_action(hwnd, cell, action);
                }
                return 0;
            }
            WM_SIZE => {
                if w == SIZE_MINIMIZED as usize
                    && let Ok(mut state) = cell.try_borrow_mut()
                {
                    state.surface = None;
                }
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
            WM_TIMER => {
                KillTimer(hwnd, 1);
                InvalidateRect(hwnd, std::ptr::null(), 0);
                return 0;
            }
            WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_MOUSELEAVE => {
                let point = [
                    (l as u16 as i16) as f32,
                    ((l as usize >> 16) as u16 as i16) as f32,
                ];
                let event = match msg {
                    WM_LBUTTONDOWN => {
                        SetCapture(hwnd);
                        PointerEvent::Button(point, true)
                    }
                    WM_LBUTTONUP => {
                        ReleaseCapture();
                        PointerEvent::Button(point, false)
                    }
                    WM_MOUSELEAVE => PointerEvent::Leave,
                    _ => {
                        let mut tracking = TRACKMOUSEEVENT {
                            cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                            dwFlags: TME_LEAVE,
                            hwndTrack: hwnd,
                            dwHoverTime: 0,
                        };
                        TrackMouseEvent(&mut tracking);
                        PointerEvent::Move(point)
                    }
                };
                if let Ok(mut state) = cell.try_borrow_mut() {
                    if state.events.len() < 128 {
                        state.events.push(event);
                    } else {
                        state.failure =
                            Some("原生软件实验：输入队列超过容量，拒绝丢弃按钮释放事件".into());
                        PostQuitMessage(1);
                    }
                }
                InvalidateRect(hwnd, std::ptr::null(), 0);
                return 0;
            }
            WM_CLOSE => {
                let root = cell.borrow().root;
                if root {
                    DestroyWindow(hwnd);
                    PostQuitMessage(0);
                } else {
                    KillTimer(hwnd, 1);
                    ShowWindow(hwnd, SW_HIDE);
                    cell.borrow_mut().surface = None;
                }
                return 0;
            }
            WM_GETMINMAXINFO => {
                let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                let mut info = MONITORINFO {
                    cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                    ..Default::default()
                };
                if GetMonitorInfoW(monitor, &mut info) != 0 {
                    let limit = &mut *(l as *mut MINMAXINFO);
                    limit.ptMaxPosition = POINT {
                        x: info.rcWork.left - info.rcMonitor.left,
                        y: info.rcWork.top - info.rcMonitor.top,
                    };
                    limit.ptMaxSize = POINT {
                        x: info.rcWork.right - info.rcWork.left,
                        y: info.rcWork.bottom - info.rcWork.top,
                    };
                }
                return 0;
            }
            _ => {}
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}

fn apply_action(hwnd: HWND, cell: &RefCell<State>, action: FrameAction) {
    unsafe {
        KillTimer(hwnd, 1);
        if let Some(delay) = action.repaint_after {
            SetTimer(
                hwnd,
                1,
                delay.as_millis().clamp(1, u32::MAX as u128) as u32,
                None,
            );
        }
        if let Some(maximize) = action.maximize {
            ShowWindow(hwnd, if maximize { SW_MAXIMIZE } else { SW_RESTORE });
        }
        if action.open_settings {
            let settings = cell.borrow().settings;
            if !settings.is_null() {
                ShowWindow(settings, SW_SHOWNORMAL);
                InvalidateRect(settings, std::ptr::null(), 0);
            }
        }
        if action.close {
            PostMessageW(hwnd, WM_CLOSE, 0, 0);
        }
        if action.drag {
            ReleaseCapture();
            SendMessageW(hwnd, WM_NCLBUTTONDOWN, HTCAPTION as usize, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc, time::Duration};

    #[test]
    fn closing_settings_keeps_the_root_message_loop_alive_until_explicit_root_close() {
        let root_frames = Rc::new(Cell::new(0));
        let settings_frames = Rc::new(Cell::new(0));
        let root_count = root_frames.clone();
        let child_seen = settings_frames.clone();
        let child_count = settings_frames.clone();
        let root: Draw = Box::new(move |pixels, _| {
            pixels.fill(0);
            root_count.set(root_count.get() + 1);
            if root_count.get() > 30 {
                return Err("设置窗口未绘制，原生生命周期测试失败".into());
            }
            Ok(FrameAction {
                // Native paint order is not guaranteed: the child may paint first.
                close: root_count.get() >= 2 && child_seen.get() > 0,
                repaint_after: Some(Duration::from_millis(10)),
                ..Default::default()
            })
        });
        let child: Draw = Box::new(move |pixels, _| {
            pixels.fill(0);
            child_count.set(child_count.get() + 1);
            Ok(FrameAction {
                close: true,
                ..Default::default()
            })
        });
        run(root, child).unwrap();
        assert!(root_frames.get() >= 2);
        assert!(settings_frames.get() >= 1);
    }
}
