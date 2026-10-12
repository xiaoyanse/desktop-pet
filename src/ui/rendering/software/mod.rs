mod raster;
#[cfg(test)]
mod tests;
mod textures;

use eframe::egui::{self, epaint};
use textures::Textures;

pub(crate) const MAX_SURFACE_BYTES: usize = 32 * 1024 * 1024;

/// Framework adapter for the explicit software experiment, independent of native windows.
#[derive(Default)]
pub(crate) struct Painter {
    textures: Textures,
}

impl Painter {
    pub fn update(&mut self, delta: &egui::TexturesDelta) -> Result<(), String> {
        self.textures.update(delta)
    }

    pub fn free(&mut self, delta: &egui::TexturesDelta) {
        self.textures.free(delta);
    }

    pub fn texture_bytes(&self) -> usize {
        self.textures.bytes()
    }

    pub fn paint(
        &self,
        target: &mut [u8],
        size: [usize; 2],
        scale: f32,
        primitives: &[egui::ClippedPrimitive],
    ) -> Result<(), String> {
        let expected = size[0]
            .checked_mul(size[1])
            .and_then(|pixels| pixels.checked_mul(4))
            .filter(|bytes| *bytes <= MAX_SURFACE_BYTES);
        if size.contains(&0) || expected != Some(target.len()) || !scale.is_finite() || scale <= 0.0
        {
            return Err("软件绘制实验：非法或超过容量的像素缓冲".into());
        }
        for item in primitives {
            let epaint::Primitive::Mesh(mesh) = &item.primitive else {
                return Err("软件绘制实验尚不支持自定义GPU回调".into());
            };
            if !mesh.is_valid()
                || mesh.indices.len() % 3 != 0
                || mesh
                    .vertices
                    .iter()
                    .any(|v| !(v.pos * scale).is_finite() || !v.uv.is_finite())
                || !item.clip_rect.is_finite()
                || !self.textures.contains(mesh.texture_id)
            {
                return Err("软件绘制实验：非法网格、裁切或缺失纹理".into());
            }
        }
        target.fill(0);
        for item in primitives {
            if let epaint::Primitive::Mesh(mesh) = &item.primitive {
                raster::paint(
                    target,
                    size,
                    scale,
                    item.clip_rect,
                    mesh,
                    self.textures.get(mesh.texture_id),
                );
            }
        }
        Ok(())
    }
}
