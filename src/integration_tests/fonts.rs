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
