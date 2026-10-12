use super::super::{glass, theme};
use eframe::egui::{self, Color32};

#[derive(Clone, Copy)]
pub enum Role {
    Primary,
    Secondary,
    Quiet,
    Selected,
    Danger,
}

pub fn paint(ui: &egui::Ui, response: &egui::Response, role: Role, radius: u8) {
    let enabled = response.enabled();
    let (base, edge) = match role {
        Role::Primary => (theme::CONTROL_SELECTED, theme::ACCENT),
        Role::Secondary => (theme::CONTROL, theme::LINE),
        Role::Quiet => (Color32::TRANSPARENT, Color32::TRANSPARENT),
        Role::Selected => (theme::CONTROL_SELECTED, theme::ACCENT.gamma_multiply(0.65)),
        Role::Danger => (theme::DANGER_SURFACE, theme::DANGER.gamma_multiply(0.65)),
    };
    let fill = if !enabled {
        theme::CONTROL_DISABLED
    } else if response.is_pointer_button_down_on() {
        if matches!(role, Role::Danger) {
            theme::DANGER_PRESSED
        } else {
            theme::CONTROL_PRESSED
        }
    } else if response.hovered() {
        if matches!(role, Role::Danger) {
            theme::DANGER_SURFACE
        } else {
            theme::HOVER
        }
    } else {
        base
    };
    ui.painter().rect_filled(response.rect, radius, fill);
    let stroke = if response.has_focus() && enabled {
        egui::Stroke::new(1.5_f32, theme::ACCENT)
    } else {
        egui::Stroke::new(
            1.0_f32,
            if enabled {
                edge
            } else {
                theme::LINE.gamma_multiply(0.45)
            },
        )
    };
    ui.painter()
        .rect_stroke(response.rect, radius, stroke, egui::StrokeKind::Inside);
    if enabled && !matches!(role, Role::Quiet) {
        ui.painter().add(glass::finish(
            response.rect.shrink(1.0),
            (radius - 1) as f32,
            response.hovered(),
        ));
    }
}

pub fn foreground(response: &egui::Response) -> Color32 {
    if response.enabled() {
        theme::TEXT
    } else {
        theme::DISABLED_TEXT
    }
}
