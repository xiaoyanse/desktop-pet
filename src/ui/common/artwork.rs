use crate::assets;
use eframe::egui::{self, Color32};

pub fn texture(ctx: &egui::Context, name: &str) -> egui::TextureHandle {
    let id = egui::Id::new(("embedded-artwork-v1", name));
    if let Some(texture) = ctx.data_mut(|data| data.get_temp::<egui::TextureHandle>(id)) {
        return texture;
    }
    let source = assets::get(name);
    let image = egui::ColorImage::from_rgba_unmultiplied(source.size, source.rgba);
    let texture = ctx.load_texture(source.name, image, egui::TextureOptions::LINEAR);
    ctx.data_mut(|data| data.insert_temp(id, texture.clone()));
    texture
}

pub fn draw(painter: &egui::Painter, rect: egui::Rect, name: &str, tint: Color32) {
    let texture = texture(painter.ctx(), name);
    painter.image(
        texture.id(),
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        tint,
    );
}

pub fn release(ctx: &egui::Context, name: &str) {
    ctx.data_mut(|data| {
        data.remove::<egui::TextureHandle>(egui::Id::new(("embedded-artwork-v1", name)))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_painting_reuses_texture_and_release_allows_reloading() {
        let ctx = egui::Context::default();
        let mut initial = None;
        for frame in 0..8 {
            let out = ctx.run(egui::RawInput::default(), |ctx| {
                let texture = texture(ctx, "clean");
                if let Some(id) = initial {
                    assert_eq!(texture.id(), id);
                } else {
                    initial = Some(texture.id());
                }
                let p = ctx.layer_painter(egui::LayerId::background());
                for _ in 0..20 {
                    draw(
                        &p,
                        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(24.0, 24.0)),
                        "clean",
                        Color32::WHITE,
                    );
                }
            });
            if frame == 0 {
                assert!(out.textures_delta.set.len() <= 2);
            } else {
                assert!(out.textures_delta.set.is_empty());
            }
        }
        release(&ctx, "clean");
        assert_ne!(texture(&ctx, "clean").id(), initial.unwrap());
    }
}
