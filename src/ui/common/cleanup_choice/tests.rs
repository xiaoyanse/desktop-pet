use super::*;
use crate::domain::cleanup::Operation;

fn input(events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(840.0, 600.0),
        )),
        events,
        ..Default::default()
    }
}

fn pointer(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

fn label_center(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                Some(text.pos + text.galley.size() * 0.5)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing visible label: {label}"))
}

#[test]
fn profile_tiles_select_real_operations_and_disabled_tiles_do_not_change_them() {
    for enabled in [false, true] {
        for (profile, title) in Profile::ALL.into_iter().zip(["轻度", "中度", "重度"]) {
            let ctx = egui::Context::default();
            let mut config = CleanupConfig {
                operations: vec![Operation::Combine],
                ..Default::default()
            };
            let mut changed = false;
            let mut render = |ctx: &egui::Context| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_width(532.0);
                    changed |= profiles(ui, &mut config, enabled);
                });
            };
            let output = ctx.run(input(vec![]), &mut render);
            let pos = label_center(&output, title);
            for pressed in [true, false] {
                let _ = ctx.run(input(pointer(pos, pressed)), &mut render);
            }
            assert_eq!(changed, enabled);
            assert_eq!(
                config.operations,
                if enabled {
                    profile.operations()
                } else {
                    &[Operation::Combine]
                }
            );
        }
    }
}

#[test]
fn modal_cancel_backdrop_and_escape_never_produce_a_cleanup_request() {
    for dismiss in ["cancel", "backdrop", "escape"] {
        let ctx = egui::Context::default();
        let mut pending = Review::default();
        pending.begin(&CleanupConfig::default());
        let mut render = |ctx: &egui::Context| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("underlying page");
            });
            assert!(modal(ctx, &mut pending, true).is_none());
        };
        let mut output = ctx.run(input(vec![]), &mut render);
        for _ in 0..2 {
            output = ctx.run(input(vec![]), &mut render);
        }
        if dismiss == "escape" {
            let _ = ctx.run(
                input(vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }]),
                &mut render,
            );
        } else {
            let pos = if dismiss == "cancel" {
                label_center(&output, "取消")
            } else {
                egui::pos2(12.0, 12.0)
            };
            for pressed in [true, false] {
                let _ = ctx.run(input(pointer(pos, pressed)), &mut render);
            }
        }
        assert!(
            pending.operations().is_none(),
            "{dismiss} must close the review"
        );
    }
}

#[test]
fn modal_confirmation_emits_frozen_operations_once_and_fits_minimum_window() {
    let ctx = egui::Context::default();
    let mut pending = Review::default();
    let mut config = CleanupConfig {
        operations: Operation::ALL.to_vec(),
        ..Default::default()
    };
    pending.begin(&config);
    config.operations.clear();
    let mut requests = Vec::new();
    let mut render = |ctx: &egui::Context| {
        if let Some(request) = modal(ctx, &mut pending, true) {
            requests.push(request);
        }
    };
    let mut output = ctx.run(input(vec![]), &mut render);
    for _ in 0..2 {
        output = ctx.run(input(vec![]), &mut render);
    }
    for label in ["确认本次清理", "确认清理一次", "取消"] {
        let pos = label_center(&output, label);
        assert!((0.0..840.0).contains(&pos.x) && (0.0..600.0).contains(&pos.y));
    }
    let pos = label_center(&output, "确认清理一次");
    for pressed in [true, false] {
        let _ = ctx.run(input(pointer(pos, pressed)), &mut render);
    }
    let _ = ctx.run(input(vec![]), &mut render);
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].operations, Operation::ALL);
    assert!(pending.operations().is_none());
}
