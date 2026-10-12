mod clipboard;
mod selector;
use crate::{
    domain::screenshot::{Image, Region},
    interfaces::ScreenCapture,
};
use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};

pub use clipboard::copy;
pub struct WindowsCapture;
impl ScreenCapture for WindowsCapture {
    fn validate(&self, selection: &crate::domain::screenshot::Selection) -> Result<(), String> {
        selection.region.bytes()?;
        let bounds = self.bounds()?;
        if !bounds.contains(selection.region) {
            return Err("选区已超出桌面，请重新选择".into());
        }
        selector::validate(selection, bounds)
    }
    fn select(
        &self,
        mode: crate::domain::screenshot::Mode,
        cancel: &AtomicBool,
    ) -> Result<Option<crate::domain::screenshot::Selection>, String> {
        selector::select(mode, self.bounds()?, cancel)
    }
    fn bounds(&self) -> Result<Region, String> {
        unsafe {
            let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
            if width <= 0 || height <= 0 {
                return Err("无法读取桌面工作区".into());
            }
            Ok(Region {
                left: GetSystemMetrics(SM_XVIRTUALSCREEN),
                top: GetSystemMetrics(SM_YVIRTUALSCREEN),
                width: width as u32,
                height: height as u32,
            })
        }
    }
    fn capture(&self, region: Region, cancel: &AtomicBool) -> Result<Image, String> {
        region.bytes()?;
        if !self.bounds()?.contains(region) {
            return Err("选区已超出桌面，请重新选择".into());
        }
        if cancel.load(Ordering::Relaxed) {
            return Err("截图已取消".into());
        }
        unsafe {
            windows_sys::Win32::Graphics::Dwm::DwmFlush();
        }
        let screen = ScreenDc::new()?;
        let result = capture_from(screen.0, region)?;
        if cancel.load(Ordering::Relaxed) {
            return Err("截图已取消".into());
        }
        Ok(result)
    }
}
struct ScreenDc(HDC);
impl ScreenDc {
    fn new() -> Result<Self, String> {
        let dc = unsafe { GetDC(std::ptr::null_mut()) };
        if dc.is_null() {
            Err("无法访问桌面，请检查会话是否锁定".into())
        } else {
            Ok(Self(dc))
        }
    }
}
impl Drop for ScreenDc {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(std::ptr::null_mut(), self.0);
        }
    }
}
struct Bitmap {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
}
impl Drop for Bitmap {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}

fn capture_from(source: HDC, region: Region) -> Result<Image, String> {
    let len = region.bytes()?;
    unsafe {
        let dc = CreateCompatibleDC(source);
        if dc.is_null() {
            return Err("截图缓冲无法创建".into());
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: region.width as i32,
                biHeight: -(region.height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(
            dc,
            &info,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        );
        if bitmap.is_null() || bits.is_null() {
            if !bitmap.is_null() {
                DeleteObject(bitmap);
            }
            DeleteDC(dc);
            return Err("截图像素缓冲无法创建".into());
        }
        let previous = SelectObject(dc, bitmap);
        if previous.is_null() || previous as isize == GDI_ERROR as isize {
            DeleteObject(bitmap);
            DeleteDC(dc);
            return Err("截图缓冲无法选入".into());
        }
        let buffer = Bitmap {
            dc,
            bitmap,
            previous,
        };
        if BitBlt(
            buffer.dc,
            0,
            0,
            region.width as i32,
            region.height as i32,
            source,
            region.left,
            region.top,
            SRCCOPY | CAPTUREBLT,
        ) == 0
        {
            return Err(format!(
                "截图失败（{}）；受保护内容可能无法截取",
                GetLastError()
            ));
        }
        // GDI may defer writes to a selected DIB. Flush before reading its CPU pixels.
        if GdiFlush() == 0 {
            return Err("截图缓冲同步失败".into());
        }
        let pixels = std::slice::from_raw_parts(bits.cast::<u8>(), len);
        let mut rgba = Vec::with_capacity(len);
        for p in pixels.chunks_exact(4) {
            rgba.extend_from_slice(&[p[2], p[1], p[0], 255]);
        }
        Image::new([region.width, region.height], rgba)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_request_never_creates_desktop_dc_or_returns_pixels() {
        let capture = WindowsCapture;
        let bounds = capture.bounds().unwrap();
        let result = capture.capture(
            Region {
                left: bounds.left,
                top: bounds.top,
                width: 1,
                height: 1,
            },
            &AtomicBool::new(true),
        );
        assert!(result.is_err_and(|message| message.contains("取消")));
    }
    #[test]
    fn synthetic_gdi_source_crops_top_down_and_never_reads_the_desktop() {
        unsafe {
            let dc = CreateCompatibleDC(std::ptr::null_mut());
            assert!(!dc.is_null());
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: 4,
                    biHeight: -3,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let bitmap = CreateDIBSection(
                dc,
                &info,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            assert!(!bitmap.is_null());
            let previous = SelectObject(dc, bitmap);
            let source = Bitmap {
                dc,
                bitmap,
                previous,
            };
            let p = std::slice::from_raw_parts_mut(bits.cast::<u8>(), 48);
            for (index, pixel) in p.chunks_exact_mut(4).enumerate() {
                pixel.copy_from_slice(&[index as u8, 30, 200, 0]);
            }
            let image = capture_from(
                source.dc,
                Region {
                    left: 1,
                    top: 1,
                    width: 2,
                    height: 2,
                },
            )
            .unwrap();
            assert_eq!(
                image.pixels(),
                &[
                    200, 30, 5, 255, 200, 30, 6, 255, 200, 30, 9, 255, 200, 30, 10, 255
                ]
            );
        }
    }
}
