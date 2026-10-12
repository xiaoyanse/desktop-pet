use super::{Image, Region};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    Fullscreen,
    #[default]
    Rectangle,
    Freehand,
    Window,
}
impl Mode {
    pub const ALL: [Self; 4] = [
        Self::Rectangle,
        Self::Freehand,
        Self::Window,
        Self::Fullscreen,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Fullscreen => "全屏截图",
            Self::Rectangle => "矩形区域",
            Self::Freehand => "自由形状",
            Self::Window => "识别窗口",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Fullscreen => "截取整个桌面，包含副屏",
            Self::Rectangle => "框选区域，方向键微调后确认",
            Self::Freehand => "画出闭合轮廓，外部保持透明",
            Self::Window => "悬停识别可见窗口，点击选定",
        }
    }
}

#[derive(Clone, Debug)]
pub struct WindowTarget {
    pub handle: u64,
    pub pid: u32,
}
#[derive(Clone, Debug)]
pub struct Selection {
    pub region: Region,
    pub polygon: Vec<[i32; 2]>,
    pub window: Option<WindowTarget>,
}
impl Selection {
    pub fn rectangle(a: [i32; 2], b: [i32; 2], bounds: Region) -> Option<Self> {
        let width = (a[0] as i64 - b[0] as i64).unsigned_abs();
        let height = (a[1] as i64 - b[1] as i64).unsigned_abs();
        let region = Region {
            left: a[0].min(b[0]),
            top: a[1].min(b[1]),
            width: u32::try_from(width).ok()?,
            height: u32::try_from(height).ok()?,
        };
        bounds.intersect(region).map(|region| Self {
            region,
            polygon: vec![],
            window: None,
        })
    }
    pub fn freehand(points: Vec<[i32; 2]>, bounds: Region) -> Result<Self, String> {
        if !(3..=1024).contains(&points.len()) {
            return Err("自由形状需要至少三个点，最多1024点".into());
        }
        let lo = [
            points.iter().map(|p| p[0]).min().unwrap(),
            points.iter().map(|p| p[1]).min().unwrap(),
        ];
        let hi = [
            points.iter().map(|p| p[0]).max().unwrap(),
            points.iter().map(|p| p[1]).max().unwrap(),
        ];
        let mut selection = Self::rectangle(lo, hi, bounds).ok_or("自由形状没有有效面积")?;
        if points.iter().any(|p| {
            p[0] < bounds.left
                || p[1] < bounds.top
                || p[0] as i64 >= bounds.left as i64 + bounds.width as i64
                || p[1] as i64 >= bounds.top as i64 + bounds.height as i64
        }) {
            return Err("自由形状超出桌面".into());
        }
        selection.region.bytes()?;
        selection.polygon = points;
        Ok(selection)
    }
    pub fn apply_mask(&self, image: Image) -> Result<Image, String> {
        if image.size() != [self.region.width, self.region.height] {
            return Err("选区尺寸已经变化，请重试".into());
        }
        if self.polygon.is_empty() {
            return Ok(image);
        }
        // Scanline even/odd fill also gives self-crossing paths a deterministic mask.
        let mut pixels = image.pixels().to_vec();
        let mut inside = false;
        let mut intersections = Vec::with_capacity(self.polygon.len());
        for row in 0..self.region.height {
            intersections.clear();
            let y = self.region.top as f64 + row as f64 + 0.5;
            for index in 0..self.polygon.len() {
                let a = self.polygon[index];
                let b = self.polygon[(index + 1) % self.polygon.len()];
                if (a[1] as f64 > y) != (b[1] as f64 > y) {
                    intersections.push(
                        a[0] as f64
                            + (y - a[1] as f64) * (b[0] as f64 - a[0] as f64)
                                / (b[1] as f64 - a[1] as f64),
                    );
                }
            }
            intersections.sort_by(f64::total_cmp);
            let mut edge = 0;
            for column in 0..self.region.width {
                let x = self.region.left as f64 + column as f64 + 0.5;
                while edge < intersections.len() && intersections[edge] <= x {
                    edge += 1;
                }
                if edge % 2 == 0 {
                    let index = ((row as usize * self.region.width as usize) + column as usize) * 4;
                    pixels[index..index + 4].fill(0);
                } else {
                    inside = true;
                }
            }
        }
        if !inside {
            return Err("自由形状没有有效面积，请重新画出轮廓".into());
        }
        Image::new(image.size(), pixels)
    }
}
