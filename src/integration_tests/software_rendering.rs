use crate::ui::{development::software_preview::Preview, rendering::software::Painter};
use eframe::egui;

#[test]
fn actual_chinese_and_embedded_artwork_generate_nonempty_native_pixels() {
    let ctx = egui::Context::default();
    let mut preview = Preview::new(
        &ctx,
        true,
        crate::infrastructure::windows::fonts::load().unwrap(),
    );
    let raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(240.0, 142.0),
        )),
        max_texture_side: Some(2048),
        ..Default::default()
    };
    let output = ctx.run(raw, |ctx| {
        preview.show(ctx, false);
    });
    let mut painter = Painter::default();
    painter.update(&output.textures_delta).unwrap();
    let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
    let mut pixels = vec![0; 240 * 142 * 4];
    painter
        .paint(
            &mut pixels,
            [240, 142],
            output.pixels_per_point,
            &primitives,
        )
        .unwrap();
    assert!(pixels.chunks_exact(4).any(|p| p[3] > 0));
    assert_eq!(&pixels[..4], &[0; 4]);
    assert!(painter.texture_bytes() < 1024 * 1024);
}

#[test]
fn software_header_darkens_artwork_highlights_before_drawing_real_chinese_controls() {
    let ctx = egui::Context::default();
    let mut preview = Preview::new(
        &ctx,
        false,
        crate::infrastructure::windows::fonts::load().unwrap(),
    );
    let raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1040.0, 820.0),
        )),
        max_texture_side: Some(2048),
        ..Default::default()
    };
    let output = ctx.run(raw, |ctx| {
        preview.show(ctx, false);
    });
    let mut painter = Painter::default();
    painter.update(&output.textures_delta).unwrap();
    let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
    let mut pixels = vec![0; 1040 * 820 * 4];
    painter
        .paint(
            &mut pixels,
            [1040, 820],
            output.pixels_per_point,
            &primitives,
        )
        .unwrap();
    let sample = &pixels[(10 * 1040 + 500) * 4..][..4];
    assert!(sample[0] <= 63 && sample[1] <= 52 && sample[2] <= 44);
    assert_eq!(sample[3], 255);
    assert!(pixels.chunks_exact(4).filter(|p| p[3] == 255).count() > 800_000);
}
