use eframe::egui::{self, Color32};

pub const BACKGROUND: Color32 = Color32::from_rgb(17, 25, 35);
pub const SURFACE: Color32 = Color32::from_rgb(27, 39, 51);
pub const LINE: Color32 = Color32::from_rgb(57, 76, 91);
pub const LINE_HOVER: Color32 = Color32::from_rgb(92, 122, 133);
pub const TEXT: Color32 = Color32::from_rgb(236, 239, 240);
pub const MUTED: Color32 = Color32::from_rgb(181, 194, 204);
pub const ACCENT: Color32 = Color32::from_rgb(165, 213, 200);
pub const CHAMPAGNE: Color32 = Color32::from_rgb(218, 201, 174);
pub const HOVER: Color32 = Color32::from_rgb(43, 57, 68);
pub const PROGRESS_TRACK: Color32 = Color32::from_rgb(75, 93, 105);
pub const FIELD: Color32 = Color32::from_rgb(23, 37, 49);
pub const CONTROL: Color32 = Color32::from_rgb(33, 49, 62);
pub const CONTROL_SELECTED: Color32 = Color32::from_rgb(42, 70, 73);
pub const CONTROL_PRESSED: Color32 = Color32::from_rgb(27, 55, 59);
pub const CONTROL_DISABLED: Color32 = Color32::from_rgb(29, 40, 51);
pub const DISABLED_TEXT: Color32 = Color32::from_rgb(112, 130, 145);
pub const WARNING: Color32 = Color32::from_rgb(225, 194, 134);
pub const DANGER: Color32 = Color32::from_rgb(242, 151, 147);
pub const DANGER_SURFACE: Color32 = Color32::from_rgb(68, 46, 53);
pub const DANGER_PRESSED: Color32 = Color32::from_rgb(55, 35, 43);
pub const CONTROL_HEIGHT: f32 = 36.0;
pub const CONTROL_RADIUS: u8 = 10;

pub fn glass_panel<R>(
    ui: &mut egui::Ui,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    super::glass::show(ui, glass_frame(), contents)
}

pub fn glass_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(28, 41, 54, 190))
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
    visuals.extreme_bg_color = FIELD;
    visuals.faint_bg_color = CONTROL;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, MUTED);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, TEXT);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, ACCENT);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, MUTED);
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::same(CONTROL_RADIUS);
    }
    visuals.window_corner_radius = egui::CornerRadius::same(16);
    visuals.menu_corner_radius = egui::CornerRadius::same(12);
    visuals.window_stroke = egui::Stroke::new(1.0_f32, LINE);
    visuals.slider_trailing_fill = true;
    visuals.warn_fg_color = WARNING;
    visuals.error_fg_color = DANGER;
    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 7.0);
        style.spacing.button_padding = egui::vec2(12.0, 8.0);
        style.spacing.slider_width = 200.0;
        style.spacing.scroll.bar_width = 5.0;
        style.animation_time = 0.08;
    });
}
