use crate::native::wide;
use eframe::egui::Context;
use std::{sync::mpsc, thread};
use windows_sys::Win32::{Foundation::*, System::LibraryLoader::GetModuleHandleW, UI::{WindowsAndMessaging::*, Shell::*}};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey, MOD_CONTROL, MOD_ALT, MOD_NOREPEAT};

#[derive(Clone, Copy)]
pub enum Action { Unlock, Passthrough, Settings, ToggleMode, Exit }
struct State { tx: mpsc::Sender<Action>, ctx: Context, restart: u32, hotkey: bool }
pub struct Tray { pub rx: mpsc::Receiver<Action>, hwnd: isize, thread: Option<thread::JoinHandle<()>> }
impl Tray {
    pub fn start(ctx: Context) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel(); let (ready_tx, ready_rx) = mpsc::channel();
        let thread = thread::spawn(move || unsafe {
            let class = wide("DesktopMeterTray"); let instance = GetModuleHandleW(std::ptr::null());
            let wc = WNDCLASSW { lpfnWndProc: Some(window_proc), hInstance: instance, lpszClassName: class.as_ptr(), ..Default::default() };
            RegisterClassW(&wc);
            let state = Box::new(State { tx, ctx, restart: RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()), hotkey: false });
            let ptr = Box::into_raw(state);
            let hwnd = CreateWindowExW(0, class.as_ptr(), wide("DesktopMeter tray").as_ptr(), 0, 0, 0, 0, 0,
                std::ptr::null_mut(), std::ptr::null_mut(), instance, ptr.cast());
            if hwnd.is_null() {
                drop(Box::from_raw(ptr)); let _ = ready_tx.send(Err("托盘窗口创建失败".to_string())); return;
            }
            if !add_icon(hwnd) { DestroyWindow(hwnd); let _ = ready_tx.send(Err("托盘图标创建失败，已禁用锁定".to_string())); return; }
            (*ptr).hotkey = RegisterHotKey(hwnd, 1, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, b'M' as u32) != 0;
            let _ = ready_tx.send(Ok(hwnd as isize));
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 { TranslateMessage(&msg); DispatchMessageW(&msg); }
        });
        match ready_rx.recv().map_err(|e| e.to_string())? {
            Ok(hwnd) => Ok(Self { rx, hwnd, thread: Some(thread) }),
            Err(error) => { let _ = thread.join(); Err(error) }
        }
    }
}
impl Drop for Tray {
    fn drop(&mut self) {
        unsafe {
            PostMessageW(self.hwnd as HWND, WM_CANCELMODE, 0, 0);
            PostMessageW(self.hwnd as HWND, WM_CLOSE, 0, 0);
        }
        if let Some(thread) = self.thread.take() { let _ = thread.join(); }
    }
}
unsafe fn add_icon(hwnd: HWND) -> bool {
    unsafe {
        let mut icon = NOTIFYICONDATAW { cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd,
            uID: 1, uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP, uCallbackMessage: WM_APP + 1,
            hIcon: LoadIconW(std::ptr::null_mut(), IDI_INFORMATION), ..Default::default() };
        let tip = wide("资源监视器 · 单击恢复 / Ctrl+Alt+M / 右键设置");
        icon.szTip[..tip.len()].copy_from_slice(&tip);
        Shell_NotifyIconW(NIM_ADD, &icon) != 0
    }
}
unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_NCCREATE {
            let create = &*(lparam as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State;
        if !ptr.is_null() {
            let state = &*ptr;
            if state.restart != 0 && msg == state.restart {
                add_icon(hwnd);
                let _ = state.tx.send(Action::Unlock); state.ctx.request_repaint(); return 0;
            }
            let action = match msg {
                WM_HOTKEY if wparam == 1 => Some(Action::Unlock),
                m if m == WM_APP + 2 => Some(Action::Unlock),
                m if m == WM_APP + 1 => match lparam as u32 {
                    WM_LBUTTONUP | WM_LBUTTONDBLCLK => Some(Action::Unlock),
                    WM_RBUTTONUP | WM_CONTEXTMENU => {
                        let menu = CreatePopupMenu();
                        if menu.is_null() { return 0; }
                        for (id, text) in [(1, "恢复操作 / 显示"), (2, "设置"), (3, "退出")] {
                            AppendMenuW(menu, MF_STRING, id, wide(text).as_ptr());
                        }
                        AppendMenuW(menu, if state.hotkey { MF_STRING } else { MF_STRING | MF_GRAYED }, 4, wide("鼠标穿透（Ctrl+Alt+M 恢复）").as_ptr());
                        AppendMenuW(menu, MF_STRING, 5, wide("切换正常 / 精简视图").as_ptr());
                        let mut point = POINT::default(); GetCursorPos(&mut point); SetForegroundWindow(hwnd);
                        let selected = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_NONOTIFY, point.x, point.y, 0, hwnd, std::ptr::null());
                        DestroyMenu(menu); PostMessageW(hwnd, WM_NULL, 0, 0);
                        match selected { 1 => Some(Action::Unlock), 2 => Some(Action::Settings), 3 => Some(Action::Exit), 4 if state.hotkey => Some(Action::Passthrough), 5 => Some(Action::ToggleMode), _ => None }
                    }
                    _ => None,
                },
                _ => None,
            };
            if let Some(action) = action { let _ = state.tx.send(action); state.ctx.request_repaint(); return 0; }
        }
        if msg == WM_CLOSE { DestroyWindow(hwnd); return 0; }
        if msg == WM_DESTROY {
            UnregisterHotKey(hwnd, 1);
            let icon = NOTIFYICONDATAW { cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd, uID: 1, ..Default::default() };
            Shell_NotifyIconW(NIM_DELETE, &icon); PostQuitMessage(0); return 0;
        }
        if msg == WM_NCDESTROY && !ptr.is_null() {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0); drop(Box::from_raw(ptr));
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}
