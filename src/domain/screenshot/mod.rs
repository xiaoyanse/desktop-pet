mod drawing;
mod selection;
pub use selection::{Mode, Selection, WindowTarget};
#[cfg(test)]
mod tests;
use std::sync::Arc;

pub const MAX_IMAGE_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_MARKS: usize = 128;
const MAX_RENDER_WORK: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}
impl Region {
    pub fn bytes(self) -> Result<usize, String> {
        if self.width == 0
            || self.height == 0
            || self.width > 16384
            || self.height > 16384
            || self.left.checked_add_unsigned(self.width).is_none()
            || self.top.checked_add_unsigned(self.height).is_none()
        {
            return Err("截图区域为空或坐标越界".into());
        }
        (self.width as usize)
            .checked_mul(self.height as usize)
            .and_then(|n| n.checked_mul(4))
            .filter(|n| *n <= MAX_IMAGE_BYTES)
            .ok_or_else(|| "选区超过128 MiB图片容量，请缩小选区或分屏截图".into())
    }
    pub fn contains(self, other: Self) -> bool {
        other.left >= self.left
            && other.top >= self.top
            && other.left as i64 + other.width as i64 <= self.left as i64 + self.width as i64
            && other.top as i64 + other.height as i64 <= self.top as i64 + self.height as i64
    }
    pub fn intersect(self, other: Self) -> Option<Self> {
        let left = self.left.max(other.left);
        let top = self.top.max(other.top);
        let right =
            (self.left as i64 + self.width as i64).min(other.left as i64 + other.width as i64);
        let bottom =
            (self.top as i64 + self.height as i64).min(other.top as i64 + other.height as i64);
        (right > left as i64 && bottom > top as i64).then_some(Self {
            left,
            top,
            width: (right - left as i64).max(0) as u32,
            height: (bottom - top as i64).max(0) as u32,
        })
    }
}

#[derive(Clone)]
pub struct Image {
    size: [u32; 2],
    pixels: Arc<Vec<u8>>,
}
impl Image {
    pub fn new(size: [u32; 2], pixels: Vec<u8>) -> Result<Self, String> {
        let expected = Region {
            left: 0,
            top: 0,
            width: size[0],
            height: size[1],
        }
        .bytes()?;
        if expected != pixels.len() {
            return Err("图片像素与尺寸不一致".into());
        }
        Ok(Self {
            size,
            pixels: Arc::new(pixels),
        })
    }
    pub fn size(&self) -> [u32; 2] {
        self.size
    }
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Rectangle,
    Arrow,
    Pen,
    Cover,
}
impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Self::Rectangle => "矩形",
            Self::Arrow => "箭头",
            Self::Pen => "画笔",
            Self::Cover => "遮挡",
        }
    }
}
#[derive(Clone)]
pub struct Mark {
    pub tool: Tool,
    pub points: Vec<[u32; 2]>,
    pub color: [u8; 3],
    pub width: u32,
}

#[derive(Clone)]
pub struct Document {
    pub image: Image,
    marks: Vec<Mark>,
    redo: Vec<Mark>,
}
impl Document {
    pub fn new(image: Image) -> Self {
        Self {
            image,
            marks: Vec::new(),
            redo: Vec::new(),
        }
    }
    pub fn marks(&self) -> &[Mark] {
        &self.marks
    }
    pub fn can_undo(&self) -> bool {
        !self.marks.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn add(&mut self, mark: Mark) -> Result<(), String> {
        if self.marks.len() >= MAX_MARKS {
            return Err("批注达到128项，请撤销或保存后重新截图".into());
        }
        let [w, h] = self.image.size;
        let valid_count = match mark.tool {
            Tool::Pen => (2..=1024).contains(&mark.points.len()),
            _ => mark.points.len() == 2,
        };
        if !valid_count
            || !(1..=24).contains(&mark.width)
            || mark.points.iter().any(|p| p[0] >= w || p[1] >= h)
        {
            return Err("批注坐标、笔宽或点数量非法".into());
        }
        self.marks.push(mark);
        if self.marks.iter().map(Mark::render_work).sum::<u64>() > MAX_RENDER_WORK {
            self.marks.pop();
            return Err("批注绘制量较大，请撤销部分批注或保存后重新截图".into());
        }
        self.redo.clear();
        Ok(())
    }
    pub fn undo(&mut self) {
        if let Some(mark) = self.marks.pop() {
            self.redo.push(mark);
        }
    }
    pub fn redo(&mut self) {
        if let Some(mark) = self.redo.pop() {
            self.marks.push(mark);
        }
    }
    pub fn render(&self) -> Result<Image, String> {
        let mut pixels = self.image.pixels.to_vec();
        drawing::render(&mut pixels, self.image.size, &self.marks);
        Image::new(self.image.size, pixels)
    }
}
impl Mark {
    fn render_work(&self) -> u64 {
        let a = self.points[0];
        let b = *self.points.last().unwrap();
        let dx = a[0].abs_diff(b[0]) as u64;
        let dy = a[1].abs_diff(b[1]) as u64;
        let brush = self.width as u64 * self.width as u64;
        match self.tool {
            Tool::Cover => (dx + 1) * (dy + 1),
            Tool::Rectangle => (2 * (dx + dy) + 4) * brush,
            Tool::Arrow => (dx.max(dy) + 1) * brush * 3,
            Tool::Pen => {
                self.points
                    .windows(2)
                    .map(|p| p[0][0].abs_diff(p[1][0]).max(p[0][1].abs_diff(p[1][1])) as u64 + 1)
                    .sum::<u64>()
                    * brush
            }
        }
    }
}
