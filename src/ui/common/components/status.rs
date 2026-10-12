use super::super::{
    icons::{self, Icon},
    theme,
};
use eframe::egui::{self, RichText};
#[derive(Clone, Copy)]
pub enum Tone {
    Neutral,
    Success,
    Warning,
    Error,
}
impl Tone {
    pub fn color(self) -> egui::Color32 {
        match self {
            Self::Neutral => theme::MUTED,
            Self::Success => theme::ACCENT,
            Self::Warning => theme::WARNING,
            Self::Error => theme::DANGER,
        }
    }
}
pub fn badge(ui: &mut egui::Ui, label: &str, tone: Tone) -> egui::Response {
    egui::Frame::new()
        .fill(tone.color().gamma_multiply(0.12))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(9, 5))
        .show(ui, |ui| {
            ui.label(RichText::new(label).size(12.0).color(tone.color()))
        })
        .response
}
pub fn message(ui: &mut egui::Ui, label: &str, tone: Tone) {
    egui::Frame::new()
        .fill(theme::FIELD)
        .stroke(egui::Stroke::new(1.0_f32, tone.color().gamma_multiply(0.4)))
        .corner_radius(theme::CONTROL_RADIUS)
        .inner_margin(12)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::splat(22.0), egui::Sense::hover());
                icons::draw(
                    ui.painter(),
                    rect,
                    match tone {
                        Tone::Error => Icon::Cancel,
                        Tone::Warning => Icon::Reminder,
                        _ => Icon::Check,
                    },
                    egui::Color32::WHITE,
                );
                ui.add(
                    egui::Label::new(RichText::new(label).size(13.0).color(tone.color())).wrap(),
                );
            });
        });
}
