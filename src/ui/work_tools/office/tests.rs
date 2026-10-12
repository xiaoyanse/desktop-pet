use super::*;
use crate::application::office::tests::{fake_service, settle};
use crate::ui::common::theme;
fn paint(
    ctx: &egui::Context,
    page: &mut Page,
    service: &mut OfficeService,
    size: egui::Vec2,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().inner_margin(28))
                .show(ctx, |ui| page.show(ui, service));
        },
    )
}
fn point(out: &egui::FullOutput, label: &str) -> egui::Pos2 {
    out.shapes
        .iter()
        .find_map(|s| {
            if let egui::Shape::Text(t) = &s.shape {
                if t.galley.text() == label {
                    let bounds = egui::Rect::from_min_size(t.pos, t.galley.size());
                    assert!(
                        s.clip_rect.contains_rect(bounds),
                        "{label} clipped: {bounds:?}, {:?}",
                        s.clip_rect
                    );
                    return Some(t.pos + t.galley.size() * 0.5);
                }
                None
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("Missing {label}"))
}
fn click(
    ctx: &egui::Context,
    page: &mut Page,
    service: &mut OfficeService,
    size: egui::Vec2,
    at: egui::Pos2,
) {
    for pressed in [true, false] {
        paint(
            ctx,
            page,
            service,
            size,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        );
    }
}
fn reachable(
    ctx: &egui::Context,
    page: &mut Page,
    service: &mut OfficeService,
    size: egui::Vec2,
    label: &str,
) -> egui::Pos2 {
    for _ in 0..20 {
        let out = paint(ctx, page, service, size, vec![]);
        let visible = out.shapes.iter().find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == label => {
                let r = egui::Rect::from_min_size(t.pos, t.galley.size());
                s.clip_rect.contains_rect(r).then_some(r.center())
            }
            _ => None,
        });
        if let Some(at) = visible {
            return at;
        }
        paint(
            ctx,
            page,
            service,
            size,
            vec![
                egui::Event::PointerMoved(egui::pos2(size.x * 0.5, size.y - 45.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -40.0),
                    modifiers: Default::default(),
                },
            ],
        );
    }
    let out = paint(ctx, page, service, size, vec![]);
    let bounds: Vec<_> = out
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == label => {
                Some((t.pos, t.galley.size(), s.clip_rect))
            }
            _ => None,
        })
        .collect();
    panic!("{label} unreachable after scrolling: {bounds:?}");
}
fn icon_point(ctx: &egui::Context, out: &egui::FullOutput, name: &str) -> egui::Pos2 {
    let id = crate::ui::common::artwork::texture(ctx, name).id();
    out.shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Mesh(mesh) if mesh.texture_id == id => {
                let r = mesh.calc_bounds();
                assert!(s.clip_rect.contains_rect(r), "{name} icon clipped");
                Some(r.center())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing icon {name}"))
}
#[test]
fn actual_page_create_save_failure_preserves_draft_and_retry_persists() {
    for size in [egui::vec2(556.0, 430.0), egui::vec2(756.0, 650.0)] {
        let ctx = egui::Context::default();
        theme::setup(&ctx);
        let (mut service, memory) = fake_service();
        let mut page = Page::default();
        let out = paint(&ctx, &mut page, &mut service, size, vec![]);
        let at = point(&out, "新建便签");
        click(&ctx, &mut page, &mut service, size, at);
        assert!(page.editor.is_some());
        page.editor.as_mut().unwrap().title = "中文便签".into();
        page.editor.as_mut().unwrap().body = "需要保留的工作线索".into();
        memory
            .fail_save
            .store(true, std::sync::atomic::Ordering::SeqCst);
        for _ in 0..2 {
            paint(&ctx, &mut page, &mut service, size, vec![]);
        }
        let at = reachable(&ctx, &mut page, &mut service, size, "保存到本地");
        click(&ctx, &mut page, &mut service, size, at);
        settle(&mut service, 1);
        paint(&ctx, &mut page, &mut service, size, vec![]);
        assert!(service.book.items.is_empty());
        assert_eq!(page.editor.as_ref().unwrap().title, "中文便签");
        memory
            .fail_save
            .store(false, std::sync::atomic::Ordering::SeqCst);
        let at = reachable(&ctx, &mut page, &mut service, size, "保存到本地");
        click(&ctx, &mut page, &mut service, size, at);
        settle(&mut service, 1);
        paint(&ctx, &mut page, &mut service, size, vec![]);
        assert!(page.editor.is_none());
        assert_eq!(service.book.items[0].title, "中文便签");
        assert_eq!(memory.book.lock().unwrap().items, service.book.items);
    }
}

#[test]
fn actual_list_edit_pin_confirmed_delete_and_invalid_task_date() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let size = egui::vec2(556.0, 430.0);
    let (mut service, _) = fake_service();
    service.apply_at(
        crate::domain::office::Command::Save {
            id: None,
            kind: Kind::Note,
            title: "便签标题".into(),
            body: "正文".into(),
            pinned: false,
            due: None,
        },
        1,
    );
    settle(&mut service, 1);
    let mut page = Page::default();
    let mut out = paint(&ctx, &mut page, &mut service, size, vec![]);
    for _ in 0..2 {
        out = paint(&ctx, &mut page, &mut service, size, vec![]);
    }
    let at = icon_point(&ctx, &out, "capture-annotate");
    click(&ctx, &mut page, &mut service, size, at);
    assert_eq!(page.editor.as_ref().unwrap().id, Some(1));
    page.editor.as_mut().unwrap().title = "修改后的标题".into();
    let at = reachable(&ctx, &mut page, &mut service, size, "保存到本地");
    click(&ctx, &mut page, &mut service, size, at);
    settle(&mut service, 1);
    let mut out = paint(&ctx, &mut page, &mut service, size, vec![]);
    for _ in 0..3 {
        out = paint(&ctx, &mut page, &mut service, size, vec![]);
    }
    assert_eq!(service.book.items[0].title, "修改后的标题");
    let at = icon_point(&ctx, &out, "pin");
    click(&ctx, &mut page, &mut service, size, at);
    settle(&mut service, 1);
    assert!(service.book.items[0].pinned);
    let out = paint(&ctx, &mut page, &mut service, size, vec![]);
    let at = icon_point(&ctx, &out, "close");
    click(&ctx, &mut page, &mut service, size, at);
    assert_eq!(service.book.items.len(), 1);
    assert_eq!(page.confirm_delete, Some(1));
    let at = reachable(&ctx, &mut page, &mut service, size, "确认删除");
    click(&ctx, &mut page, &mut service, size, at);
    settle(&mut service, 1);
    assert!(service.book.items.is_empty());
    page.open(Kind::Task);
    let at = reachable(&ctx, &mut page, &mut service, size, "新建待办");
    click(&ctx, &mut page, &mut service, size, at);
    let draft = page.editor.as_mut().unwrap();
    draft.title = "有截止时间的行动".into();
    draft.due = "invalid".into();
    let at = reachable(&ctx, &mut page, &mut service, size, "保存到本地");
    click(&ctx, &mut page, &mut service, size, at);
    assert!(service.book.items.is_empty());
    assert!(page.editor.is_some());
    page.editor.as_mut().unwrap().due = "100".into();
    let at = reachable(&ctx, &mut page, &mut service, size, "保存到本地");
    click(&ctx, &mut page, &mut service, size, at);
    settle(&mut service, 1);
    assert_eq!(service.book.items[0].due, Some(100));
}

