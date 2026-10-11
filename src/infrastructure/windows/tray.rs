use super::native::wide;
use crate::interfaces::DesktopAction as Action;
use crate::interfaces::Wake;
use std::{sync::mpsc, thread};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey,
};
use windows_sys::Win32::{
    Foundation::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Shell::*, WindowsAndMessaging::*},
};

struct State {
    tx: mpsc::Sender<Action>,
    wake: Wake,
    restart: u32,
    hotkey: bool,
    icon: HICON,
}
pub struct Tray {
    pub rx: mpsc::Receiver<Action>,
    hwnd: isize,
    thread: Option<thread::JoinHandle<()>>,
}
impl Tray {
    pub fn set_clean_hotkey(&self, enabled: bool) -> Result<(), String> {
        if unsafe { SendMessageW(self.hwnd as HWND, WM_APP + 3, enabled as usize, 0) } == 0
            && enabled
        {
            Err("Ctrl+Alt+C 已被其他程序占用，未启用清理快捷键".into())
        } else {
            Ok(())
        }
    }
    pub fn start(wake: Wake) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let thread = thread::spawn(move || unsafe {
            let class = wide("DesktopMeterTray");
            let instance = GetModuleHandleW(std::ptr::null());
            let wc = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance,
                lpszClassName: class.as_ptr(),
                ..Default::default()
            };
            RegisterClassW(&wc);
            let png = include_bytes!(concat!(env!("OUT_DIR"), "/icon-32.png"));
            let icon = CreateIconFromResourceEx(
                png.as_ptr(),
                png.len() as u32,
                1,
                0x30000,
                32,
                32,
                LR_DEFAULTCOLOR,
            );
            let state = Box::new(State {
                tx,
                wake,
                restart: RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()),
                hotkey: false,
                icon,
            });
            let ptr = Box::into_raw(state);
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                wide("DesktopMeter tray").as_ptr(),
                0,
                0,
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                ptr.cast(),
            );
            if hwnd.is_null() {
                DestroyIcon((*ptr).icon);
                drop(Box::from_raw(ptr));
                let _ = ready_tx.send(Err("托盘窗口创建失败".to_string()));
                return;
            }
            if !add_icon(hwnd, (*ptr).icon) {
                DestroyWindow(hwnd);
                let _ = ready_tx.send(Err("托盘图标创建失败，已禁用锁定".to_string()));
                return;
            }
            (*ptr).hotkey =
                RegisterHotKey(hwnd, 1, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, b'M' as u32) != 0;
            let _ = ready_tx.send(Ok(hwnd as isize));
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        });
        match ready_rx.recv().map_err(|e| e.to_string())? {
            Ok(hwnd) => Ok(Self {
                rx,
                hwnd,
                thread: Some(thread),
            }),
            Err(error) => {
                let _ = thread.join();
                Err(error)
            }
        }
    }
}
impl Drop for Tray {
    fn drop(&mut self) {
        unsafe {
            PostMessageW(self.hwnd as HWND, WM_CANCELMODE, 0, 0);
            PostMessageW(self.hwnd as HWND, WM_CLOSE, 0, 0);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
unsafe fn add_icon(hwnd: HWND, brand: HICON) -> bool {
    unsafe {
        let mut icon = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_APP + 1,
            hIcon: if brand.is_null() {
                LoadIconW(std::ptr::null_mut(), IDI_INFORMATION)
            } else {
                brand
            },
            ..Default::default()
        };
        let tip = wide("资源监视器 · 单击恢复 / Ctrl+Alt+M / 右键设置");
        icon.szTip[..tip.len()].copy_from_slice(&tip);
        Shell_NotifyIconW(NIM_ADD, &icon) != 0
    }
}
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if let Some(result) = super::menus::handle(hwnd, msg, lparam) {
            return result;
        }
        if msg == WM_NCCREATE {
            let create = &*(lparam as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;
        if !ptr.is_null() {
            let state = &*ptr;
            if msg == WM_APP + 3 {
                UnregisterHotKey(hwnd, 2);
                return if wparam != 0 {
                    RegisterHotKey(hwnd, 2, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, b'C' as u32)
                        as isize
                } else {
                    1
                };
            }
            if state.restart != 0 && msg == state.restart {
                add_icon(hwnd, state.icon);
                let _ = state.tx.send(Action::Unlock);
                (state.wake)();
                return 0;
            }
            let action = match msg {
                WM_HOTKEY if wparam == 1 => Some(Action::Unlock),
                WM_HOTKEY if wparam == 2 => Some(Action::Cleanup),
                m if m == WM_APP + 2 => Some(Action::Unlock),
                m if m == WM_APP + 4 => Some(Action::Settings),
                m if m == WM_APP + 5 => Some(Action::Exit),
                m if m == WM_APP + 1 => match lparam as u32 {
                    WM_LBUTTONUP | WM_LBUTTONDBLCLK => Some(Action::Unlock),
                    WM_RBUTTONUP | WM_CONTEXTMENU => {
                        let selected = super::menus::popup(
                            hwnd,
                            &[
                                (1, "恢复完整视图", super::menus::Glyph::Restore, true),
                                (2, "设置", super::menus::Glyph::Settings, true),
                                (5, "切换完整 / 精简视图", super::menus::Glyph::Compact, true),
                                (7, "切换桌面宠物", super::menus::Glyph::Pet, true),
                                (6, "快速清理", super::menus::Glyph::Memory, true),
                                (
                                    4,
                                    "鼠标穿透 · Ctrl+Alt+M 恢复",
                                    super::menus::Glyph::Cursor,
                                    state.hotkey,
                                ),
                                (3, "退出", super::menus::Glyph::Exit, true),
                            ],
                        );
                        match selected {
                            1 => Some(Action::Unlock),
                            2 => Some(Action::Settings),
                            6 => Some(Action::Cleanup),
                            3 => Some(Action::Exit),
                            4 if state.hotkey => Some(Action::Passthrough),
                            5 => Some(Action::ToggleMode),
                            7 => Some(Action::Pet),
                            _ => None,
                        }
                    }
                    _ => None,
                },
                _ => None,
            };
            if let Some(action) = action {
                let _ = state.tx.send(action);
                (state.wake)();
                return 0;
            }
        }
        if msg == WM_CLOSE {
            DestroyWindow(hwnd);
            return 0;
        }
        if msg == WM_DESTROY {
            UnregisterHotKey(hwnd, 1);
            UnregisterHotKey(hwnd, 2);
            let icon = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 1,
                ..Default::default()
            };
            Shell_NotifyIconW(NIM_DELETE, &icon);
            PostQuitMessage(0);
            return 0;
        }
        if msg == WM_NCDESTROY && !ptr.is_null() {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DestroyIcon((*ptr).icon);
            drop(Box::from_raw(ptr));
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}
