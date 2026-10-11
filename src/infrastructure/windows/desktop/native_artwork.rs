use crate::assets;
use std::{cell::RefCell, collections::HashMap};
use windows_sys::Win32::Graphics::Gdi::*;

struct Bitmap {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    size: [usize; 2],
}

impl Bitmap {
    fn new(target: HDC, size: [usize; 2], rgba: &[u8]) -> Option<Self> {
        if rgba.len() != size[0].checked_mul(size[1])?.checked_mul(4)? {
            return None;
        }
        let width = i32::try_from(size[0]).ok().filter(|v| *v > 0)?;
        let height = i32::try_from(size[1]).ok().filter(|v| *v > 0)?;
        unsafe {
            let dc = CreateCompatibleDC(target);
            if dc.is_null() {
                return None;
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
                target,
                &info,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            if bitmap.is_null() || bits.is_null() {
                DeleteDC(dc);
                if !bitmap.is_null() {
                    DeleteObject(bitmap);
                }
                return None;
            }
            let pixels = std::slice::from_raw_parts_mut(bits.cast::<u8>(), rgba.len());
            for (dst, src) in pixels.chunks_exact_mut(4).zip(rgba.chunks_exact(4)) {
                let premultiply = |c: u8| ((c as u32 * src[3] as u32 + 127) / 255) as u8;
                dst.copy_from_slice(&[
                    premultiply(src[2]),
                    premultiply(src[1]),
                    premultiply(src[0]),
                    src[3],
                ]);
            }
            let previous = SelectObject(dc, bitmap);
            if previous.is_null() || previous as isize == GDI_ERROR as isize {
                DeleteDC(dc);
                DeleteObject(bitmap);
                return None;
            }
            Some(Self {
                dc,
                bitmap,
                previous,
                size,
            })
        }
    }

    fn paint(&self, target: HDC, rect: [i32; 4], disabled: bool) -> bool {
        unsafe {
            AlphaBlend(
                target,
                rect[0],
                rect[1],
                rect[2],
                rect[3],
                self.dc,
                0,
                0,
                self.size[0] as i32,
                self.size[1] as i32,
                BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: if disabled { 75 } else { 255 },
                    AlphaFormat: AC_SRC_ALPHA as u8,
                },
            ) != 0
        }
    }
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

thread_local! { static CACHE: RefCell<HashMap<&'static str,Bitmap>> = RefCell::new(HashMap::new()); }

pub fn paint(target: HDC, rect: [i32; 4], name: &'static str, disabled: bool) {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if !cache.contains_key(name) {
            let source = assets::get(name);
            if let Some(bitmap) = Bitmap::new(target, source.size, source.rgba) {
                cache.insert(name, bitmap);
            }
        }
        if let Some(bitmap) = cache.get(name) {
            bitmap.paint(target, rect, disabled);
        }
    });
}

pub fn clear() {
    CACHE.with(|cache| cache.borrow_mut().clear());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_gdi_alpha_blending_preserves_transparent_corners_and_draws_the_icon() {
        let pixels = [45, 55, 65, 255].repeat(64 * 64);
        let target = Bitmap::new(std::ptr::null_mut(), [64, 64], &pixels).unwrap();
        let source = assets::get("settings");
        let icon = Bitmap::new(target.dc, source.size, source.rgba).unwrap();
        assert!(icon.paint(target.dc, [0, 0, 64, 64], false));
        unsafe {
            assert_eq!(GetPixel(target.dc, 0, 0), 45 | (55 << 8) | (65 << 16));
            assert_ne!(GetPixel(target.dc, 32, 15), GetPixel(target.dc, 0, 0));
        }
        assert!(icon.paint(target.dc, [0, 0, 32, 32], true));
    }
}