#[test]
fn reminder_buttons_are_inside_the_card_and_real_clicks_snooze_and_dismiss() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let (mut service, _) = fake_service();
    service.apply_at(
        crate::domain::office::Command::Save {
            id: None,
            kind: Kind::Task,
            title: "需要提交的工作".into(),
            body: String::new(),
            pinned: false,
            due: Some(1),
        },
        0,
    );
    settle(&mut service, 10);
    let paint = |service: &mut OfficeService, events: Vec<egui::Event>| {
        let item = service.notice().unwrap().clone();
        let mut action = None;
        let out = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(360.0, 296.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .frame(theme::glass_frame().fill(theme::SURFACE))
                    .show(ctx, |ui| action = notice::panel(ui, service, &item).0);
            },
        );
        (out, action)
    };
    let (out, _) = paint(&mut service, vec![]);
    for label in ["标记完成", "10分钟后提醒", "本次忽略", "打开待办列表"] {
        point(&out, label);
    }
    let at = point(&out, "10分钟后提醒");
    let mut action = None;
    for pressed in [true, false] {
        action = paint(
            &mut service,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        )
        .1;
    }
    service.apply_at(action.take().unwrap(), 10);
    settle(&mut service, 10);
    assert_eq!(service.book.items[0].due, Some(610));
    assert!(service.notice().is_none());
    service.tick(610);
    let (out, _) = paint(&mut service, vec![]);
    let at = point(&out, "本次忽略");
    for pressed in [true, false] {
        action = paint(
            &mut service,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        )
        .1;
    }
    service.apply_at(action.take().unwrap(), 610);
    settle(&mut service, 610);
    assert!(service.notice().is_none());
    assert_eq!(service.book.items[0].acknowledged_due, Some(610));
}
