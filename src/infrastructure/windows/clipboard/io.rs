use super::codec;
use crate::domain::clipboard::{Content, MAX_IMAGE, MAX_TEXT, Packet};
use windows_sys::Win32::{
    Foundation::*,
    System::{DataExchange::*, Memory::*},
    UI::WindowsAndMessaging::*,
};
pub struct Clip;
impl Drop for Clip {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}
struct Lock(HGLOBAL);
struct Raw {
    format: u32,
    png_format: u32,
    bytes: Vec<u8>,
}
impl Drop for Lock {
    fn drop(&mut self) {
        unsafe {
            GlobalUnlock(self.0);
        }
    }
}
pub fn read(hwnd: HWND, max_bytes: usize) -> Result<Option<Packet>, String> {
    let Some(Raw {
        format,
        png_format,
        bytes,
    }) = read_bytes(hwnd, max_bytes)?
    else {
        return Ok(None);
    };
    if format == 13 {
        codec::text(&bytes)
    } else if format == png_format {
        codec::png(&bytes, max_bytes).map(Some)
    } else {
        codec::dib(&bytes, max_bytes).map(Some)
    }
}
fn read_bytes(hwnd: HWND, max_bytes: usize) -> Result<Option<Raw>, String> {
    unsafe {
        if OpenClipboard(hwnd) == 0 {
            return Err("剪贴板被其他程序占用，下一次复制后重试".into());
        }
        let _clip = Clip;
        let png_name: Vec<u16> = "PNG\0".encode_utf16().collect();
        let png_format = RegisterClipboardFormatW(png_name.as_ptr());
        let format = if IsClipboardFormatAvailable(13) != 0 {
            13
        } else if png_format != 0 && IsClipboardFormatAvailable(png_format) != 0 {
            png_format
        } else if IsClipboardFormatAvailable(17) != 0 {
            17
        } else if IsClipboardFormatAvailable(8) != 0 {
            8
        } else {
            return Ok(None);
        };
        let memory = GetClipboardData(format);
        if memory.is_null() {
            return Err("当前格式无法读取，请重新复制".into());
        }
        let len = GlobalSize(memory);
        let cap = if format == 13 {
            MAX_TEXT * 2 + 2
        } else {
            MAX_IMAGE.min(max_bytes) + 4096
        };
        if len == 0 || len > cap {
            return Err("当前复制内容超过单项容量，已跳过".into());
        }
        let data = GlobalLock(memory);
        if data.is_null() {
            return Err("无法锁定剪贴板内容".into());
        }
        let _lock = Lock(memory);
        let bytes = std::slice::from_raw_parts(data.cast::<u8>(), len);
        Ok(Some(Raw {
            format,
            png_format,
            bytes: bytes.to_vec(),
        }))
    }
}
pub fn copy(content: &Content) -> Result<(), String> {
    match content {
        Content::Picture(image) => crate::infrastructure::windows::screenshot::copy(image),
        Content::Text(text) => copy_text(text),
    }
}
fn copy_text(text: &str) -> Result<(), String> {
    let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2);
        if memory.is_null() {
            return Err("无法创建文字剪贴板缓冲".into());
        }
        let owned = Owned(memory);
        let data = GlobalLock(memory).cast::<u16>();
        if data.is_null() {
            return Err("无法锁定文字缓冲".into());
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), data, wide.len());
        GlobalUnlock(memory);
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            std::ptr::null(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null(),
        );
        if hwnd.is_null() {
            return Err("无法创建剪贴板所有者".into());
        }
        let _owner = Owner(hwnd);
        if OpenClipboard(hwnd) == 0 {
            return Err("剪贴板被占用，请重试".into());
        }
        let _clip = Clip;
        if EmptyClipboard() == 0 || SetClipboardData(13, memory).is_null() {
            return Err("文字写入失败，请重试".into());
        }
        std::mem::forget(owned);
        Ok(())
    }
}
struct Owned(HGLOBAL);
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe {
            GlobalFree(self.0);
        }
    }
}
struct Owner(HWND);
impl Drop for Owner {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0);
        }
    }
}
