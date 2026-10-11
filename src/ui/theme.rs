use eframe::egui::{self, Color32};

pub fn setup(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(Color32::from_rgb(225, 232, 239));
    visuals.selection.bg_fill = Color32::from_rgb(39, 93, 91);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(37, 46, 58);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(42, 58, 73);
    visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(1.0_f32, Color32::from_rgb(80, 108, 126));
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(53, 68, 83);
    visuals.window_fill = Color32::from_rgb(23, 29, 37);
    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 7.0);
        style.spacing.button_padding = egui::vec2(9.0, 5.0);
        style.animation_time = 0.08;
    });
}
