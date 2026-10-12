use super::*;
use crate::{
    application::screenshot::tests::fake_service,
    domain::screenshot::{Document, Image},
    ui::common::theme,
};
use std::sync::atomic::Ordering;

fn canvas_frame(
    ctx: &egui::Context,
    page: &mut Page,
    service: &mut ScreenshotService,
    events: Vec<egui::Event>,
) {
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(160.0, 160.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| canvas::show(ui, page, service));
        },
    );
}
fn move_pointer(x: f32, y: f32) -> egui::Event {
    egui::Event::PointerMoved(egui::pos2(x, y))
}
fn pointer_button(x: f32, y: f32, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: egui::pos2(x, y),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}
fn escape() -> egui::Event {
    egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn all_annotation_tools_commit_real_pointer_drag_and_undo_without_changing_original() {
    for tool in [Tool::Rectangle, Tool::Arrow, Tool::Pen, Tool::Cover] {
        let ctx = egui::Context::default();
        let (mut service, backend) = fake_service();
        let original = vec![255; 64 * 64 * 4];
        service.document = Some(Document::new(
            Image::new([64, 64], original.clone()).unwrap(),
        ));
        let mut page = Page {
            tool,
            ..Default::default()
        };
        canvas_frame(&ctx, &mut page, &mut service, vec![]);
        canvas_frame(
            &ctx,
            &mut page,
            &mut service,
            vec![move_pointer(20.0, 20.0), pointer_button(20.0, 20.0, true)],
        );
        canvas_frame(
            &ctx,
            &mut page,
            &mut service,
            vec![move_pointer(110.0, 100.0)],
        );
        canvas_frame(
            &ctx,
            &mut page,
            &mut service,
            vec![pointer_button(110.0, 100.0, false)],
        );
        let doc = service.document.as_mut().unwrap();
        assert_eq!(doc.marks().len(), 1, "{}", tool.label());
        assert_eq!(doc.marks()[0].tool, tool);
        assert_ne!(doc.render().unwrap().pixels(), original);
        assert_eq!(doc.image.pixels(), original);
        doc.undo();
        assert_eq!(doc.render().unwrap().pixels(), original);
        assert!(page.stroke.is_empty());
        assert_eq!(backend.calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn escape_cancels_drag_through_release_and_a_new_drag_works() {
    for release_in_escape_frame in [false, true] {
        let ctx = egui::Context::default();
        let (mut service, _) = fake_service();
        service.document = Some(Document::new(
            Image::new([64, 64], vec![255; 64 * 64 * 4]).unwrap(),
        ));
        let mut page = Page {
            tool: Tool::Pen,
            ..Default::default()
        };
        canvas_frame(&ctx, &mut page, &mut service, vec![]);
        canvas_frame(
            &ctx,
            &mut page,
            &mut service,
            vec![move_pointer(20.0, 20.0), pointer_button(20.0, 20.0, true)],
        );
        canvas_frame(
            &ctx,
            &mut page,
            &mut service,
            vec![move_pointer(90.0, 80.0)],
        );
        assert!(!page.stroke.is_empty());
        let mut events = vec![escape()];
        if release_in_escape_frame {
            events.push(pointer_button(90.0, 80.0, false));
        }
        canvas_frame(&ctx, &mut page, &mut service, events);
        if !release_in_escape_frame {
            canvas_frame(
                &ctx,
                &mut page,
                &mut service,
                vec![move_pointer(120.0, 100.0)],
            );
            canvas_frame(
                &ctx,
                &mut page,
                &mut service,
                vec![pointer_button(120.0, 100.0, false)],
            );
        }
        assert!(service.document.as_ref().unwrap().marks().is_empty());
        assert!(page.stroke.is_empty());
        canvas_frame(
            &ctx,
            &mut page,
            &mut service,
            vec![move_pointer(30.0, 30.0), pointer_button(30.0, 30.0, true)],
        );
        canvas_frame(
            &ctx,
            &mut page,
            &mut service,
            vec![move_pointer(110.0, 110.0)],
        );
        canvas_frame(
            &ctx,
            &mut page,
            &mut service,
            vec![pointer_button(110.0, 110.0, false)],
        );
        assert_eq!(service.document.as_ref().unwrap().marks().len(), 1);
    }
}

#[test]
fn opening_screenshot_page_never_captures_and_four_modes_and_outputs_stay_visible() {
    for size in [egui::vec2(1040.0, 820.0), egui::vec2(840.0, 600.0)] {
        let ctx = egui::Context::default();
        theme::setup(&ctx);
        let (mut service, backend) = fake_service();
        let mut page = Page::default();
        service.document = Some(Document::new(
            Image::new([16, 9], vec![255; 16 * 9 * 4]).unwrap(),
        ));
        let mut result = None;
        for _ in 0..3 {
            result = Some(ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::new().inner_margin(28))
                        .show(ctx, |ui| {
                            ui.set_width(size.x - 228.0 - 56.0);
                            ui.add_space(150.0);
                            page.show(ui, &mut service);
                        });
                },
            ));
        }
        let result = result.unwrap();
        for label in [
            "全屏截图",
            "自由形状",
            "矩形区域",
            "识别窗口",
            "复制图片",
            "保存 PNG",
            "置顶贴图",
        ] {
            assert!(result.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text()==label && shape.clip_rect.contains_rect(egui::Rect::from_min_size(text.pos,text.galley.size())))),"{label} missing/clipped at {size:?}: {:?}",result.shapes.iter().filter_map(|shape|if let egui::Shape::Text(text)=&shape.shape{Some((text.galley.text(),text.pos,text.galley.size(),shape.clip_rect))}else{None}).collect::<Vec<_>>());
        }
        assert_eq!(backend.calls.load(Ordering::SeqCst), 0);
        assert!(page.request.is_none());
        page.release();
        assert!(page.texture.is_none());
    }
}
