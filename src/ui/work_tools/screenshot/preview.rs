use crate::domain::screenshot::Image;
use eframe::egui;

pub fn texture(ctx: &egui::Context, image: &Image, name: &str) -> egui::TextureHandle {
    let [w, h] = image.size();
    let limit = ctx.input(|i| i.max_texture_side).clamp(1, 1536) as f64;
    let scale = (limit / w.max(h) as f64).min(1.0);
    let size = [
        ((w as f64 * scale).round() as usize).max(1),
        ((h as f64 * scale).round() as usize).max(1),
    ];
    let mut pixels = Vec::with_capacity(size[0] * size[1]);
    for y in 0..size[1] {
        let source_y = (y as u64 * h as u64 / size[1] as u64) as usize;
        for x in 0..size[0] {
            let source_x = (x as u64 * w as u64 / size[0] as u64) as usize;
            let index = (source_y * w as usize + source_x) * 4;
            let p = &image.pixels()[index..index + 4];
            pixels.push(egui::Color32::from_rgba_unmultiplied(
                p[0], p[1], p[2], p[3],
            ));
        }
    }
    ctx.load_texture(
        name,
        egui::ColorImage { size, pixels },
        egui::TextureOptions::LINEAR,
    )
}
