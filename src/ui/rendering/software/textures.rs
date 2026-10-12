use eframe::egui::{self, ImageData, TextureId, TextureOptions};
use std::{collections::HashMap, sync::Arc};

const MAX_TEXTURE_BYTES: usize = 16 * 1024 * 1024;
const MAX_TEXTURES: usize = 64;

pub(super) struct Texture {
    pub image: Arc<egui::ColorImage>,
    pub linear: bool,
}

#[derive(Default)]
pub(super) struct Textures(HashMap<TextureId, Texture>);

impl Textures {
    pub fn contains(&self, id: TextureId) -> bool {
        self.0.contains_key(&id)
    }
    pub fn get(&self, id: TextureId) -> &Texture {
        &self.0[&id]
    }
    pub fn bytes(&self) -> usize {
        self.0.values().map(|t| t.image.pixels.len() * 4).sum()
    }

    pub fn update(&mut self, delta: &egui::TexturesDelta) -> Result<(), String> {
        // Validate the complete batch before allocating or changing live textures.
        let mut planned: HashMap<_, _> = self.0.iter().map(|(id, t)| (*id, t.image.size)).collect();
        for (id, update) in &delta.set {
            validate_options(update.options)?;
            let size = update.image.size();
            let count = size[0].checked_mul(size[1]).filter(|n| *n > 0);
            let pixels = match &update.image {
                ImageData::Color(image) => image.pixels.len(),
                ImageData::Font(image) => image.pixels.len(),
            };
            if count != Some(pixels) || pixels > MAX_TEXTURE_BYTES / 4 {
                return Err("软件绘制实验：纹理尺寸或容量非法".into());
            }
            if let Some(pos) = update.pos {
                let old = planned.get(id).ok_or("软件绘制实验：局部纹理尚未创建")?;
                for axis in 0..2 {
                    if pos[axis]
                        .checked_add(size[axis])
                        .is_none_or(|end| end > old[axis])
                    {
                        return Err("软件绘制实验：纹理更新越界".into());
                    }
                }
            } else {
                planned.insert(*id, size);
            }
            if planned.len() > MAX_TEXTURES {
                return Err("软件绘制实验：纹理数量超过预算".into());
            }
        }
        let bytes = planned.values().try_fold(0usize, |sum, size| {
            sum.checked_add(size[0].checked_mul(size[1])?.checked_mul(4)?)
        });
        if planned.len() > MAX_TEXTURES || bytes.is_none_or(|bytes| bytes > MAX_TEXTURE_BYTES) {
            return Err("软件绘制实验：纹理总容量超过预算".into());
        }
        for (id, update) in &delta.set {
            if let Some([x, y]) = update.pos {
                let texture = self.0.get_mut(id).expect("批次已验证纹理存在");
                let image = Arc::make_mut(&mut texture.image);
                let [width, height] = update.image.size();
                let mut font_pixels = match &update.image {
                    ImageData::Font(patch) => Some(patch.srgba_pixels(None)),
                    _ => None,
                };
                for row in 0..height {
                    let source = row * width;
                    let dest = (row + y) * image.size[0] + x;
                    match &update.image {
                        ImageData::Color(patch) => image.pixels[dest..dest + width]
                            .copy_from_slice(&patch.pixels[source..source + width]),
                        ImageData::Font(_) => {
                            for (dst, src) in image.pixels[dest..dest + width].iter_mut().zip(
                                font_pixels
                                    .as_mut()
                                    .expect("字体分支已创建迭代器")
                                    .by_ref()
                                    .take(width),
                            ) {
                                *dst = src;
                            }
                        }
                    }
                }
                texture.linear = update.options.magnification == egui::TextureFilter::Linear;
            } else {
                let image = match &update.image {
                    ImageData::Color(image) => image.clone(),
                    ImageData::Font(image) => Arc::new(egui::ColorImage {
                        size: image.size,
                        pixels: image.srgba_pixels(None).collect(),
                    }),
                };
                self.0.insert(
                    *id,
                    Texture {
                        image,
                        linear: update.options.magnification == egui::TextureFilter::Linear,
                    },
                );
            }
        }
        Ok(())
    }

    pub fn free(&mut self, delta: &egui::TexturesDelta) {
        for id in &delta.free {
            self.0.remove(id);
        }
    }
}

fn validate_options(options: TextureOptions) -> Result<(), String> {
    if options.wrap_mode != egui::TextureWrapMode::ClampToEdge
        || options.minification != options.magnification
        || options.mipmap_mode.is_some()
    {
        return Err("软件绘制实验尚不支持此纹理过滤或重复模式".into());
    }
    Ok(())
}

impl Texture {
    pub fn sample(&self, uv: egui::Pos2) -> [f32; 4] {
        let [w, h] = self.image.size;
        let x = (uv.x * w as f32 - 0.5).clamp(0.0, (w - 1) as f32);
        let y = (uv.y * h as f32 - 0.5).clamp(0.0, (h - 1) as f32);
        let pixel = |x: usize, y: usize| self.image.pixels[y * w + x].to_array().map(f32::from);
        if !self.linear {
            return pixel((x + 0.5).floor() as usize, (y + 0.5).floor() as usize);
        }
        let (ix, iy) = (x.floor() as usize, y.floor() as usize);
        let (fx, fy) = (x - ix as f32, y - iy as f32);
        let p = [
            pixel(ix, iy),
            pixel((ix + 1).min(w - 1), iy),
            pixel(ix, (iy + 1).min(h - 1)),
            pixel((ix + 1).min(w - 1), (iy + 1).min(h - 1)),
        ];
        std::array::from_fn(|i| {
            (p[0][i] * (1.0 - fx) + p[1][i] * fx) * (1.0 - fy)
                + (p[2][i] * (1.0 - fx) + p[3][i] * fx) * fy
        })
    }
}
