use super::{
    components::{numeric, selection},
    theme,
};
use eframe::egui::{self, Color32};
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
        theme::PROGRESS_TRACK
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

pub fn number(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f64,
    range: RangeInclusive<f64>,
    unit: &str,
) -> bool {
    numeric::number(ui, label, value, range, unit)
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
    selection::choice(ui, label, selected, width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::common::icons::Icon;

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

    fn click_number(ctx: &egui::Context, value: &mut f64, icon: Icon) {
        let output = number_frame(ctx, value, Vec::new());
        let point = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh)
                    if mesh.texture_id
                        == super::super::artwork::texture(ctx, icon.asset()).id() =>
                {
                    Some(mesh.calc_bounds().center())
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
        click_number(&ctx, &mut value, Icon::Plus);
        assert_eq!(value, 6.0);
        click_number(&ctx, &mut value, Icon::Plus);
        assert_eq!(value, 6.0);
        click_number(&ctx, &mut value, Icon::Minus);
        assert_eq!(value, 5.0);
        value = 0.0;
        click_number(&ctx, &mut value, Icon::Minus);
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
