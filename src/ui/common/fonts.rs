use crate::interfaces::FontSource;
use eframe::egui;
use std::{borrow::Cow, sync::Arc};

pub fn install(ctx: &egui::Context, source: FontSource) {
    ctx.set_fonts(definitions(source));
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
