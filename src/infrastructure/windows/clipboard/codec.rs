use crate::domain::{
    clipboard::{MAX_IMAGE, MAX_TEXT, Packet},
    screenshot::Image,
};
pub fn text(bytes: &[u8]) -> Result<Option<Packet>, String> {
    if !bytes.len().is_multiple_of(2) || bytes.len() > MAX_TEXT * 2 + 2 {
        return Err("Unicode文字长度无效或超过限制".into());
    }
    let units: Vec<_> = bytes
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .take_while(|u| *u != 0)
        .collect();
    if units.len() == bytes.len() / 2 {
        return Err("Unicode文字缺少终止符".into());
    }
    if units.is_empty() {
        return Ok(None);
    }
    let text = String::from_utf16(&units).map_err(|_| "Unicode文字无效")?;
    Packet::text(text).map(Some)
}
fn u32_at(bytes: &[u8], at: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(
        bytes
            .get(at..at + 4)
            .ok_or("图片头不完整")?
            .try_into()
            .unwrap(),
    ))
}
fn u16_at(bytes: &[u8], at: usize) -> Result<u16, String> {
    Ok(u16::from_le_bytes(
        bytes
            .get(at..at + 2)
            .ok_or("图片头不完整")?
            .try_into()
            .unwrap(),
    ))
}
fn image_bytes(width: u32, height: u32, max_bytes: usize) -> Result<usize, String> {
    if width == 0 || height == 0 || width > 16384 || height > 16384 {
        return Err("图片尺寸无效".into());
    }
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .filter(|n| *n <= MAX_IMAGE.min(max_bytes))
        .ok_or("图片超过剪贴板单项容量".into())
}
pub fn dib(bytes: &[u8], max_bytes: usize) -> Result<Packet, String> {
    let header = u32_at(bytes, 0)? as usize;
    if ![40, 52, 56, 108, 124].contains(&header) || bytes.len() < header {
        return Err("暂不支持这种DIB图片头".into());
    }
    let width = u32_at(bytes, 4)? as i32;
    let height = u32_at(bytes, 8)? as i32;
    if width <= 0 || height == 0 || height == i32::MIN || u16_at(bytes, 12)? != 1 {
        return Err("DIB图片尺寸或平面无效".into());
    }
    let w = width as u32;
    let h = height.unsigned_abs();
    let len = image_bytes(w, h, max_bytes)?;
    let bits = u16_at(bytes, 14)? as usize;
    let compression = u32_at(bytes, 16)?;
    if !(bits == 24 && compression == 0 || bits == 32 && [0, 3, 6].contains(&compression))
        || u32_at(bytes, 32)? != 0
    {
        return Err("暂不支持这种DIB位深、压缩或调色板".into());
    }
    if compression == 6 && header == 52 {
        return Err("DIB透明掩码不完整".into());
    }
    let extra = if header == 40 && compression == 3 {
        12
    } else if header == 40 && compression == 6 {
        16
    } else {
        0
    };
    let mut alpha = false;
    if compression == 3 || compression == 6 {
        if [u32_at(bytes, 40)?, u32_at(bytes, 44)?, u32_at(bytes, 48)?] != [0xff0000, 0xff00, 0xff]
        {
            return Err("暂不支持这种DIB颜色掩码".into());
        }
        let mask = if header >= 56 || extra == 16 {
            u32_at(bytes, 52)?
        } else {
            0
        };
        if mask != 0 && mask != 0xff000000 {
            return Err("暂不支持这种DIB透明掩码".into());
        }
        alpha = mask != 0;
    }
    let stride = (w as usize * bits).div_ceil(32) * 4;
    let offset = header + extra;
    let required = offset
        .checked_add(stride.checked_mul(h as usize).ok_or("图片容量溢出")?)
        .ok_or("图片容量溢出")?;
    if required > bytes.len() {
        return Err("DIB图片像素不完整".into());
    }
    let mut rgba = vec![0; len];
    for y in 0..h as usize {
        let src_y = if height > 0 { h as usize - 1 - y } else { y };
        for x in 0..w as usize {
            let at = offset + src_y * stride + x * bits / 8;
            let out = (y * w as usize + x) * 4;
            rgba[out..out + 4].copy_from_slice(&[
                bytes[at + 2],
                bytes[at + 1],
                bytes[at],
                if alpha { bytes[at + 3] } else { 255 },
            ]);
        }
    }
    Packet::picture(Image::new([w, h], rgba)?)
}
pub fn png(bytes: &[u8], max_bytes: usize) -> Result<Packet, String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_limits(png::Limits { bytes: MAX_IMAGE });
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("PNG图片头无效：{e}"))?;
    let [w, h] = [reader.info().width, reader.info().height];
    let len = image_bytes(w, h, max_bytes)?;
    let size = reader
        .output_buffer_size()
        .filter(|n| *n <= MAX_IMAGE)
        .ok_or("PNG输出超过容量")?;
    let mut decoded = vec![0; size];
    let info = reader
        .next_frame(&mut decoded)
        .map_err(|e| format!("PNG解码失败：{e}"))?;
    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => return Err("PNG色彩格式未展开".into()),
    };
    let mut rgba = Vec::with_capacity(len);
    for pixel in decoded[..info.buffer_size()].chunks_exact(channels) {
        rgba.extend_from_slice(&match channels {
            1 => [pixel[0], pixel[0], pixel[0], 255],
            2 => [pixel[0], pixel[0], pixel[0], pixel[1]],
            3 => [pixel[0], pixel[1], pixel[2], 255],
            _ => [pixel[0], pixel[1], pixel[2], pixel[3]],
        });
    }
    Packet::picture(Image::new([w, h], rgba)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::clipboard::Content;
    #[test]
    fn unicode_preserves_chinese_emoji_and_rejects_bad_termination_or_surrogates() {
        let bytes: Vec<_> = "中文 😀\n"
            .encode_utf16()
            .chain(Some(0))
            .flat_map(u16::to_le_bytes)
            .collect();
        assert!(
            matches!(text(&bytes).unwrap().unwrap().content,Content::Text(t) if t.as_ref()=="中文 😀\n")
        );
        assert!(text(&bytes[..bytes.len() - 2]).is_err());
        assert!(text(&[0, 0xd8, 0, 0]).is_err());
        assert!(text(&[1]).is_err());
        assert!(text(&[0, 0]).unwrap().is_none());
    }
    #[test]
    fn bottom_up_rgb_and_top_down_rgba_preserve_original_pixels() {
        let mut bytes = vec![0; 48];
        bytes[..4].copy_from_slice(&40u32.to_le_bytes());
        bytes[4..8].copy_from_slice(&1i32.to_le_bytes());
        bytes[8..12].copy_from_slice(&2i32.to_le_bytes());
        bytes[12..14].copy_from_slice(&1u16.to_le_bytes());
        bytes[14..16].copy_from_slice(&24u16.to_le_bytes());
        bytes[40..].copy_from_slice(&[3, 2, 1, 0, 6, 5, 4, 0]);
        let p = dib(&bytes, MAX_IMAGE).unwrap();
        assert!(matches!(p.content,Content::Picture(i) if i.pixels()==[4,5,6,255,1,2,3,255]));
        let mut b = vec![0; 128];
        for (at, v) in [
            (0, 124u32),
            (4, 1),
            (8, (-1i32) as u32),
            (16, 3),
            (40, 0xff0000),
            (44, 0xff00),
            (48, 0xff),
            (52, 0xff000000),
        ] {
            b[at..at + 4].copy_from_slice(&v.to_le_bytes());
        }
        b[12..14].copy_from_slice(&1u16.to_le_bytes());
        b[14..16].copy_from_slice(&32u16.to_le_bytes());
        b[124..].copy_from_slice(&[3, 2, 1, 70]);
        assert!(
            matches!(dib(&b,MAX_IMAGE).unwrap().content,Content::Picture(i) if i.pixels()==[1,2,3,70])
        );
        assert!(dib(&b[..125], MAX_IMAGE).is_err());
        b[4..8].copy_from_slice(&i32::MAX.to_le_bytes());
        assert!(dib(&b, MAX_IMAGE).is_err());
    }
    #[test]
    fn png_is_bounded_and_preserves_rgba() {
        let mut bytes = vec![];
        let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[1, 2, 3, 255, 4, 5, 6, 0])
            .unwrap();
        assert!(
            matches!(png(&bytes,MAX_IMAGE).unwrap().content,Content::Picture(i) if i.size()==[2,1] && i.pixels()==[1,2,3,255,4,5,6,0])
        );
        assert!(png(&bytes, 4).is_err());
        assert!(png(&bytes[..10], MAX_IMAGE).is_err());
    }
}
