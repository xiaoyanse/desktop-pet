use super::{fields, selection, status, surface};
use crate::ui::common::{icons::Icon, theme};
use eframe::egui;

fn click_events(point: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(point),
        egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

fn input(events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(360.0, 400.0),
        )),
        events,
        ..Default::default()
    }
}

fn dropdown_frame(
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    selected: &mut bool,
) -> (egui::FullOutput, egui::Pos2) {
    let mut center = egui::Pos2::ZERO;
    let output = ctx.run(input(events), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            center = selection::dropdown(
                ui,
                "test-device",
                if *selected {
                    "第二块显卡"
                } else {
                    "自动选择"
                },
                |ui| {
                    if selection::choice(ui, "第二块显卡", *selected, ui.available_width())
                        .clicked()
                    {
                        *selected = true;
                    }
                },
            )
            .response
            .rect
            .center();
        });
    });
    (output, center)
}

#[test]
fn themed_dropdown_selects_and_closes_using_real_pointer_input() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let mut selected = false;
    let (_, at) = dropdown_frame(&ctx, vec![], &mut selected);
    for pressed in [true, false] {
        dropdown_frame(&ctx, click_events(at, pressed), &mut selected);
    }
    let (out, _) = dropdown_frame(&ctx, vec![], &mut selected);
    let option = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == "第二块显卡" => {
                Some(t.pos + t.galley.size() * 0.5)
            }
            _ => None,
        })
        .expect("popup option must be visible");
    for pressed in [true, false] {
        dropdown_frame(&ctx, click_events(option, pressed), &mut selected);
    }
    assert!(selected);
    let (out, _) = dropdown_frame(&ctx, vec![], &mut selected);
    assert_eq!(
        out.shapes
            .iter()
            .filter(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "第二块显卡"))
            .count(),
        1,
        "selected label only; popup should close"
    );
}

#[test]
fn fields_keep_chinese_input_limit_focus_and_error_edge() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let mut text = String::new();
    let render = |ui: &mut egui::Ui, text: &mut String| {
        fields::entry(
            ui,
            egui::TextEdit::singleline(text)
                .char_limit(4)
                .desired_width(ui.available_width()),
            true,
        )
    };
    let _ = ctx.run(input(vec![]), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            render(ui, &mut text).request_focus();
        });
    });
    let output = ctx.run(
        input(vec![egui::Event::Text("中文测试更多".into())]),
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let r = render(ui, &mut text);
                assert!(r.has_focus());
                assert!(r.rect.height() >= theme::CONTROL_HEIGHT);
                assert!(ui.clip_rect().contains_rect(r.rect));
            });
        },
    );
    assert_eq!(text, "中文测试");
    assert!(
        output
            .shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.stroke.color == theme::DANGER))
    );
}

#[test]
fn real_pointer_hover_press_and_keyboard_focus_have_distinct_feedback() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let mut center = egui::Pos2::ZERO;
    let _ = ctx.run(input(vec![]), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            center = selection::choice(ui, "待办", false, 100.0).rect.center();
        });
    });
    for (events, expected) in [
        (vec![egui::Event::PointerMoved(center)], theme::HOVER),
        (
            vec![
                egui::Event::PointerMoved(center),
                egui::Event::PointerButton {
                    pos: center,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            theme::CONTROL_PRESSED,
        ),
    ] {
        let out = ctx.run(input(events), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                selection::choice(ui, "待办", false, 100.0);
            });
        });
        assert!(
            out.shapes
                .iter()
                .any(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.fill == expected))
        );
    }
    let _ = ctx.run(
        input(vec![egui::Event::PointerButton {
            pos: center,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]),
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                selection::choice(ui, "待办", false, 100.0).request_focus();
            });
        },
    );
    let out = ctx.run(input(vec![egui::Event::PointerGone]), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let r = selection::choice(ui, "待办", false, 100.0);
            assert!(r.has_focus());
        });
    });
    assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.stroke.color == theme::ACCENT && r.stroke.width == 1.5)));
}

#[test]
fn checkbox_keyboard_disabled_and_changed_contract() {
    for enabled in [true, false] {
        let ctx = egui::Context::default();
        theme::setup(&ctx);
        let mut value = false;
        let _ = ctx.run(input(vec![]), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    selection::checkbox(ui, &mut value, "完成任务").request_focus()
                });
            });
        });
        let _ = ctx.run(
            input(vec![egui::Event::Key {
                key: egui::Key::Space,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }]),
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let r = ui
                        .add_enabled_ui(enabled, |ui| {
                            selection::checkbox(ui, &mut value, "完成任务")
                        })
                        .inner;
                    assert_eq!(r.changed(), enabled);
                });
            },
        );
        assert_eq!(value, enabled);
    }
}

#[test]
fn shared_states_and_status_use_theme_tokens() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let out = ctx.run(input(vec![]), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let r = selection::choice(ui, "待办", true, 100.0);
            assert_eq!(r.rect.height(), theme::CONTROL_HEIGHT);
            ui.add_enabled_ui(false, |ui| {
                let (_, r) = ui.allocate_exact_size(egui::vec2(80.0, 36.0), egui::Sense::click());
                surface::paint(ui, &r, surface::Role::Primary, theme::CONTROL_RADIUS);
                assert_eq!(surface::foreground(&r), theme::DISABLED_TEXT);
            });
            status::badge(ui, "已到期", status::Tone::Error);
            crate::ui::common::actions::danger(ui, Icon::Cancel, "删除条目", true);
        });
    });
    for fill in [theme::CONTROL_SELECTED, theme::DANGER_SURFACE] {
        assert!(
            out.shapes
                .iter()
                .any(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.fill == fill)),
            "missing {fill:?}"
        );
    }
    assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.rect.width() == 80.0 && r.fill.r() < theme::CONTROL.r() && r.fill.g() < theme::CONTROL.g())));
}
