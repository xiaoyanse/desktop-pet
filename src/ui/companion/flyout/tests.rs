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
pub(super) fn services() -> Services {
    Services {
        office: crate::application::office::tests::fake_service().0,
        screenshot: crate::application::screenshot::tests::fake_service().0,
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
fn paint_layout(
    ctx: &egui::Context,
    content: &mut Content<'_>,
    state: &MenuState,
    events: Vec<egui::Event>,
    layout: &Layout,
    now: Duration,
) -> (egui::FullOutput, render::Output) {
    let mut result = render::Output::default();
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(layout.size[0], layout.size[1]),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| {
                    result = render::show(ui, layout, state, now, content, false);
                });
        },
    );
    (output, result)
}
#[test]
fn edge_spiral_keeps_rendered_regions_and_action_labels_inside_its_viewport() {
    let area = [0.0, 0.0, 1920.0, 1032.0];
    for frame in [
        [0.0, 0.0, 72.0, 84.0],
        [1848.0, 0.0, 1920.0, 84.0],
        [0.0, 948.0, 72.0, 1032.0],
        [1848.0, 948.0, 1920.0, 1032.0],
    ] {
        for group in Group::ALL {
            let ctx = egui::Context::default();
            crate::ui::common::theme::setup(&ctx);
            let config = Config::default();
            let snapshot = Snapshot::default();
            let mut services = services();
            let mut review = Review::default();
            let mut state = MenuState::default();
            state.open(Duration::ZERO);
            state.hover(Duration::ZERO, Some(group), true);
            let mut content = Content {
                config: &config,
                snapshot: &snapshot,
                services: &mut services,
                review: &mut review,
            };
            let layout = Layout::new(DesktopGeometry {
                frame,
                work_area: area,
            });
            let viewport = egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(layout.size[0], layout.size[1]),
            );
            for milliseconds in [0, 35, 70, 120, 180, 250, 1000] {
                let (output, rendered) = paint_layout(
                    &ctx,
                    &mut content,
                    &state,
                    vec![],
                    &layout,
                    Duration::from_millis(milliseconds),
                );
                for raw in rendered.regions {
                    assert!(
                        viewport.expand(0.1).contains_rect(render::rect(raw)),
                        "region outside viewport: frame={frame:?}, group={group:?}, ms={milliseconds}, region={raw:?}"
                    );
                }
                for label in [
                    "完整",
                    "精简",
                    "检查清单 · 1 项",
                    "开始计时",
                    "详细计时",
                    "打开诊断报告",
                    "打开设置",
                    "便签",
                    "待办",
                ] {
                    for shape in &output.shapes {
                        if let Some(rect) = text_bounds(&shape.shape, label) {
                            assert!(
                                viewport.contains_rect(rect) && shape.clip_rect.contains_rect(rect),
                                "action outside clip: frame={frame:?}, group={group:?}, ms={milliseconds}, label={label}, rect={rect:?}"
                            );
                        }
                    }
                }
            }
        }
    }
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
    let layout = Layout::new(DesktopGeometry {
        frame: [0.0, 210.0, 72.0, 294.0],
        work_area: [0.0, 0.0, 1000.0, 800.0],
    });
    click_layout(ctx, content, state, label, &layout)
}
fn click_layout(
    ctx: &egui::Context,
    content: &mut Content<'_>,
    state: &MenuState,
    label: &str,
    layout: &Layout,
) -> render::Output {
    click_at(ctx, content, state, label, layout, Duration::from_secs(1))
}
fn click_at(
    ctx: &egui::Context,
    content: &mut Content<'_>,
    state: &MenuState,
    label: &str,
    layout: &Layout,
    now: Duration,
) -> render::Output {
    let paint =
        |content: &mut Content<'_>, events| paint_layout(ctx, content, state, events, layout, now);
    let (mut output, mut regions) = paint(content, vec![]);
    for _ in 0..2 {
        (output, regions) = paint(content, vec![]);
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
            content,
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
fn moving_components_do_not_start_work_or_route_tools_before_settling() {
    let ctx = egui::Context::default();
    crate::ui::common::theme::setup(&ctx);
    let config = Config::default();
    let snapshot = Snapshot::default();
    let mut services = services();
    let mut review = Review::default();
    let mut content = Content {
        config: &config,
        snapshot: &snapshot,
        services: &mut services,
        review: &mut review,
    };
    let layout = Layout::new(DesktopGeometry {
        frame: [0.0, 210.0, 72.0, 294.0],
        work_area: [0.0, 0.0, 1000.0, 800.0],
    });
    let mut state = MenuState::default();
    state.open(Duration::ZERO);
    state.hover(Duration::ZERO, Some(Group::Office), true);
    assert!(
        click_at(
            &ctx,
            &mut content,
            &state,
            "开始计时",
            &layout,
            Duration::from_millis(100)
        )
        .action
        .is_none()
    );
    assert_eq!(content.services.timer.session.state, State::Idle);
    click_layout(&ctx, &mut content, &state, "开始计时", &layout);
    assert_eq!(content.services.timer.session.state, State::Running);
    state.hover(Duration::ZERO, Some(Group::Tools), true);
    assert!(
        click_at(
            &ctx,
            &mut content,
            &state,
            "打开诊断报告",
            &layout,
            Duration::from_millis(100)
        )
        .action
        .is_none()
    );
    assert_eq!(
        click_layout(&ctx, &mut content, &state, "打开诊断报告", &layout).action,
        Some(Action::Report)
    );
}
#[test]
fn mirrored_spiral_actions_reach_the_same_use_cases_at_all_four_corners() {
    let area = [-1920.0, -1032.0, 0.0, 0.0];
    for frame in [
        [-1920.0, -1032.0, -1848.0, -948.0],
        [-72.0, -1032.0, 0.0, -948.0],
        [-1920.0, -84.0, -1848.0, 0.0],
        [-72.0, -84.0, 0.0, 0.0],
    ] {
        let ctx = egui::Context::default();
        crate::ui::common::theme::setup(&ctx);
        let config = Config::default();
        let snapshot = Snapshot::default();
        let mut services = services();
        let mut review = Review::default();
        let mut content = Content {
            config: &config,
            snapshot: &snapshot,
            services: &mut services,
            review: &mut review,
        };
        let layout = Layout::new(DesktopGeometry {
            frame,
            work_area: area,
        });
        let mut state = MenuState::default();
        state.open(Duration::ZERO);
        state.hover(Duration::ZERO, Some(Group::Resources), true);
        assert_eq!(
            click_layout(&ctx, &mut content, &state, "完整", &layout).action,
            Some(Action::Restore)
        );
        assert_eq!(
            click_layout(&ctx, &mut content, &state, "精简", &layout).action,
            Some(Action::Compact)
        );
        click_layout(&ctx, &mut content, &state, "检查清单 · 1 项", &layout);
        assert_eq!(
            content.review.operations(),
            Some(Profile::Light.operations())
        );
        click_layout(&ctx, &mut content, &state, "取消", &layout);
        assert!(content.review.operations().is_none());
        state.hover(Duration::ZERO, Some(Group::Office), true);
        click_layout(&ctx, &mut content, &state, "开始计时", &layout);
        assert_eq!(content.services.timer.session.state, State::Running);
        click_layout(&ctx, &mut content, &state, "暂停计时", &layout);
        assert_eq!(content.services.timer.session.state, State::Paused);
        state.hover(Duration::ZERO, Some(Group::Tools), true);
        assert_eq!(
            click_layout(&ctx, &mut content, &state, "打开诊断报告", &layout).action,
            Some(Action::Report)
        );
        assert_eq!(
            click_layout(&ctx, &mut content, &state, "打开设置", &layout).action,
            Some(Action::Settings)
        );
    }
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
    assert_eq!(
        click(&ctx, &mut content, &state, "便签").action,
        Some(Action::Notes)
    );
    assert_eq!(
        click(&ctx, &mut content, &state, "待办").action,
        Some(Action::Tasks)
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
