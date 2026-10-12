use crate::interfaces::FontSource;
use eframe::egui;
use std::{borrow::Cow, sync::Arc};

pub fn install(ctx: &egui::Context, source: FontSource) {
    configure_atlas(ctx);
    ctx.set_fonts(definitions(source));
}

pub const MAX_ATLAS_SIDE: usize = 2048;

pub fn configure_atlas(ctx: &egui::Context) {
    ctx.on_begin_pass(
        "bounded-font-atlas",
        Arc::new(|ctx| {
            let maximum = ctx.input(|input| input.max_texture_side);
            if maximum > MAX_ATLAS_SIDE {
                ctx.input_mut(|input| input.max_texture_side = MAX_ATLAS_SIDE);
                // Immediate child viewports bypass App::raw_input_hook and arrive with the GPU limit.
                let scale = ctx.pixels_per_point();
                let style = ctx.style();
                let preload = ctx.options(|options| options.preload_font_glyphs);
                ctx.fonts(|fonts| {
                    fonts.begin_pass(scale, MAX_ATLAS_SIDE);
                    if preload {
                        // Preserve framework font-cache insertion order without rasterizing unused ASCII.
                        for font in style.text_styles.values() {
                            fonts.row_height(font);
                        }
                    }
                });
            }
        }),
    );
}

pub fn definitions(source: FontSource) -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    let data = match source.bytes {
        Cow::Borrowed(bytes) => egui::FontData::from_static(bytes),
        Cow::Owned(bytes) => egui::FontData::from_owned(bytes),
    };
    fonts
        .font_data
        .insert(source.name.to_string(), Arc::new(data));
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, source.name.to_string());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push(source.name.to_string());
    fonts
}
