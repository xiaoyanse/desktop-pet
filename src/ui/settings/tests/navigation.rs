use super::super::Settings;
use crate::ui::companion::flyout::Action;
use crate::{domain::config::Config, ui::companion::flyout::tests::services};
use eframe::egui;

#[test]
fn every_pet_tool_selects_its_real_page_and_requests_window_reveal() {
    let mut settings = Settings::default();
    for (action, tab) in [
        (Action::Screenshot, 8),
        (Action::Report, 7),
        (Action::Timer, 6),
        (Action::Notes, 9),
        (Action::Tasks, 9),
        (Action::Meeting, 10),
        (Action::Clipboard, 11),
        (Action::AdvancedClean, 4),
    ] {
        settings.open = false;
        settings.reveal_requested = false;
        assert!(settings.open_from_pet(action));
        assert_eq!(settings.tab, tab, "wrong page for {action:?}");
        assert!(settings.open && settings.reveal_requested);
    }
    assert!(settings.open_from_pet(Action::Settings));
    assert!(settings.open && settings.reveal_requested);
    settings.open = false;
    settings.reveal_requested = false;
    for action in [
        Action::Restore,
        Action::Compact,
        Action::Capture(crate::domain::screenshot::Mode::Freehand),
    ] {
        assert!(!settings.open_from_pet(action));
        assert!(!settings.open && !settings.reveal_requested);
    }
}

#[test]
fn opening_screenshot_restores_a_minimized_workbench_once() {
    let ctx = egui::Context::default();
    crate::ui::common::theme::setup(&ctx);
    ctx.set_embed_viewports(true);
    let mut settings = Settings {
        open: true,
        ..Default::default()
    };
    assert!(settings.open_from_pet(Action::Screenshot));
    assert_eq!(settings.tab, 8);
    let mut config = Config::default();
    let mut services = services();
    let paint = |settings: &mut Settings, config: &mut Config, services: &mut _| {
        ctx.run(Default::default(), |ctx| {
            settings.show(
                ctx,
                config,
                &Default::default(),
                &mut String::new(),
                services,
            );
        })
    };
    let output = paint(&mut settings, &mut config, &mut services);
    let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
    assert!(commands.contains(&egui::ViewportCommand::Minimized(false)));
    assert!(commands.contains(&egui::ViewportCommand::Focus));
    let output = paint(&mut settings, &mut config, &mut services);
    assert!(
        !output.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&egui::ViewportCommand::Focus)
    );
    assert!(!services.screenshot.busy());
    assert!(services.report.document.is_none());
}
