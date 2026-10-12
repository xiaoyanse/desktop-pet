use super::*;
use crate::application::meeting::tests::fake_service;
use std::sync::atomic::Ordering;
fn frame(
    ctx: &egui::Context,
    config: &mut MeetingConfig,
    service: &mut MeetingService,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(556.0, 650.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| show(ui, config, service));
            });
        },
    )
}
fn click(
    ctx: &egui::Context,
    config: &mut MeetingConfig,
    service: &mut MeetingService,
    label: &str,
) {
    let out = frame(ctx, config, service, vec![]);
    let at = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == label => {
                let r = egui::Rect::from_min_size(t.pos, t.galley.size());
                assert!(s.clip_rect.contains_rect(r), "{label} clipped");
                Some(r.center())
            }
            _ => None,
        })
        .expect("action must be visible");
    for pressed in [true, false] {
        frame(
            ctx,
            config,
            service,
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
fn production_page_start_pause_resume_finish_has_real_actions_and_no_audio_query() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let (mut service, source) = fake_service();
    let mut config = MeetingConfig::default();
    click(&ctx, &mut config, &mut service, "开始会议计时");
    assert_eq!(service.session.state, State::Running);
    click(&ctx, &mut config, &mut service, "暂停会议计时");
    assert_eq!(service.session.state, State::Paused);
    click(&ctx, &mut config, &mut service, "继续会议计时");
    assert_eq!(service.session.state, State::Running);
    click(&ctx, &mut config, &mut service, "结束会议");
    assert_eq!(service.session.state, State::Complete);
    click(&ctx, &mut config, &mut service, "清除本次计时");
    assert_eq!(service.session.state, State::Idle);
    assert_eq!(
        source.calls.load(Ordering::SeqCst),
        0,
        "page render must not read hardware itself"
    );
}
#[test]
fn unknown_and_unavailable_microphone_are_not_shown_as_muted_or_zero() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let (mut service, _) = fake_service();
    let mut config = MeetingConfig::default();
    for result in [None, Some(Err("没有通信输入设备".into()))] {
        service.microphone = result;
        let out = frame(&ctx, &mut config, &mut service, vec![]);
        assert!(!out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="系统已静音" || t.galley.text()=="输入音量 0%")));
    }
}

#[test]
fn marker_reminders_can_be_acknowledged_in_production_page() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let (mut service, _) = fake_service();
    let mut config = MeetingConfig::default();
    service
        .notices
        .push_back(crate::domain::meeting::Notice::Time);
    service
        .notices
        .push_back(crate::domain::meeting::Notice::Microphone);
    click(&ctx, &mut config, &mut service, "已查看提醒");
    assert_eq!(service.notices.len(), 1);
    click(&ctx, &mut config, &mut service, "已查看提醒");
    assert!(service.notices.is_empty());
}

#[test]
fn native_sized_reminder_panel_actions_fit_and_dismiss_once() {
    for label in ["查看会议", "知道了"] {
        let ctx = egui::Context::default();
        theme::setup(&ctx);
        let (mut service, _) = fake_service();
        service
            .notices
            .push_back(crate::domain::meeting::Notice::Time);
        service
            .notices
            .push_back(crate::domain::meeting::Notice::Microphone);
        let mut render = |events| {
            let mut open = false;
            let out = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(360.0, 250.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .frame(crate::ui::common::components::layout::dialog_frame())
                        .show(ctx, |ui| {
                            open = notice::panel(
                                ui,
                                &mut service,
                                crate::domain::meeting::Notice::Time,
                            );
                        });
                },
            );
            (out, open)
        };
        let (out, _) = render(vec![]);
        let at = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == label => {
                    let rect = egui::Rect::from_min_size(t.pos, t.galley.size());
                    assert!(s.clip_rect.contains_rect(rect) && rect.bottom() < 250.0);
                    Some(rect.center())
                }
                _ => None,
            })
            .expect("reminder action visible");
        let mut opened = false;
        for pressed in [true, false] {
            opened |= render(vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ])
            .1;
        }
        assert_eq!(opened, label == "查看会议");
        assert_eq!(
            service.notices.len(),
            1,
            "one click dismisses exactly one notice"
        );
    }
}
