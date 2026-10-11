use eframe::egui::{self, Color32};
use std::{collections::VecDeque, time::Instant};

pub fn show(
    ui: &mut egui::Ui,
    history: Option<&VecDeque<(Instant, f64)>>,
    percent: bool,
    color: Color32,
) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::hover());
    let Some(history) = history.filter(|h| h.len() > 1) else {
        return;
    };
    let max = if percent {
        100.0
    } else {
        history
            .iter()
            .map(|(_, v)| *v)
            .reduce(f64::max)
            .unwrap_or(1.0)
            .max(0.001)
    };
    let now = Instant::now();
    let points = history
        .iter()
        .map(|(time, value)| {
            egui::pos2(
                rect.right()
                    - (now.duration_since(*time).as_secs_f32() / 60.0).min(1.0) * rect.width(),
                rect.bottom() - (*value / max).clamp(0.0, 1.0) as f32 * rect.height(),
            )
        })
        .collect();
    ui.painter().add(egui::Shape::line(
        points,
        egui::Stroke::new(1.25_f32, color),
    ));
}
