use eframe::egui::{self, Color32};

pub const BACKGROUND: Color32 = Color32::from_rgb(20, 24, 28);
pub const SURFACE: Color32 = Color32::from_rgb(28, 33, 38);
pub const LINE: Color32 = Color32::from_rgb(49, 57, 63);
pub const TEXT: Color32 = Color32::from_rgb(232, 236, 235);
pub const MUTED: Color32 = Color32::from_rgb(151, 162, 164);
pub const ACCENT: Color32 = Color32::from_rgb(119, 219, 191);

pub fn setup(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = Color32::from_rgb(35, 69, 59);
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(35, 42, 46);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(35, 42, 46);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(68, 79, 82));
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(44, 56, 55);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(44, 56, 55);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, ACCENT.gamma_multiply(0.7));
    visuals.widgets.active.bg_fill = Color32::from_rgb(39, 79, 66);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5_f32, ACCENT);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, LINE);
    visuals.window_fill = SURFACE;
    visuals.panel_fill = BACKGROUND;
    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 7.0);
        style.spacing.button_padding = egui::vec2(9.0, 5.0);
        style.spacing.slider_width = 200.0;
        style.spacing.scroll.bar_width = 5.0;
        style.animation_time = 0.08;
    });
}
