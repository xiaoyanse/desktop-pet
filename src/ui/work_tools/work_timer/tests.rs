use crate::{
    application::work_timer::WorkTimer,
    domain::work_timer::{Mode, State, TimerConfig},
    ui::common::theme,
};
use eframe::egui;
use std::time::{Duration, Instant};

fn render(
    ctx: &egui::Context,
    config: &mut TimerConfig,
    timer: &mut WorkTimer,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(840.0, 600.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.allocate_ui(egui::vec2(532.0, 369.0), |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(369.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| super::show(ui, config, timer));
                });
            });
        },
    )
}

fn bounds(shape: &egui::Shape, label: &str) -> Option<egui::Rect> {
    match shape {
        egui::Shape::Text(text) if text.galley.text() == label => {
            Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
        }
        egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| bounds(s, label)),
        _ => None,
    }
}

fn click(ctx: &egui::Context, config: &mut TimerConfig, timer: &mut WorkTimer, label: &str) {
    let mut output = render(ctx, config, timer, vec![]);
    for _ in 0..2 {
        output = render(ctx, config, timer, vec![]);
    }
    let position = output
        .shapes
        .iter()
        .find_map(|shape| {
            bounds(&shape.shape, label).map(|bounds| {
                assert!(
                    shape.clip_rect.contains_rect(bounds),
                    "action must be reachable: {label}, text {bounds:?}, clip {:?}",
                    shape.clip_rect
                );
                bounds.center()
            })
        })
        .unwrap_or_else(|| panic!("missing label: {label}"));
    for pressed in [true, false] {
        let _ = render(
            ctx,
            config,
            timer,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}

#[test]
fn main_actions_work_in_a_narrow_short_viewport_and_freeze_active_scheme() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let mut config = TimerConfig::default();
    let mut timer = WorkTimer::default();
    click(&ctx, &mut config, &mut timer, "开始专注");
    assert_eq!(timer.session.state, State::Running);
    assert_eq!(timer.session.target, Some(Duration::from_secs(1500)));
    click(&ctx, &mut config, &mut timer, "正计时");
    assert!(!config.stopwatch, "running mode selector must be disabled");
    click(&ctx, &mut config, &mut timer, "暂停计时");
    assert_eq!(timer.session.state, State::Paused);
    click(&ctx, &mut config, &mut timer, "继续计时");
    assert_eq!(timer.session.state, State::Running);
    click(&ctx, &mut config, &mut timer, "重置本次");
    assert_eq!(timer.session.state, State::Idle);
    render(
        &ctx,
        &mut config,
        &mut timer,
        vec![
            egui::Event::PointerMoved(egui::pos2(200.0, 160.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, 1000.0),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    click(&ctx, &mut config, &mut timer, "正计时");
    assert!(config.stopwatch);
    click(&ctx, &mut config, &mut timer, "开始正计时");
    assert_eq!(timer.session.mode, Some(Mode::Stopwatch));
    timer.tick(Instant::now() + Duration::from_secs(42));
    let elapsed = timer.session.elapsed;
    click(&ctx, &mut config, &mut timer, "结束计时");
    assert_eq!(timer.session.state, State::Complete);
    assert!(timer.session.elapsed >= elapsed);
    assert!(timer.next_wakeup().is_none());
}

#[test]
fn completion_waits_for_explicit_rest_action_and_dismissal() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let mut config = TimerConfig::default();
    let mut timer = WorkTimer::default();
    let now = Instant::now();
    timer.start(Mode::Focus, &config, now);
    timer.tick(now + Duration::from_secs(1500));
    assert!(timer.notice);
    click(&ctx, &mut config, &mut timer, "知道了");
    assert!(!timer.notice);
    assert_eq!(timer.session.state, State::Complete);
    click(&ctx, &mut config, &mut timer, "开始休息");
    assert_eq!(timer.session.mode, Some(Mode::Break));
    assert_eq!(timer.session.target, Some(Duration::from_secs(300)));
}
