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

pub fn draw_content(painter: &egui::Painter, rect: egui::Rect, name: &str) {
    let ctx = painter.ctx();
    let key = egui::Id::new(("artwork-content-bounds", name));
    let uv = ctx
        .data_mut(|data| data.get_temp::<egui::Rect>(key))
        .unwrap_or_else(|| {
            let source = assets::get(name);
            let [width, height] = source.size;
            let mut bounds = [width, height, 0, 0];
            for (index, pixel) in source.rgba.chunks_exact(4).enumerate() {
                if pixel[3] >= 180 {
                    let (x, y) = (index % width, index / width);
                    bounds = [
                        bounds[0].min(x),
                        bounds[1].min(y),
                        bounds[2].max(x + 1),
                        bounds[3].max(y + 1),
                    ];
                }
            }
            let uv = if bounds[2] > bounds[0] && bounds[3] > bounds[1] {
                egui::Rect::from_min_max(
                    egui::pos2(
                        bounds[0] as f32 / width as f32,
                        bounds[1] as f32 / height as f32,
                    ),
                    egui::pos2(
                        bounds[2] as f32 / width as f32,
                        bounds[3] as f32 / height as f32,
                    ),
                )
            } else {
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0))
            };
            ctx.data_mut(|data| data.insert_temp(key, uv));
            uv
        });
    let source = assets::get(name);
    let size = uv.size() * egui::vec2(source.size[0] as f32, source.size[1] as f32);
    let ratio = (rect.width() / size.x).min(rect.height() / size.y);
    painter.image(
        texture(ctx, name).id(),
        egui::Rect::from_center_size(rect.center(), size * ratio),
        uv,
        Color32::WHITE,
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
    fn material_thumbnail_keeps_content_aspect_and_reuses_bounds() {
        let ctx = egui::Context::default();
        let target = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(214.0, 94.0));
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            draw_content(
                &ctx.layer_painter(egui::LayerId::background()),
                target,
                "view-glass",
            );
        });
        let uv = ctx
            .data_mut(|data| {
                data.get_temp::<egui::Rect>(egui::Id::new(("artwork-content-bounds", "view-glass")))
            })
            .unwrap();
        let drawn = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Mesh(m) if m.texture_id == texture(&ctx, "view-glass").id() => {
                    Some(m.calc_bounds())
                }
                _ => None,
            })
            .unwrap();
        assert!(target.contains_rect(drawn));
        assert!((drawn.aspect_ratio() - uv.aspect_ratio()).abs() < 0.001);
        assert!(drawn.height() > 80.0);
    }

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
