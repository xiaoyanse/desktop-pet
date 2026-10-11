use super::{
    icons::{self, Icon},
    theme,
};
use eframe::egui::{self, Color32, RichText};
use std::ops::RangeInclusive;

pub fn switch(ui: &mut egui::Ui, value: &mut bool, label: &str) -> egui::Response {
    let color = if ui.is_enabled() {
        theme::TEXT
    } else {
        theme::MUTED.gamma_multiply(0.5)
    };
    let text = ui.painter().layout(
        label.to_owned(),
        egui::FontId::proportional(13.0),
        color,
        (ui.available_width() - 46.0).max(1.0),
    );
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(
            (text.size().x + 46.0).min(ui.available_width()),
            text.size().y.max(24.0) + 6.0,
        ),
        egui::Sense::hover(),
    );
    let mut response = ui.interact(rect, ui.id().with(("switch", label)), egui::Sense::click());
    if response.clicked() && ui.is_enabled() {
        *value = !*value;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *value, label)
    });
    let track = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 16.0, rect.center().y),
        egui::vec2(32.0, 18.0),
    );
    let fill = if *value {
        theme::ACCENT
    } else {
        Color32::from_rgb(67, 78, 81)
    };
    ui.painter().rect_filled(
        track,
        9,
        if ui.is_enabled() {
            fill
        } else {
            fill.gamma_multiply(0.4)
        },
    );
    if response.hovered() || response.has_focus() {
        ui.painter().rect_stroke(
            track.expand(3.0),
            12,
            egui::Stroke::new(1.0_f32, theme::ACCENT),
            egui::StrokeKind::Outside,
        );
    }
    let progress = ui.ctx().animate_bool_with_time(response.id, *value, 0.08);
    ui.painter().circle_filled(
        egui::pos2(track.left() + 9.0 + 14.0 * progress, track.center().y),
        6.5,
        if ui.is_enabled() {
            Color32::from_rgb(245, 248, 246)
        } else {
            theme::MUTED
        },
    );
    ui.painter().galley(
        egui::pos2(rect.left() + 44.0, rect.center().y - text.size().y * 0.5),
        text,
        color,
    );
    response
}

pub fn choice(ui: &mut egui::Ui, icon: Icon, label: &str, selected: bool) -> egui::Response {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 38.0), egui::Sense::hover());
    let response = ui.interact(rect, ui.id().with(("choice", label)), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    ui.painter().rect_filled(
        rect,
        7,
        if selected {
            Color32::from_rgb(32, 59, 50)
        } else {
            theme::BACKGROUND
        },
    );
    if response.hovered() || response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            7,
            egui::Stroke::new(1.0_f32, theme::ACCENT),
            egui::StrokeKind::Inside,
        );
    }
    let color = if selected {
        theme::ACCENT
    } else {
        theme::MUTED
    };
    icons::draw(
        ui.painter(),
        egui::Rect::from_center_size(
            rect.left_center() + egui::vec2(20.0, 0.0),
            egui::vec2(16.0, 16.0),
        ),
        icon,
        color,
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(40.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        theme::TEXT,
    );
    if selected {
        icons::draw(
            ui.painter(),
            egui::Rect::from_center_size(
                rect.right_center() - egui::vec2(20.0, 0.0),
                egui::vec2(14.0, 14.0),
            ),
            Icon::Check,
            color,
        );
    }
    response
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
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).size(13.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let plus = ui
                    .add_enabled(
                        *value < *range.end(),
                        egui::Button::new("+")
                            .corner_radius(6)
                            .min_size(egui::vec2(30.0, 30.0)),
                    )
                    .on_hover_text("增加 1");
                if plus.clicked() {
                    *value = (*value + 1.0).min(*range.end());
                    changed = true;
                }
                changed |= ui
                    .add(
                        egui::DragValue::new(value)
                            .range(range.clone())
                            .speed(1.0)
                            .fixed_decimals(0)
                            .suffix(unit),
                    )
                    .changed();
                let minus = ui
                    .add_enabled(
                        *value > *range.start(),
                        egui::Button::new("−")
                            .corner_radius(6)
                            .min_size(egui::vec2(30.0, 30.0)),
                    )
                    .on_hover_text("减少 1");
                if minus.clicked() {
                    *value = (*value - 1.0).max(*range.start());
                    changed = true;
                }
            });
        });
        ui.scope(|ui| {
            ui.spacing_mut().slider_width = ui.available_width();
            changed |= ui
                .add(
                    egui::Slider::new(value, range)
                        .show_value(false)
                        .step_by(1.0),
                )
                .changed();
        });
    });
    changed
}

