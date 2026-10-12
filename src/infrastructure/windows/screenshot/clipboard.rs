use crate::domain::screenshot::Image;
use windows_sys::Win32::{
    Foundation::*,
    System::{DataExchange::*, Memory::*},
    UI::WindowsAndMessaging::*,
};

fn encode_into(image: &Image, bytes: &mut [u8]) {
    let [width, height] = image.size();
    bytes[..124].fill(0);
    for (offset, value) in [
        (0, 124u32),
        (4, width),
        (8, (-(height as i32)) as u32),
        (16, 3),
        (20, image.pixels().len() as u32),
        (40, 0xff0000),
        (44, 0xff00),
        (48, 0xff),
        (52, 0xff000000),
        (56, 0x73524742),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes[12..14].copy_from_slice(&1u16.to_le_bytes());
    bytes[14..16].copy_from_slice(&32u16.to_le_bytes());
    for (out, pixel) in bytes[124..]
        .chunks_exact_mut(4)
        .zip(image.pixels().chunks_exact(4))
    {
        out.copy_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
    }
}
pub fn copy(image: &Image) -> Result<(), String> {
    let len = image
        .pixels()
        .len()
        .checked_add(124)
        .ok_or("图片容量溢出")?;
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, len);
        if memory.is_null() {
            return Err("剪贴板缓冲无法创建".into());
        }
        let guard = OwnedMemory(memory);
        let data = GlobalLock(memory).cast::<u8>();
        if data.is_null() {
            return Err("剪贴板缓冲无法锁定".into());
        }
        encode_into(image, std::slice::from_raw_parts_mut(data, len));
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
            return Err("剪贴板正被其他程序使用，请重试".into());
        }
        let _clipboard = Clipboard;
        if EmptyClipboard() == 0 || SetClipboardData(17, memory).is_null() {
            return Err("图片写入剪贴板失败，请重试".into());
        }
        std::mem::forget(guard);
        Ok(())
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
struct OwnedMemory(HGLOBAL);
impl Drop for OwnedMemory {
    fn drop(&mut self) {
        unsafe {
            GlobalFree(self.0);
        }
    }
}
struct Clipboard;
impl Drop for Clipboard {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clipboard_dib_v5_preserves_top_down_pixels_and_alpha_without_touching_clipboard() {
        let image = Image::new([2, 1], vec![230, 20, 10, 255, 40, 50, 60, 0]).unwrap();
        let mut bytes = vec![0; 132];
        encode_into(&image, &mut bytes);
        assert_eq!(&bytes[..4], &124u32.to_le_bytes());
        assert_eq!(&bytes[8..12], &(-1i32).to_le_bytes());
        assert_eq!(&bytes[52..56], &0xff000000u32.to_le_bytes());
        assert_eq!(&bytes[124..], &[10, 20, 230, 255, 60, 50, 40, 0]);
    }
}
