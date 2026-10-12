use crate::infrastructure::windows::native::wide;
use std::cell::RefCell;
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    UI::{
        Controls::{DRAWITEMSTRUCT, MEASUREITEMSTRUCT, ODS_DISABLED, ODS_SELECTED, ODT_MENU},
        HiDpi::GetDpiForWindow,
        WindowsAndMessaging::*,
    },
};

#[derive(Clone, Copy, Debug)]
pub enum Glyph {
    Restore,
    Settings,
    Compact,
    Memory,
    Cursor,
    Exit,
    Background,
    Border,
    Pet,
}
impl Glyph {
    fn asset(self) -> &'static str {
        match self {
            Self::Restore => "restore",
            Self::Settings => "settings",
            Self::Compact => "compact",
            Self::Memory => "nav-clean",
            Self::Cursor => "unpin",
            Self::Exit => "close",
            Self::Background => "nav-monitor",
            Self::Border => "maximize",
            Self::Pet => "nav-appearance",
        }
    }
}
thread_local! {static ACTIVE:RefCell<Vec<(u32,String,Glyph)>>=const{RefCell::new(Vec::new())};}
fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r | (g << 8) | (b << 16)
}
pub fn popup(hwnd: HWND, items: &[(u32, &str, Glyph, bool)]) -> i32 {
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return 0;
        }
        ACTIVE.with(|active| {
            *active.borrow_mut() = items
                .iter()
                .map(|(id, text, glyph, _)| (*id, (*text).into(), *glyph))
                .collect()
        });
        let background = CreateSolidBrush(rgb(22, 31, 41));
        let info = MENUINFO {
            cbSize: std::mem::size_of::<MENUINFO>() as u32,
            fMask: MIM_BACKGROUND,
            hbrBack: background,
            ..Default::default()
        };
        SetMenuInfo(menu, &info);
        for (id, _, _, enabled) in items {
            AppendMenuW(
                menu,
                MF_OWNERDRAW | if *enabled { 0 } else { MF_GRAYED },
                *id as usize,
                std::ptr::null(),
            );
        }
        let mut point = POINT::default();
        GetCursorPos(&mut point);
        SetForegroundWindow(hwnd);
        let selected = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY,
            point.x,
            point.y,
            0,
            hwnd,
            std::ptr::null(),
        );
        DestroyMenu(menu);
        DeleteObject(background);
        ACTIVE.with(|a| a.borrow_mut().clear());
        super::native_artwork::clear();
        PostMessageW(hwnd, WM_NULL, 0, 0);
        selected
    }
}
pub unsafe fn handle(hwnd: HWND, msg: u32, l: LPARAM) -> Option<LRESULT> {
    if l == 0 || !matches!(msg, WM_DRAWITEM | WM_MEASUREITEM) {
        return None;
    }
    let scale = unsafe { GetDpiForWindow(hwnd).max(96) } as f32 / 96.0;
    unsafe {
        if msg == WM_MEASUREITEM {
            let item = &mut *(l as *mut MEASUREITEMSTRUCT);
            if item.CtlType != ODT_MENU
                || !ACTIVE.with(|a| a.borrow().iter().any(|(id, _, _)| *id == item.itemID))
            {
                return None;
            }
            item.itemWidth = (292.0 * scale) as u32;
            item.itemHeight = (38.0 * scale) as u32;
            return Some(1);
        }
        let item = &*(l as *const DRAWITEMSTRUCT);
        if item.CtlType != ODT_MENU {
            return None;
        }
        let (text, glyph) = ACTIVE.with(|a| {
            a.borrow()
                .iter()
                .find(|(id, _, _)| *id == item.itemID)
                .map(|(_, s, glyph)| (s.clone(), *glyph))
        })?;
        let saved = SaveDC(item.hDC);
        let selected = item.itemState & ODS_SELECTED != 0;
        let disabled = item.itemState & ODS_DISABLED != 0;
        let brush = CreateSolidBrush(if selected {
            rgb(35, 66, 72)
        } else {
            rgb(22, 31, 41)
        });
        FillRect(item.hDC, &item.rcItem, brush);
        DeleteObject(brush);
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(
            item.hDC,
            if disabled {
                rgb(90, 106, 119)
            } else {
                rgb(218, 232, 237)
            },
        );
        let font = CreateFontW(
            -(14.0 * scale) as i32,
            0,
            0,
            0,
            FW_NORMAL as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            0,
            0,
            CLEARTYPE_QUALITY as u32,
            0,
            wide("Microsoft YaHei UI").as_ptr(),
        );
        if !font.is_null() {
            SelectObject(item.hDC, font);
        }
        let mut rect = item.rcItem;
        rect.left += (44.0 * scale) as i32;
        let text = wide(text);
        DrawTextW(
            item.hDC,
            text.as_ptr(),
            (text.len() - 1) as i32,
            &mut rect,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        );
        let size = (28.0 * scale) as i32;
        super::native_artwork::paint(
            item.hDC,
            [
                item.rcItem.left + (10.0 * scale) as i32,
                (item.rcItem.top + item.rcItem.bottom - size) / 2,
                size,
                size,
            ],
            glyph.asset(),
            disabled,
        );
        RestoreDC(item.hDC, saved);
        if !font.is_null() {
            DeleteObject(font);
        }
        Some(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn menu_actions_have_distinct_glyphs() {
        let glyphs = [
            Glyph::Restore,
            Glyph::Settings,
            Glyph::Compact,
            Glyph::Memory,
            Glyph::Cursor,
            Glyph::Exit,
            Glyph::Background,
            Glyph::Border,
            Glyph::Pet,
        ];
        for (i, glyph) in glyphs.iter().enumerate() {
            for other in &glyphs[i + 1..] {
                assert_ne!(
                    crate::assets::get(glyph.asset()).rgba,
                    crate::assets::get(other.asset()).rgba
                );
            }
        }
    }
}
