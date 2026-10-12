use eframe::egui;
use std::borrow::Cow;

#[test]
fn borrowed_chinese_font_keeps_glyphs_and_layout_at_multiple_scales() {
    let Some(source) = crate::infrastructure::windows::fonts::load() else {
        return;
    };
    assert!(matches!(source.bytes, Cow::Borrowed(_)));
    let original_pointer = source.bytes.as_ptr();
    let name = source.name.clone();
    let definitions = crate::ui::test_font_definitions(source);
    assert!(matches!(
        definitions.font_data[name.as_ref()].font,
        Cow::Borrowed(_)
    ));
    assert_eq!(
        definitions.font_data[name.as_ref()].font.as_ptr(),
        original_pointer
    );
    let mut previous_width: Option<f32> = None;
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let ctx = egui::Context::default();
        crate::ui::install_test_font(&ctx, crate::infrastructure::windows::fonts::load().unwrap());
        ctx.set_pixels_per_point(scale);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            ctx.fonts(|fonts| {
                assert_eq!(fonts.pixels_per_point(), scale);
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                    let font = egui::FontId::new(14.0, family.clone());
                    assert!(fonts.has_glyphs(&font, "桌面仪表资源监测分页池诊断报告工作计时"));
                    let galley = fonts.layout_no_wrap(
                        "资源监测 CPU 81.5%".into(),
                        font,
                        egui::Color32::WHITE,
                    );
                    assert!(galley.size().x > 100.0);
                    if family == egui::FontFamily::Proportional {
                        if let Some(width) = previous_width {
                            assert!((galley.size().x - width).abs() < 3.0);
                        }
                        previous_width = Some(galley.size().x);
                    }
                }
            });
        });
    }
}

#[test]
fn gpu_sized_font_atlas_is_bounded_for_root_and_immediate_children_without_layout_loss() {
    let Some(source) = crate::infrastructure::windows::fonts::load() else {
        return;
    };
    let unbounded = egui::Context::default();
    unbounded.set_fonts(crate::ui::test_font_definitions(source));
    let bounded = egui::Context::default();
    crate::ui::install_test_font(
        &bounded,
        crate::infrastructure::windows::fonts::load().unwrap(),
    );
    let mut baseline_sizes = Vec::new();
    let mut bounded_sizes = Vec::new();
    let text = "桌面仪表监测项目外观宠物设备设置内存清理工作计时诊断报告分页池非分页池CPU网络磁盘处理器频率可用容量上传下载";
    let mut atlas_bytes = [0; 2];
    for scale in [1.0, 1.25, 1.5, 2.0] {
        for viewport in [
            egui::ViewportId::ROOT,
            egui::ViewportId::from_hash_of("settings"),
        ] {
            for (index, ctx) in [&unbounded, &bounded].into_iter().enumerate() {
                let mut raw = egui::RawInput {
                    viewport_id: viewport,
                    max_texture_side: Some(16384),
                    ..Default::default()
                };
                raw.viewports.insert(
                    viewport,
                    egui::ViewportInfo {
                        native_pixels_per_point: Some(scale),
                        ..Default::default()
                    },
                );
                let out = ctx.run(raw, |ctx| {
                    ctx.fonts(|fonts| {
                        if index == 1 {
                            assert_eq!(fonts.max_texture_side(), 2048);
                            assert_eq!(fonts.font_image_size()[0], 2048);
                        }
                        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace]
                        {
                            for size in [12.0, 14.0, 28.0] {
                                let font = egui::FontId::new(size, family.clone());
                                assert!(fonts.has_glyphs(&font, text));
                                let layout =
                                    fonts.layout_no_wrap(text.into(), font, egui::Color32::WHITE);
                                if index == 0 {
                                    baseline_sizes.push(layout.size());
                                } else {
                                    bounded_sizes.push(layout.size());
                                }
                            }
                        }
                    });
                });
                let bytes = out
                    .textures_delta
                    .set
                    .iter()
                    .filter(|(id, _)| *id == egui::TextureId::Managed(0))
                    .map(|(_, delta)| delta.image.width() * delta.image.height() * 4)
                    .max()
                    .unwrap_or(0);
                atlas_bytes[index] = atlas_bytes[index].max(bytes);
            }
        }
    }
    assert_eq!(
        baseline_sizes, bounded_sizes,
        "atlas packing must not change logical text layout"
    );
    assert!(
        atlas_bytes[1] < atlas_bytes[0],
        "narrow atlas must reduce the largest upload: {atlas_bytes:?}"
    );
}