pub fn integer(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut u64,
    range: RangeInclusive<u64>,
    unit: &str,
) -> bool {
    let mut edit = *value as f64;
    let changed = number(
        ui,
        label,
        &mut edit,
        *range.start() as f64..=*range.end() as f64,
        unit,
    );
    if changed {
        *value = edit.round() as u64;
    }
    changed
}

pub fn tab(ui: &mut egui::Ui, label: &str, selected: bool, width: f32) -> egui::Response {
    ui.add_sized(
        [width, 34.0],
        egui::Button::new(RichText::new(label).size(12.0).color(if selected {
            theme::ACCENT
        } else {
            theme::MUTED
        }))
        .fill(if selected {
            Color32::from_rgb(30, 53, 46)
        } else {
            theme::BACKGROUND
        })
        .stroke(egui::Stroke::new(
            1.0_f32,
            if selected {
                theme::ACCENT.gamma_multiply(0.6)
            } else {
                theme::LINE
            },
        ))
        .corner_radius(7),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number_frame(
        ctx: &egui::Context,
        value: &mut f64,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    number(ui, "Threshold", value, 0.0..=6.0, "%");
                });
            },
        )
    }

    fn click_number(ctx: &egui::Context, value: &mut f64, symbol: &str) {
        let output = number_frame(ctx, value, Vec::new());
        let point = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == symbol => {
                    Some(text.pos + text.galley.size() * 0.5)
                }
                _ => None,
            })
            .expect("numeric button must be visible");
        for pressed in [true, false] {
            number_frame(
                ctx,
                value,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    }

    #[test]
    fn numeric_buttons_follow_bounds_and_do_not_wrap() {
        let ctx = egui::Context::default();
        let mut value = 5.0;
        click_number(&ctx, &mut value, "+");
        assert_eq!(value, 6.0);
        click_number(&ctx, &mut value, "+");
        assert_eq!(value, 6.0);
        click_number(&ctx, &mut value, "−");
        assert_eq!(value, 5.0);
        value = 0.0;
        click_number(&ctx, &mut value, "−");
        assert_eq!(value, 0.0);
    }

    #[test]
    fn rendering_integer_control_preserves_long_cooldown() {
        let ctx = egui::Context::default();
        let mut value = 120;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                assert!(!integer(ui, "Cooldown", &mut value, 1..=1440, " min"));
            });
        });
        assert_eq!(value, 120);
    }

    #[test]
    fn focused_switch_supports_keyboard_activation() {
        for key in [egui::Key::Space, egui::Key::Enter] {
            let ctx = egui::Context::default();
            let mut value = false;
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    switch(ui, &mut value, "Background").request_focus();
                });
            });
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        switch(ui, &mut value, "Background");
                    });
                },
            );
            assert!(value);
        }
    }
    #[test]
    fn switch_label_is_clickable_and_disabled_control_does_not_change() {
        for enabled in [true, false] {
            let ctx = egui::Context::default();
            let mut value = false;
            let mut point = egui::Pos2::ZERO;
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let r = ui
                        .add_enabled_ui(enabled, |ui| switch(ui, &mut value, "背景"))
                        .inner;
                    point = r.rect.right_center() - egui::vec2(2.0, 0.0);
                    assert!(r.rect.height() >= 30.0);
                    assert!(!r.changed());
                });
            });
            for pressed in [true, false] {
                let input = egui::RawInput {
                    events: vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..Default::default()
                };
                let _ = ctx.run(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.add_enabled_ui(enabled, |ui| switch(ui, &mut value, "背景"));
                    });
                });
            }
            assert_eq!(value, enabled);
        }
    }
}
