use super::super::{
    icons::{self, Icon},
    theme,
};
use eframe::egui::{self, Color32};
use std::ops::RangeInclusive;

pub fn compact(ui: &mut egui::Ui, value: &mut f64, range: RangeInclusive<f64>, unit: &str) -> bool {
    let mut changed = false;
    let mut focused = false;
    let frame = egui::Frame::new()
        .fill(if ui.is_enabled() {
            theme::FIELD
        } else {
            theme::CONTROL_DISABLED
        })
        .stroke(egui::Stroke::new(1.0_f32, theme::LINE))
        .corner_radius(theme::CONTROL_RADIUS)
        .inner_margin(egui::Margin::symmetric(3, 2));
    let group = frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let minus = ui
                .add_enabled_ui(*value > *range.start(), |ui| {
                    icons::sized_button(ui, Icon::Minus, "减少 1", 30.0)
                })
                .inner
                .on_hover_text("减少 1");
            if minus.clicked() {
                *value = (*value - 1.0).max(*range.start());
                changed = true;
            }
            let edit = ui
                .scope(|ui| {
                    ui.spacing_mut().interact_size = egui::vec2(62.0, 30.0);
                    ui.spacing_mut().button_padding = egui::vec2(6.0, 3.0);
                    let widgets = &mut ui.visuals_mut().widgets;
                    for v in [
                        &mut widgets.inactive,
                        &mut widgets.hovered,
                        &mut widgets.active,
                    ] {
                        v.bg_fill = Color32::TRANSPARENT;
                        v.weak_bg_fill = Color32::TRANSPARENT;
                        v.bg_stroke = egui::Stroke::NONE;
                    }
                    ui.add(
                        egui::DragValue::new(value)
                            .range(range.clone())
                            .speed(1.0)
                            .fixed_decimals(0)
                            .suffix(unit),
                    )
                    .on_hover_text("拖动调整，单击后输入数值")
                })
                .inner;
            changed |= edit.changed();
            focused = edit.has_focus();
            let plus = ui
                .add_enabled_ui(*value < *range.end(), |ui| {
                    icons::sized_button(ui, Icon::Plus, "增加 1", 30.0)
                })
                .inner
                .on_hover_text("增加 1");
            if plus.clicked() {
                *value = (*value + 1.0).min(*range.end());
                changed = true;
            }
        });
    });
    if ui.is_enabled() && (focused || group.response.hovered()) {
        ui.painter().rect_stroke(
            group.response.rect,
            theme::CONTROL_RADIUS,
            egui::Stroke::new(
                if focused { 1.5_f32 } else { 1.0_f32 },
                if focused {
                    theme::ACCENT
                } else {
                    theme::LINE_HOVER
                },
            ),
            egui::StrokeKind::Inside,
        );
    }
    changed
}

pub fn number(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f64,
    range: RangeInclusive<f64>,
    unit: &str,
) -> bool {
    let mut changed = false;
    ui.push_id(label, |ui| {
        ui.spacing_mut().interact_size.y = theme::CONTROL_HEIGHT;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(label).size(13.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(132.0 + unit.chars().count() as f32 * 7.0, 36.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        changed |= compact(ui, value, range.clone(), unit);
                    },
                );
            });
        });
        changed |= slider(ui, label, value, range).changed();
    });
    changed
}

pub fn slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f64,
    range: RangeInclusive<f64>,
) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(24.0), 24.0),
        egui::Sense::click_and_drag(),
    );
    let start = *range.start();
    let end = *range.end();
    let rail = rect.shrink2(egui::vec2(8.0, 10.0));
    let before = *value;
    if response.enabled() && start.is_finite() && end.is_finite() && start < end {
        if (response.clicked() || response.dragged())
            && let Some(pos) = response.interact_pointer_pos()
        {
            *value = (start
                + ((pos.x - rail.left()) / rail.width()).clamp(0.0, 1.0) as f64 * (end - start))
                .round()
                .clamp(start, end);
            response.request_focus();
        }
        if response.has_focus() {
            ui.input_mut(|i| {
                for (key, target) in [
                    (egui::Key::ArrowLeft, *value - 1.0),
                    (egui::Key::ArrowDown, *value - 1.0),
                    (egui::Key::ArrowRight, *value + 1.0),
                    (egui::Key::ArrowUp, *value + 1.0),
                    (egui::Key::Home, start),
                    (egui::Key::End, end),
                ] {
                    if i.consume_key(egui::Modifiers::NONE, key) {
                        *value = target.clamp(start, end);
                    }
                }
            });
        }
    }
    if *value != before {
        response.mark_changed();
    }
    response.widget_info(|| egui::WidgetInfo::slider(response.enabled(), *value, label));
    let fraction = if start < end && value.is_finite() {
        ((*value - start) / (end - start)).clamp(0.0, 1.0) as f32
    } else {
        0.0
    };
    let center = egui::pos2(rail.left() + rail.width() * fraction, rect.center().y);
    ui.painter().rect_filled(rail, 2, theme::CONTROL);
    let tint = if response.enabled() {
        theme::ACCENT
    } else {
        theme::DISABLED_TEXT
    };
    ui.painter().rect_filled(
        egui::Rect::from_min_max(rail.min, egui::pos2(center.x, rail.bottom())),
        2,
        tint.gamma_multiply(0.65),
    );
    ui.painter().circle_filled(
        center,
        7.0,
        if response.enabled() {
            theme::FIELD
        } else {
            theme::CONTROL_DISABLED
        },
    );
    ui.painter()
        .circle_stroke(center, 7.0, egui::Stroke::new(1.5_f32, tint));
    ui.painter().circle_filled(center, 2.5, tint);
    if response.enabled() && (response.hovered() || response.has_focus()) {
        ui.painter().circle_stroke(
            center,
            10.0,
            egui::Stroke::new(1.0_f32, theme::ACCENT.gamma_multiply(0.35)),
        );
    }
    response
}

#[cfg(test)]
mod tests;
