use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

fn advance(
    ctx: &egui::Context,
    flow: &mut CaptureFlow,
    settings: &mut Settings,
    service: &mut ScreenshotService,
    now: Instant,
) {
    let _ = ctx.run(Default::default(), |ctx| {
        flow.tick(ctx, settings, service, now, false);
    });
}

fn finish(
    ctx: &egui::Context,
    flow: &mut CaptureFlow,
    settings: &mut Settings,
    service: &mut ScreenshotService,
    now: Instant,
) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while flow.hidden {
        advance(ctx, flow, settings, service, now);
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn every_direct_mode_copies_after_capture_without_opening_workbench() {
    for mode in Mode::ALL {
        let ctx = egui::Context::default();
        let mut settings = Settings::default();
        let mut flow = CaptureFlow::default();
        let (mut service, backend) = crate::application::screenshot::tests::fake_service();
        let now = Instant::now();
        let output = ctx.run(Default::default(), |ctx| {
            assert!(flow.begin(ctx, &mut settings, &service, mode, now));
            assert!(!flow.begin(ctx, &mut settings, &service, Mode::Fullscreen, now));
        });
        assert!(!settings.open && !service.busy());
        assert!(
            !output
                .viewport_output
                .contains_key(&egui::ViewportId::from_hash_of("settings"))
        );
        assert!(
            !output
                .viewport_output
                .contains_key(&egui::ViewportId::from_hash_of("pinned-screenshot"))
        );
        assert_eq!(flow.pending.unwrap().0, mode);
        advance(
            &ctx,
            &mut flow,
            &mut settings,
            &mut service,
            now + Duration::from_millis(199),
        );
        assert_eq!(backend.calls.load(Ordering::SeqCst), 0);
        assert!(!settings.open);
        finish(
            &ctx,
            &mut flow,
            &mut settings,
            &mut service,
            now + Duration::from_millis(201),
        );
        assert_eq!(backend.calls.load(Ordering::SeqCst), 1);
        assert!(service.document.is_none() && !settings.open);
        assert_eq!(
            backend.copied.lock().unwrap().as_ref().unwrap(),
            &vec![255; 8 * 8 * 4]
        );
    }
}

#[test]
fn copy_restores_the_previous_page_and_keeps_annotation_only_when_already_open() {
    for edit in [false, true] {
        let ctx = egui::Context::default();
        let mut settings = Settings::default();
        if edit {
            settings.open_screenshot();
        } else {
            settings.open_meeting();
        }
        let mut flow = CaptureFlow::default();
        let (mut service, backend) = crate::application::screenshot::tests::fake_service();
        let now = Instant::now();
        let _ = ctx.run(Default::default(), |ctx| {
            assert!(flow.begin(ctx, &mut settings, &service, Mode::Rectangle, now));
        });
        finish(
            &ctx,
            &mut flow,
            &mut settings,
            &mut service,
            now + Duration::from_millis(201),
        );
        assert!(settings.open);
        assert_eq!(settings.screenshot_visible(), edit);
        assert_eq!(settings.meeting_visible(), !edit);
        assert_eq!(service.document.is_some(), edit);
        assert!(backend.copied.lock().unwrap().is_some());
    }
}

struct CancelSelection;
impl crate::interfaces::ScreenCapture for CancelSelection {
    fn bounds(&self) -> Result<crate::domain::screenshot::Region, String> {
        panic!("cancel never reads the desktop")
    }
    fn select(
        &self,
        _: Mode,
        _: &AtomicBool,
    ) -> Result<Option<crate::domain::screenshot::Selection>, String> {
        Ok(None)
    }
    fn capture(
        &self,
        _: crate::domain::screenshot::Region,
        _: &AtomicBool,
    ) -> Result<crate::domain::screenshot::Image, String> {
        panic!("cancel never captures")
    }
}

#[test]
fn cancellation_or_failure_returns_to_pet_or_restores_the_previously_open_page() {
    for was_open in [false, true] {
        for cancelled in [false, true] {
            let ctx = egui::Context::default();
            let mut settings = Settings::default();
            if was_open {
                settings.open_meeting();
            }
            let mut flow = CaptureFlow::default();
            let (mut service, backend) = crate::application::screenshot::tests::fake_service();
            *backend.copied.lock().unwrap() = Some(vec![12, 23, 34, 255]);
            if cancelled {
                service = ScreenshotService::new(
                    Arc::new(CancelSelection),
                    backend.clone(),
                    Arc::new(|| {}),
                );
            } else {
                backend.fail.store(true, Ordering::SeqCst);
            }
            let now = Instant::now();
            let _ = ctx.run(Default::default(), |ctx| {
                assert!(flow.begin(ctx, &mut settings, &service, Mode::Freehand, now));
            });
            assert!(!settings.open);
            finish(
                &ctx,
                &mut flow,
                &mut settings,
                &mut service,
                now + Duration::from_millis(201),
            );
            assert_eq!(settings.open, was_open);
            assert_eq!(settings.meeting_visible(), was_open);
            assert!(service.document.is_none());
            assert_eq!(
                backend.copied.lock().unwrap().as_deref(),
                Some(&[12, 23, 34, 255][..])
            );
        }
    }
}

#[test]
fn interrupting_pending_capture_does_not_capture_or_open_a_window() {
    let ctx = egui::Context::default();
    let mut flow = CaptureFlow::default();
    let mut settings = Settings::default();
    let (mut service, backend) = crate::application::screenshot::tests::fake_service();
    let now = Instant::now();
    let _ = ctx.run(Default::default(), |ctx| {
        flow.begin(ctx, &mut settings, &service, Mode::Rectangle, now);
    });
    flow.interrupt();
    advance(
        &ctx,
        &mut flow,
        &mut settings,
        &mut service,
        now + Duration::from_secs(1),
    );
    assert!(!flow.hidden && !settings.open && !service.busy());
    assert_eq!(backend.calls.load(Ordering::SeqCst), 0);
}

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
