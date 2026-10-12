use crate::ui::common::artwork;
use eframe::egui::{self, Color32};

pub fn show(ctx: &egui::Context) {
    let rect = ctx.screen_rect();
    let texture = artwork::texture(ctx, "workspace");
    let aspect = rect.width().max(1.0) / rect.height().max(1.0);
    let image_aspect = texture.size_vec2().x / texture.size_vec2().y;
    let visible = if aspect < image_aspect {
        egui::vec2(aspect / image_aspect, 1.0)
    } else {
        egui::vec2(1.0, image_aspect / aspect)
    };
    let uv = egui::Rect::from_center_size(egui::pos2(0.5, 0.5), visible);
    let painter = ctx.layer_painter(egui::LayerId::background());
    painter.add(
        egui::epaint::RectShape::filled(rect, 18, Color32::from_rgb(127, 143, 161))
            .with_texture(texture.id(), uv),
    );
    painter.rect_filled(
        egui::Rect::from_min_size(
            rect.min,
            egui::vec2(super::navigation::WIDTH, rect.height()),
        ),
        egui::CornerRadius {
            nw: 18,
            ne: 0,
            sw: 18,
            se: 0,
        },
        Color32::from_rgba_unmultiplied(17, 29, 41, 115),
    );
    painter.rect_stroke(
        rect.shrink(0.5),
        18,
        egui::Stroke::new(1.0_f32, Color32::from_white_alpha(100)),
        egui::StrokeKind::Inside,
    );
}
