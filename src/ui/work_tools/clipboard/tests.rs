use super::*;
use crate::{
    application::clipboard::tests::fake_service,
    domain::{clipboard::Packet, screenshot::Image},
    ui::common::artwork,
};
fn paint(
    ctx: &egui::Context,
    page: &mut Page,
    service: &mut ClipboardService,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut config = service.config.clone();
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(556.0, 600.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().inner_margin(18))
                .show(ctx, |ui| page.show(ui, &mut config, service));
        },
    )
}
fn label(out: &egui::FullOutput, text: &str) -> egui::Pos2 {
    out.shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == text => Some(t.pos + t.galley.size() * 0.5),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing {text}"))
}
fn click(ctx: &egui::Context, page: &mut Page, service: &mut ClipboardService, at: egui::Pos2) {
    for pressed in [true, false] {
        paint(
            ctx,
            page,
            service,
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
fn icon(ctx: &egui::Context, out: &egui::FullOutput, name: &str) -> egui::Pos2 {
    let texture = artwork::texture(ctx, name).id();
    out.shapes
        .iter()
        .rev()
        .find_map(|s| match &s.shape {
            egui::Shape::Mesh(m) if m.texture_id == texture => {
                let bounds = m.calc_bounds();
                s.clip_rect.contains_rect(bounds).then_some(bounds.center())
            }
            _ => None,
        })
        .unwrap()
}
#[test]
fn actual_start_pause_and_confirmed_clear_do_not_copy_content() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let (mut service, source) = fake_service();
    let mut page = Page::default();
    let out = paint(&ctx, &mut page, &mut service, vec![]);
    assert_eq!(source.starts.load(std::sync::atomic::Ordering::Relaxed), 0);
    click(
        &ctx,
        &mut page,
        &mut service,
        label(&out, "开始记录新复制内容"),
    );
    service.tick();
    assert!(service.state == State::Listening);
    service
        .history
        .insert(Packet::text("中文测试".into()).unwrap(), 1, &service.config)
        .unwrap();
    let out = paint(&ctx, &mut page, &mut service, vec![]);
    click(
        &ctx,
        &mut page,
        &mut service,
        label(&out, "暂停记录 · 保留历史"),
    );
    service.tick();
    assert!(service.state == State::Off);
    assert_eq!(service.history.items.len(), 1);
    let out = paint(&ctx, &mut page, &mut service, vec![]);
    click(&ctx, &mut page, &mut service, label(&out, "清空历史"));
    let out = paint(&ctx, &mut page, &mut service, vec![]);
    click(&ctx, &mut page, &mut service, label(&out, "保留内容"));
    assert_eq!(service.history.items.len(), 1);
    let out = paint(&ctx, &mut page, &mut service, vec![]);
    click(&ctx, &mut page, &mut service, label(&out, "清空历史"));
    let out = paint(&ctx, &mut page, &mut service, vec![]);
    click(&ctx, &mut page, &mut service, label(&out, "确认清空历史"));
    assert!(service.history.items.is_empty());
    assert!(page.filtered.is_empty());
    assert!(source.copies.lock().unwrap().is_empty());
}
#[test]
fn actual_icon_pin_delete_and_original_copy_work() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let (mut service, source) = fake_service();
    let mut page = Page::default();
    service
        .history
        .insert(
            Packet::picture(Image::new([400, 200], vec![120; 400 * 200 * 4]).unwrap()).unwrap(),
            1,
            &service.config,
        )
        .unwrap();
    let out = paint(&ctx, &mut page, &mut service, vec![]);
    assert_eq!(page.textures.len(), 1);
    click(&ctx, &mut page, &mut service, icon(&ctx, &out, "pin"));
    assert!(service.history.items[0].pinned);
    let out = paint(&ctx, &mut page, &mut service, vec![]);
    click(&ctx, &mut page, &mut service, icon(&ctx, &out, "clipboard"));
    for _ in 0..100 {
        service.tick();
        if !service.copying() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(matches!(&source.copies.lock().unwrap()[0],Content::Picture(i) if i.size()==[400,200]));
    let out = paint(&ctx, &mut page, &mut service, vec![]);
    click(&ctx, &mut page, &mut service, icon(&ctx, &out, "close"));
    let out = paint(&ctx, &mut page, &mut service, vec![]);
    click(&ctx, &mut page, &mut service, label(&out, "确认删除这一条"));
    assert!(service.history.items.is_empty());
    page.release();
    assert!(page.textures.is_empty());
}
#[test]
fn search_filter_pin_order_and_capacity_apply_use_real_model() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let (mut service, _) = fake_service();
    let mut page = Page::default();
    for text in ["中文 Alpha", "第二条 Beta"] {
        service
            .history
            .insert(Packet::text(text.into()).unwrap(), 1, &service.config)
            .unwrap();
    }
    service.history.pin(1);
    page.refresh(&service);
    assert_eq!(page.filtered, vec![1, 2]);
    page.query = "alpha".into();
    page.refresh(&service);
    assert_eq!(page.filtered, vec![1]);
    page.filter = Filter::Picture;
    page.refresh(&service);
    assert!(page.filtered.is_empty());
    page.query.clear();
    page.filter = Filter::All;
    page.policy = Some(ClipboardConfig { count: 1, mib: 4 });
    paint(&ctx, &mut page, &mut service, vec![]);
    let out = paint(&ctx, &mut page, &mut service, vec![]);
    click(&ctx, &mut page, &mut service, label(&out, "应用容量"));
    assert_eq!(service.config.count, 1);
    assert_eq!(service.history.items.len(), 1);
    assert_eq!(service.history.items[0].id, 1);
}
#[test]
fn thumbnail_cache_is_viewport_bounded_and_released_when_filtered_or_closed() {
    let ctx = egui::Context::default();
    theme::setup(&ctx);
    let (mut service, _) = fake_service();
    let mut page = Page::default();
    service.config.count = 200;
    for i in 0..80 {
        service
            .history
            .insert(
                Packet::picture(Image::new([1, 1], vec![i, 120, 140, 255]).unwrap()).unwrap(),
                1,
                &service.config,
            )
            .unwrap();
    }
    for _ in 0..2 {
        paint(&ctx, &mut page, &mut service, vec![]);
    }
    assert_eq!(page.filtered.len(), 80);
    assert!(!page.textures.is_empty());
    assert!(page.textures.len() < 10);
    page.query = "没有匹配的尺寸".into();
    paint(&ctx, &mut page, &mut service, vec![]);
    assert!(page.textures.is_empty());
    page.query.clear();
    paint(&ctx, &mut page, &mut service, vec![]);
    page.release();
    assert!(page.textures.is_empty());
}
