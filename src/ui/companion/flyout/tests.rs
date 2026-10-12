use super::{
    Action,
    render::{self, Content},
};
use crate::{
    application::{Services, cleanup::CleanupService},
    domain::{
        cleanup::{CleanupReport, MemoryStatus, Operation},
        cleanup_profile::{Profile, Review},
        companion::spiral::{Group, Layout, MenuState},
        config::Config,
        desktop::DesktopGeometry,
        resources::Snapshot,
        work_timer::State,
    },
    interfaces::{
        CleanupHistory, DesktopAction, DesktopSession, MemoryCleaner, SettingsStore, Wake,
    },
};
use eframe::egui;
use std::{
    sync::{Arc, mpsc::Receiver},
    time::Duration,
};
struct Memory;
impl crate::interfaces::ReportCollector for Memory {
    fn collect(
        &self,
        _: &Config,
        _: &std::sync::atomic::AtomicBool,
    ) -> Result<crate::domain::report::Report, String> {
        panic!("opening a tool must not automatically collect a report")
    }
}
impl crate::interfaces::ReportStore for Memory {
    fn save(&self, _: &str) -> Result<String, String> {
        panic!("not exercised")
    }
}
impl SettingsStore for Memory {
    fn load(&self) -> (Config, Option<String>) {
        (Config::default(), None)
    }
    fn save(&self, _: &Config) -> Result<(), String> {
        Ok(())
    }
}
impl CleanupHistory for Memory {
    fn load(&self) -> Result<Vec<CleanupReport>, String> {
        Ok(vec![])
    }
    fn save(&self, _: &[CleanupReport]) -> Result<(), String> {
        Ok(())
    }
}
impl MemoryCleaner for Memory {
    fn request(
        &self,
        _: Vec<Operation>,
        _: bool,
        _: Wake,
    ) -> Receiver<Result<CleanupReport, String>> {
        panic!("UI review must not execute cleanup")
    }
    fn elevated(&self) -> bool {
        false
    }
    fn inspect(&self) -> Result<MemoryStatus, String> {
        Err("not sampled".into())
    }
}
struct Desktop;
impl DesktopSession for Desktop {
    fn configure_card(&self, _: bool, _: bool, _: f32, _: f32) {}
    fn geometry(&self) -> Option<DesktopGeometry> {
        None
    }
    fn cursor(&self) -> Option<[f32; 2]> {
        None
    }
    fn configure_auxiliary(&self) {}
    fn quick_regions(&self, _: &[[f32; 4]], _: bool) -> Result<(), String> {
        Ok(())
    }
    fn actions(&self) -> Vec<DesktopAction> {
        vec![]
    }
    fn set_snapping(&self, _: bool) {}
    fn tooltip(&mut self, _: &str, _: bool) {}
    fn menu(&self, _: bool) -> i32 {
        0
    }
    fn autostart(&self) -> bool {
        false
    }
    fn set_autostart(&self, _: bool) -> Result<(), String> {
        panic!("not exercised")
    }
    fn set_clean_hotkey(&self, _: bool) -> Result<(), String> {
        panic!("not exercised")
    }
}
fn services() -> Services {
    Services {
        settings: Box::new(Memory),
        desktop: Box::new(Desktop),
        cleaner: CleanupService::new(Box::new(Memory), Box::new(Memory), Arc::new(|| {})),
        timer: Default::default(),
        report: crate::application::report::ReportService::new(
            Arc::new(Memory),
            Arc::new(Memory),
            Arc::new(|| {}),
        ),
    }
}
fn paint(
    ctx: &egui::Context,
    content: &mut Content<'_>,
    state: &MenuState,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, render::Output) {
    let layout = Layout::new(DesktopGeometry {
        frame: [0.0, 210.0, 72.0, 294.0],
        work_area: [0.0, 0.0, 1000.0, 800.0],
    });
    let mut result = render::Output::default();
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(520.0, 460.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| {
                    result =
                        render::show(ui, &layout, state, Duration::from_secs(1), content, false);
                });
        },
    );
    (output, result)
}
fn text_bounds(shape: &egui::Shape, label: &str) -> Option<egui::Rect> {
    match shape {
        egui::Shape::Text(t) if t.galley.text() == label => {
            Some(egui::Rect::from_min_size(t.pos, t.galley.size()))
        }
        egui::Shape::Vec(v) => v.iter().find_map(|s| text_bounds(s, label)),
        _ => None,
    }
}
fn click(
    ctx: &egui::Context,
    content: &mut Content<'_>,
    state: &MenuState,
    label: &str,
) -> render::Output {
    let (mut output, mut regions) = paint(ctx, content, state, vec![]);
    for _ in 0..2 {
        (output, regions) = paint(ctx, content, state, vec![]);
    }
    let point = output
        .shapes
        .iter()
        .find_map(|s| {
            text_bounds(&s.shape, label).map(|r| {
                assert!(s.clip_rect.contains_rect(r), "clipped action: {label}");
                assert!(
                    regions
                        .regions
                        .iter()
                        .any(|raw| render::rect(*raw).contains_rect(r)),
                    "outside native window region: {label}"
                );
                r.center()
            })
        })
        .unwrap_or_else(|| panic!("missing {label}"));
    let mut result = render::Output::default();
    for pressed in [true, false] {
        result = paint(
            ctx,
            content,
            state,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        )
        .1;
    }
    result
}
#[test]
fn cleanup_uses_current_list_and_cancellation_never_executes() {
    let ctx = egui::Context::default();
    crate::ui::common::theme::setup(&ctx);
    let mut config = Config::default();
    config.cleanup.operations = Profile::Heavy.operations().to_vec();
    let mut services = services();
    let mut review = Review::default();
    let snapshot = Snapshot::default();
    let mut state = MenuState::default();
    state.open(Duration::ZERO);
    state.hover(Duration::ZERO, Some(Group::Resources), true);
    let mut content = Content {
        config: &config,
        snapshot: &snapshot,
        services: &mut services,
        review: &mut review,
    };
    let result = click(&ctx, &mut content, &state, "检查清单 · 4 项");
    assert!(result.action.is_none());
    assert_eq!(
        content.review.operations(),
        Some(Profile::Heavy.operations())
    );
    click(&ctx, &mut content, &state, "取消");
    assert!(content.review.operations().is_none());
    click(&ctx, &mut content, &state, "日常轻清 · 1 项");
    assert_eq!(
        content.review.operations(),
        Some(Profile::Light.operations())
    );
    click(&ctx, &mut content, &state, "取消");
    assert_eq!(config.cleanup.operations, Profile::Heavy.operations());
}
#[test]
fn office_controls_reuse_real_timer_and_categories_do_not_activate_other_tools() {
    let ctx = egui::Context::default();
    crate::ui::common::theme::setup(&ctx);
    let config = Config::default();
    let snapshot = Snapshot::default();
    let mut services = services();
    let mut review = Review::default();
    let mut state = MenuState::default();
    state.open(Duration::ZERO);
    state.hover(Duration::ZERO, Some(Group::Office), true);
    let mut content = Content {
        config: &config,
        snapshot: &snapshot,
        services: &mut services,
        review: &mut review,
    };
    click(&ctx, &mut content, &state, "开始计时");
    assert_eq!(content.services.timer.session.state, State::Running);
    click(&ctx, &mut content, &state, "暂停计时");
    assert_eq!(content.services.timer.session.state, State::Paused);
    click(&ctx, &mut content, &state, "继续计时");
    assert_eq!(content.services.timer.session.state, State::Running);
    assert_eq!(
        click(&ctx, &mut content, &state, "详细计时").action,
        Some(Action::Timer)
    );
    let result = click(&ctx, &mut content, &state, "工具");
    assert_eq!(result.group, Some(Group::Tools));
    assert!(result.action.is_none());
    state.hover(Duration::ZERO, Some(Group::Tools), true);
    assert_eq!(
        click(&ctx, &mut content, &state, "打开诊断报告").action,
        Some(Action::Report)
    );
}
