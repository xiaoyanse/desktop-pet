use super::*;

#[test]
fn suspended_root_keeps_render_loop_visible_and_passes_mouse_through() {
    let ctx = egui::Context::default();
    let output = ctx.run(egui::RawInput::default(), suspend_root);
    let root = output.viewport_output.get(&egui::ViewportId::ROOT).unwrap();
    assert!(
        root.commands
            .iter()
            .any(|c| matches!(c, egui::ViewportCommand::MousePassthrough(true)))
    );
    assert!(
        !root
            .commands
            .iter()
            .any(|c| matches!(c, egui::ViewportCommand::Visible(false)))
    );
    assert!(output.shapes.is_empty());
}

#[test]
fn restoring_root_preserves_both_prior_mouse_modes() {
    let ctx = egui::Context::default();
    for passthrough in [false, true] {
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            restore_root(ctx, passthrough)
        });
        let root = output.viewport_output.get(&egui::ViewportId::ROOT).unwrap();
        assert!(root.commands.iter().any(
            |c| matches!(c,egui::ViewportCommand::MousePassthrough(value) if *value==passthrough)
        ));
    }
}
