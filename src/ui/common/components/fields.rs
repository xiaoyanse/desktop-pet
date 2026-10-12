use super::super::theme;
use eframe::egui::{self, TextEdit};

/// Retains egui text editing, IME, selection and keyboard behavior.
pub fn entry(ui: &mut egui::Ui, edit: TextEdit<'_>, invalid: bool) -> egui::Response {
    let mut response = ui.add(
        edit.font(egui::FontId::proportional(14.0))
            .margin(egui::Margin::symmetric(12, 9))
            .min_size(egui::vec2(0.0, theme::CONTROL_HEIGHT))
            .background_color(if ui.is_enabled() {
                theme::FIELD
            } else {
                theme::CONTROL_DISABLED
            }),
    );
    // egui 0.31 returns the inner text rectangle; outlines belong to the outer field.
    response.rect = response.rect.expand2(egui::vec2(12.0, 9.0));
    if invalid || response.has_focus() || response.hovered() {
        ui.painter().rect_stroke(
            response.rect,
            theme::CONTROL_RADIUS,
            egui::Stroke::new(
                if response.has_focus() {
                    1.5_f32
                } else {
                    1.0_f32
                },
                if invalid {
                    theme::DANGER
                } else if response.has_focus() {
                    theme::ACCENT
                } else {
                    theme::LINE_HOVER
                },
            ),
            egui::StrokeKind::Inside,
        );
    }
    response
}

pub fn line(ui: &mut egui::Ui, text: &mut String, hint: &str, limit: usize) -> egui::Response {
    entry(
        ui,
        TextEdit::singleline(text)
            .hint_text(hint)
            .char_limit(limit)
            .desired_width(ui.available_width()),
        false,
    )
}

pub fn multiline(
    ui: &mut egui::Ui,
    text: &mut String,
    hint: &str,
    limit: usize,
    rows: usize,
) -> egui::Response {
    entry(
        ui,
        TextEdit::multiline(text)
            .hint_text(hint)
            .char_limit(limit)
            .desired_rows(rows)
            .desired_width(ui.available_width()),
        false,
    )
}
