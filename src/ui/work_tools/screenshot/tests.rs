use super::*;
use crate::{
    application::screenshot::tests::fake_service,
    domain::screenshot::{Document, Image},
    ui::common::theme,
};
use std::sync::atomic::Ordering;

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
