use eframe::egui::{self, Color32};

pub const BACKGROUND: Color32 = Color32::from_rgb(17, 25, 35);
pub const SURFACE: Color32 = Color32::from_rgb(27, 39, 51);
pub const LINE: Color32 = Color32::from_rgba_premultiplied(30, 31, 32, 38);
pub const TEXT: Color32 = Color32::from_rgb(236, 239, 240);
pub const MUTED: Color32 = Color32::from_rgb(181, 194, 204);
pub const ACCENT: Color32 = Color32::from_rgb(165, 213, 200);
pub const CHAMPAGNE: Color32 = Color32::from_rgb(218, 201, 174);
pub const HOVER: Color32 = Color32::from_rgb(43, 57, 68);
pub const PROGRESS_TRACK: Color32 = Color32::from_rgb(75, 93, 105);

pub fn glass_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(28, 41, 54, 96))
        .stroke(egui::Stroke::new(1.0_f32, LINE))
        .corner_radius(16)
        .inner_margin(20)
        .shadow(egui::Shadow {
            offset: [0, 4],
            blur: 12,
            spread: 0,
            color: Color32::from_black_alpha(20),
        })
}

pub fn setup(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = Color32::from_rgb(48, 68, 72);
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(35, 48, 61);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(35, 48, 61);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, LINE);
    visuals.widgets.hovered.bg_fill = HOVER;
    visuals.widgets.hovered.weak_bg_fill = HOVER;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, ACCENT.gamma_multiply(0.7));
    visuals.widgets.active.bg_fill = Color32::from_rgb(48, 72, 73);
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
