use super::super::theme;
use eframe::egui;

pub fn row(ui: &mut egui::Ui, height: f32) -> egui::Rect {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    if ui.rect_contains_pointer(rect) {
        ui.painter()
            .rect_filled(rect.shrink2(egui::vec2(0.0, 2.0)), 8, theme::CONTROL);
    }
    ui.painter().line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        egui::Stroke::new(1.0_f32, theme::LINE.gamma_multiply(0.55)),
    );
    rect
}

pub fn dialog_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::SURFACE)
        .stroke(egui::Stroke::new(1.0_f32, theme::LINE))
        .corner_radius(16)
        .inner_margin(20)
}
