use super::drag_surface::DragSurface;
use super::{docking::Docking, native, tooltip::Tooltip, tray::Tray};
use crate::interfaces::{DesktopAction, DesktopSession};

pub struct WindowsDesktop {
    hwnd: isize,
    docking: Option<Docking>,
    tooltip: Option<Tooltip>,
    tray: Option<Tray>,
    warning: Option<String>,
    drag: Option<DragSurface>,
}
impl WindowsDesktop {
    pub fn attach(hwnd: isize, snapping: bool, wake: crate::interfaces::Wake) -> Self {
        if hwnd != 0 {
            super::shell_surface::attach(hwnd as _);
            native::safe_position(hwnd as _);
        }
        let (tray, warning) = match Tray::start(wake.clone()) {
            Ok(tray) => (Some(tray), None),
            Err(error) => (
                None,
                Some(format!("托盘启动失败：{error}；可使用悬浮工具栏操作")),
            ),
        };
        Self {
            hwnd,
            docking: (hwnd != 0)
                .then(|| Docking::attach(hwnd as _, snapping))
                .flatten(),
            tooltip: (hwnd != 0).then(|| Tooltip::new(hwnd as _)).flatten(),
            tray,
            warning,
            drag: (hwnd != 0)
                .then(|| DragSurface::attach(hwnd as _, wake.clone()))
                .flatten(),
        }
    }
}
impl DesktopSession for WindowsDesktop {
    fn configure_auxiliary(&self) {
        super::shell_surface::auxiliary();
    }
    fn cursor(&self) -> Option<[f32; 2]> {
        unsafe {
            use windows_sys::Win32::{
                Foundation::POINT,
                Graphics::Gdi::ScreenToClient,
                UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::GetCursorPos},
            };
            let mut point = POINT::default();
            let hwnd = self.hwnd as _;
            if GetCursorPos(&mut point) == 0 || ScreenToClient(hwnd, &mut point) == 0 {
                return None;
            }
            let scale = GetDpiForWindow(hwnd).max(96) as f32 / 96.0;
            Some([point.x as f32 / scale, point.y as f32 / scale])
        }
    }
    fn configure_card(&self, compact: bool, draggable: bool, drag_height: f32, toolbar_width: f32) {
        if let Some(drag) = &self.drag {
            drag.configure(compact, draggable, drag_height, toolbar_width);
        }
    }
    fn geometry(&self) -> Option<crate::domain::desktop::DesktopGeometry> {
        unsafe {
            use windows_sys::Win32::{
                Foundation::RECT,
                Graphics::Gdi::*,
                UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::GetWindowRect},
            };
            let hwnd = self.hwnd as _;
            let mut rect = RECT::default();
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetWindowRect(hwnd, &mut rect) == 0
                || GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut info)
                    == 0
            {
                return None;
            }
            let scale = GetDpiForWindow(hwnd).max(96) as f32 / 96.0;
            let values = |r: RECT| {
                [
                    r.left as f32 / scale,
                    r.top as f32 / scale,
                    r.right as f32 / scale,
                    r.bottom as f32 / scale,
                ]
            };
            Some(crate::domain::desktop::DesktopGeometry {
                frame: values(rect),
                work_area: values(info.rcWork),
            })
        }
    }
    fn warning(&self) -> Option<String> {
        self.warning.clone()
    }
    fn actions(&self) -> Vec<DesktopAction> {
        let mut actions: Vec<DesktopAction> = self
            .tray
            .as_ref()
            .map(|t| t.rx.try_iter().collect())
            .unwrap_or_default();
        if let Some(drag) = &self.drag {
            actions.extend(drag.actions());
        }
        actions
    }
    fn set_snapping(&self, enabled: bool) {
        if let Some(d) = &self.docking {
            d.set_enabled(enabled);
        }
    }
    fn tooltip(&mut self, text: &str, active: bool) {
        if let Some(t) = &mut self.tooltip {
            if !text.is_empty() {
                t.update(text);
            }
            t.activate(active);
        }
    }
    fn menu(&self, compact: bool) -> i32 {
        native::card_menu(self.hwnd as _, compact)
    }
    fn autostart(&self) -> bool {
        native::autostart()
    }
    fn set_autostart(&self, enable: bool) -> Result<(), String> {
        native::set_autostart(enable)
    }
    fn set_clean_hotkey(&self, enabled: bool) -> Result<(), String> {
        self.tray
            .as_ref()
            .ok_or("托盘未初始化")?
            .set_clean_hotkey(enabled)
    }
}
