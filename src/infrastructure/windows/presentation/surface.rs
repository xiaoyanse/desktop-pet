use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};

const MAX_BYTES: usize = 32 * 1024 * 1024;

pub(super) struct Surface {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *mut u8,
    pub size: [usize; 2],
    len: usize,
}

impl Surface {
    pub fn new(size: [usize; 2]) -> Result<Self, String> {
        let len = size[0]
            .checked_mul(size[1])
            .and_then(|n| n.checked_mul(4))
            .filter(|n| *n > 0 && *n <= MAX_BYTES)
            .ok_or("原生软件实验：像素缓冲超过32 MiB或尺寸非法")?;
        let width = i32::try_from(size[0]).map_err(|e| e.to_string())?;
        let height = i32::try_from(size[1]).map_err(|e| e.to_string())?;
        unsafe {
            let dc = CreateCompatibleDC(std::ptr::null_mut());
            if dc.is_null() {
                return Err("原生软件实验：DC创建失败".into());
            }
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
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
                return Err("原生软件实验：DIB创建失败".into());
            }
            let previous = SelectObject(dc, bitmap);
            if previous.is_null() || previous as isize == GDI_ERROR as isize {
                DeleteObject(bitmap);
                DeleteDC(dc);
                return Err("原生软件实验：DIB选择失败".into());
            }
            Ok(Self {
                dc,
                bitmap,
                previous,
                bits: bits.cast(),
                size,
                len,
            })
        }
    }

    pub fn pixels(&mut self) -> &mut [u8] {
        // The selected top-down 32-bit DIB owns exactly len writable bytes until Drop.
        unsafe { std::slice::from_raw_parts_mut(self.bits, self.len) }
    }

    pub fn present(&self, hwnd: HWND) -> Result<(), String> {
        let size = SIZE {
            cx: self.size[0] as i32,
            cy: self.size[1] as i32,
        };
        let source = POINT::default();
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        if unsafe {
            UpdateLayeredWindow(
                hwnd,
                std::ptr::null_mut(),
                std::ptr::null(),
                &size,
                self.dc,
                &source,
                0,
                &blend,
                ULW_ALPHA,
            )
        } == 0
        {
            return Err(format!("原生软件实验：透明呈现失败 {}", unsafe {
                GetLastError()
            }));
        }
        Ok(())
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_dib_is_top_down_and_rejects_overflow_before_allocation() {
        assert!(Surface::new([usize::MAX, 2]).is_err());
        assert!(Surface::new([0, 12]).is_err());
        assert!(Surface::new([10_000, 10_000]).is_err());
        let mut surface = Surface::new([2, 2]).unwrap();
        surface
            .pixels()
            .copy_from_slice(&[3, 2, 1, 255, 0, 0, 0, 0, 6, 5, 4, 255, 0, 0, 0, 0]);
        assert_eq!(unsafe { GetPixel(surface.dc, 0, 0) }, 1 | 2 << 8 | 3 << 16);
        assert_eq!(unsafe { GetPixel(surface.dc, 0, 1) }, 4 | 5 << 8 | 6 << 16);
    }
}
