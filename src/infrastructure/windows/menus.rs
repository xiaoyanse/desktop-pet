use super::native::wide;
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
    fn segments(self) -> &'static [(i32, i32, i32, i32)] {
        match self {
            Self::Restore => &[
                (1, 2, 15, 2),
                (15, 2, 15, 14),
                (15, 14, 1, 14),
                (1, 14, 1, 2),
                (1, 6, 15, 6),
                (4, 10, 12, 10),
            ],
            Self::Settings => &[
                (1, 4, 15, 4),
                (1, 8, 15, 8),
                (1, 12, 15, 12),
                (5, 2, 5, 6),
                (11, 6, 11, 10),
                (7, 10, 7, 14),
            ],
            Self::Compact => &[
                (1, 5, 15, 5),
                (15, 5, 15, 11),
                (15, 11, 1, 11),
                (1, 11, 1, 5),
                (5, 7, 5, 9),
                (11, 7, 11, 9),
            ],
            Self::Memory => &[
                (4, 4, 12, 4),
                (12, 4, 12, 12),
                (12, 12, 4, 12),
                (4, 12, 4, 4),
                (1, 6, 4, 6),
                (1, 10, 4, 10),
                (12, 6, 15, 6),
                (12, 10, 15, 10),
                (6, 1, 6, 4),
                (10, 1, 10, 4),
                (6, 12, 6, 15),
                (10, 12, 10, 15),
            ],
            Self::Cursor => &[
                (3, 1, 3, 13),
                (3, 13, 7, 9),
                (7, 9, 11, 15),
                (11, 15, 13, 14),
                (13, 14, 9, 8),
                (9, 8, 15, 8),
                (15, 8, 3, 1),
            ],
            Self::Exit => &[(2, 2, 14, 14), (14, 2, 2, 14)],
            Self::Background => &[
                (3, 3, 11, 3),
                (11, 3, 11, 7),
                (11, 7, 3, 7),
                (3, 7, 3, 3),
                (11, 5, 14, 5),
                (14, 5, 14, 10),
                (14, 10, 8, 10),
                (8, 10, 8, 15),
            ],
            Self::Border => &[
                (1, 1, 6, 1),
                (1, 1, 1, 6),
                (10, 1, 15, 1),
                (15, 1, 15, 6),
                (15, 10, 15, 15),
                (15, 15, 10, 15),
                (1, 10, 1, 15),
                (1, 15, 6, 15),
            ],
            Self::Pet => &[
                (2, 7, 2, 1),
                (2, 1, 6, 5),
                (6, 5, 10, 5),
                (10, 5, 14, 1),
                (14, 1, 14, 10),
                (14, 10, 11, 14),
                (11, 14, 5, 14),
                (5, 14, 2, 10),
                (2, 10, 2, 7),
                (5, 8, 5, 10),
                (11, 8, 11, 10),
            ],
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
        let pen = CreatePen(
            PS_SOLID,
            scale.ceil() as i32,
            if disabled {
                rgb(90, 106, 119)
            } else {
                rgb(100, 210, 195)
            },
        );
        SelectObject(item.hDC, pen);
        let x = item.rcItem.left + (14.0 * scale) as i32;
        let y = (item.rcItem.top + item.rcItem.bottom) / 2 - (8.0 * scale) as i32;
        for (x1, y1, x2, y2) in glyph.segments() {
            MoveToEx(
                item.hDC,
                x + (*x1 as f32 * scale) as i32,
                y + (*y1 as f32 * scale) as i32,
                std::ptr::null_mut(),
            );
            LineTo(
                item.hDC,
                x + (*x2 as f32 * scale) as i32,
                y + (*y2 as f32 * scale) as i32,
            );
        }
        RestoreDC(item.hDC, saved);
        DeleteObject(pen);
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
                assert_ne!(glyph.segments(), other.segments());
            }
        }
    }
}
