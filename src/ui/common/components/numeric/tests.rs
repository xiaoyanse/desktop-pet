use super::*;

fn frame(
    ctx: &egui::Context,
    value: &mut f64,
    enabled: bool,
    events: Vec<egui::Event>,
) -> egui::Response {
    let mut response = None;
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(320.0, 240.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                response = Some(
                    ui.add_enabled_ui(enabled, |ui| slider(ui, "阈值", value, 10.0..=90.0))
                        .inner,
                );
            });
        },
    );
    response.unwrap()
}
fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}
#[test]
fn slider_click_drag_keyboard_and_changed_contract_follow_bounds() {
    let ctx = egui::Context::default();
    let mut value = 20.0;
    let r = frame(&ctx, &mut value, true, vec![]);
    assert!(!r.changed());
    let at = r.rect.center();
    for pressed in [true, false] {
        frame(
            &ctx,
            &mut value,
            true,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert_eq!(value, 50.0);
    assert!(frame(&ctx, &mut value, true, vec![key(egui::Key::ArrowRight)]).changed());
    assert_eq!(value, 51.0);
    frame(&ctx, &mut value, true, vec![key(egui::Key::End)]);
    assert_eq!(value, 90.0);
    assert!(!frame(&ctx, &mut value, true, vec![key(egui::Key::ArrowRight)]).changed());
    frame(&ctx, &mut value, true, vec![key(egui::Key::Home)]);
    assert_eq!(value, 10.0);
    assert!(!frame(&ctx, &mut value, true, vec![key(egui::Key::ArrowLeft)]).changed());
    let left = r.rect.left_center();
    frame(
        &ctx,
        &mut value,
        true,
        vec![
            egui::Event::PointerMoved(left),
            egui::Event::PointerButton {
                pos: left,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let outside = r.rect.right_center() + egui::vec2(50.0, 0.0);
    frame(
        &ctx,
        &mut value,
        true,
        vec![egui::Event::PointerMoved(outside)],
    );
    assert_eq!(value, 90.0, "drag beyond rail must clamp to endpoint");
}
#[test]
fn disabled_slider_and_degenerate_range_are_inert() {
    let ctx = egui::Context::default();
    let mut value = 40.0;
    let r = frame(&ctx, &mut value, false, vec![]);
    for pressed in [true, false] {
        frame(
            &ctx,
            &mut value,
            false,
            vec![
                egui::Event::PointerMoved(r.rect.center()),
                egui::Event::PointerButton {
                    pos: r.rect.center(),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
                key(egui::Key::End),
            ],
        );
    }
    assert_eq!(value, 40.0);
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            assert!(!slider(ui, "固定值", &mut value, 40.0..=40.0).changed());
        });
    });
    assert_eq!(value, 40.0);
}

#[test]
fn native_numeric_entry_retains_text_editing_and_clamps_typed_value() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let mut value = 12.0;
    let render = |value: &mut f64, events| {
        ctx.run(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    compact(ui, value, 0.0..=255.0, "");
                });
            },
        )
    };
    let out = render(&mut value, vec![]);
    let at = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == "12" => Some(t.pos + t.galley.size() * 0.5),
            _ => None,
        })
        .expect("numeric value must be visible");
    for pressed in [true, false] {
        render(
            &mut value,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    render(&mut value, vec![egui::Event::Text("999".into())]);
    assert_eq!(value, 255.0);
}
