use super::State;
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*};

pub fn show(hwnd: HWND, state: &State<'_>) {
    unsafe {
        let mut paint: PAINTSTRUCT = Default::default();
        let dc = BeginPaint(hwnd, &mut paint);
        let black = GetStockObject(BLACK_BRUSH);
        FillRect(dc, &paint.rcPaint, black);
        let pen = CreatePen(PS_SOLID, 2, 0x00d6dd98);
        let previous = SelectObject(dc, pen);
        let brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
        if let Some(selection) = &state.selected {
            let r = selection.region;
            if selection.polygon.is_empty() {
                Rectangle(
                    dc,
                    r.left - state.bounds.left,
                    r.top - state.bounds.top,
                    r.left - state.bounds.left + r.width as i32,
                    r.top - state.bounds.top + r.height as i32,
                );
            } else {
                path(dc, &selection.polygon, state, true);
            }
        }
        if !state.points.is_empty() {
            path(dc, &state.points, state, false);
        }
        SelectObject(dc, previous);
        DeleteObject(pen);
        SelectObject(dc, brush);
        let cursor = super::cursor(state.bounds);
        let left = (cursor[0] - state.bounds.left + 20)
            .clamp(12, (state.bounds.width as i32 - 552).max(12));
        let top = (cursor[1] - state.bounds.top + 24)
            .clamp(12, (state.bounds.height as i32 - 112).max(12));
        let panel = RECT {
            left,
            top,
            right: left + 540,
            bottom: top + 100,
        };
        let background = CreateSolidBrush(0x00241b15);
        FillRect(dc, &panel, background);
        DeleteObject(background);
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, 0x00eeeae5);
        let face: Vec<u16> = "微软雅黑\0".encode_utf16().collect();
        let font = CreateFontW(
            -17,
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            0,
            0,
            0,
            0,
            face.as_ptr(),
        );
        let old = SelectObject(dc, font);
        let suffix = state
            .selected
            .as_ref()
            .map(|s| format!("  {} × {}", s.region.width, s.region.height))
            .unwrap_or_default();
        text(
            dc,
            left + 16,
            top + 12,
            &format!("{}{}", state.mode.label(), suffix),
        );
        text(
            dc,
            left + 16,
            top + 40,
            "Enter 确认  ·  Esc / 右键取消  ·  Backspace 重新选择",
        );
        text(
            dc,
            left + 16,
            top + 68,
            state.notice.as_deref().unwrap_or(
                if state.mode == crate::domain::screenshot::Mode::Rectangle {
                    "方向键微调位置，Shift＋方向键调整大小"
                } else if state.mode == crate::domain::screenshot::Mode::Window {
                    "点击选定窗口；截取可见部分，其他窗口遮挡不会恢复"
                } else {
                    "按住左键画出轮廓，松开后预览；轮廓外透明"
                },
            ),
        );
        SelectObject(dc, old);
        DeleteObject(font);
        EndPaint(hwnd, &paint);
    }
}
unsafe fn path(dc: HDC, points: &[[i32; 2]], state: &State<'_>, closed: bool) {
    unsafe {
        if let Some(first) = points.first() {
            MoveToEx(
                dc,
                first[0] - state.bounds.left,
                first[1] - state.bounds.top,
                std::ptr::null_mut(),
            );
        }
        for p in points.iter().skip(1) {
            LineTo(dc, p[0] - state.bounds.left, p[1] - state.bounds.top);
        }
        if closed && let Some(first) = points.first() {
            LineTo(
                dc,
                first[0] - state.bounds.left,
                first[1] - state.bounds.top,
            );
        }
    }
}
unsafe fn text(dc: HDC, x: i32, y: i32, text: &str) {
    let text: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        TextOutW(dc, x, y, text.as_ptr(), text.len() as i32);
    }
}
