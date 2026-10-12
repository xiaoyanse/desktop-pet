use super::*;

fn frame(
    ctx: &egui::Context,
    rgb: &mut [u8; 3],
    enabled: bool,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, egui::Response) {
    let mut response = None;
    let out = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(360.0, 450.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                response = Some(ui.add_enabled_ui(enabled, |ui| picker(ui, rgb)).inner);
            });
        },
    );
    (out, response.unwrap())
}
fn click(ctx: &egui::Context, rgb: &mut [u8; 3], enabled: bool, at: egui::Pos2) {
    for pressed in [true, false] {
        frame(
            ctx,
            rgb,
            enabled,
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
}
#[test]
fn hex_parser_accepts_exact_rgb_and_rejects_partial_or_unicode_without_panicking() {
    for s in ["#a5D5c8", "A5D5C8"] {
        assert_eq!(parse(s), Some([165, 213, 200]));
    }
    for s in [
        "#123",
        "中文",
        "#GG0000",
        "abcdef0",
        "１２３",
        "#12 456",
        "",
    ] {
        assert_eq!(parse(s), None);
    }
}
#[test]
fn palette_selects_preset_keeps_popup_open_and_escape_closes_it() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let mut rgb = [10, 20, 30];
    let (_, trigger) = frame(&ctx, &mut rgb, true, vec![]);
    click(&ctx, &mut rgb, true, trigger.rect.center());
    let popup = trigger.id.with("palette");
    assert!(ctx.memory(|m| m.is_popup_open(popup)));
    let (out, _) = frame(&ctx, &mut rgb, true, vec![]);
    let swatch = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Circle(c) if c.radius == 8.0 && c.center.y > trigger.rect.bottom() => {
                Some(c.center)
            }
            _ => None,
        })
        .expect("preset must be visible in the actual popup");
    click(&ctx, &mut rgb, true, swatch);
    assert_eq!(rgb, [165, 213, 200]);
    assert!(ctx.memory(|m| m.is_popup_open(popup)));
    frame(
        &ctx,
        &mut rgb,
        true,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(!ctx.memory(|m| m.is_popup_open(popup)));
}
#[test]
fn disabled_palette_neither_opens_nor_changes_color() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let mut rgb = [165, 213, 200];
    let (_, r) = frame(&ctx, &mut rgb, false, vec![]);
    click(&ctx, &mut rgb, false, r.rect.center());
    assert!(!ctx.memory(|m| m.is_popup_open(r.id.with("palette"))));
    assert_eq!(rgb, [165, 213, 200]);
}

#[test]
fn open_palette_closes_when_parent_becomes_disabled() {
    let ctx = egui::Context::default();
    let mut rgb = [165, 213, 200];
    let (_, r) = frame(&ctx, &mut rgb, true, vec![]);
    click(&ctx, &mut rgb, true, r.rect.center());
    assert!(ctx.memory(|m| m.is_popup_open(r.id.with("palette"))));
    frame(&ctx, &mut rgb, false, vec![]);
    assert!(!ctx.memory(|m| m.is_popup_open(r.id.with("palette"))));
}

#[test]
fn actual_hex_editor_applies_valid_color_and_preserves_color_on_invalid_input() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let mut rgb = [10, 20, 30];
    let (_, trigger) = frame(&ctx, &mut rgb, true, vec![]);
    click(&ctx, &mut rgb, true, trigger.rect.center());
    let (out, _) = frame(&ctx, &mut rgb, true, vec![]);
    let at = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t)
                if t.galley.text() == "#0A141E" && t.pos.y > trigger.rect.bottom() =>
            {
                Some(t.pos + t.galley.size() * 0.5)
            }
            _ => None,
        })
        .expect("actual HEX edit field must be visible");
    click(&ctx, &mut rgb, true, at);
    for (text, expected) in [("#112233", [17, 34, 51]), ("#GG0000", [17, 34, 51])] {
        frame(
            &ctx,
            &mut rgb,
            true,
            vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers {
                        ctrl: true,
                        command: true,
                        ..Default::default()
                    },
                },
                egui::Event::Text(text.into()),
            ],
        );
        assert_eq!(rgb, expected);
    }
    let (out, _) = frame(&ctx, &mut rgb, true, vec![]);
    assert!(out.shapes.iter().any(
        |s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "请输入 6 位十六进制颜色")
    ));
}
