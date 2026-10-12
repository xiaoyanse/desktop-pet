use super::preview;
use crate::{
    application::screenshot::ScreenshotService,
    ui::common::icons::{self, Icon},
};
use eframe::egui::{self, Color32};

#[derive(Default)]
pub struct Pinned {
    generation: u64,
    texture: Option<egui::TextureHandle>,
    zoom: f32,
}
impl Pinned {
    pub fn hide(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd_to(
            egui::ViewportId::from_hash_of("pinned-screenshot"),
            egui::ViewportCommand::Visible(false),
        );
    }
    pub fn show(&mut self, ctx: &egui::Context, service: &mut ScreenshotService, hidden: bool) {
        let Some(image) = service.pinned.as_ref() else {
            self.texture = None;
            return;
        };
        if hidden {
            return;
        }
        if self.generation != service.pin_generation {
            self.texture = None;
            self.generation = service.pin_generation;
            self.zoom = 1.0;
        }
        let texture = self
            .texture
            .get_or_insert_with(|| preview::texture(ctx, image, "pinned-screenshot-preview"));
        let [w, h] = image.size();
        let scale = (640.0 / w as f32).min(480.0 / h as f32).min(1.0);
        let mut closed = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("pinned-screenshot"),
            egui::ViewportBuilder::default()
                .with_title("桌面仪表 · 置顶贴图")
                .with_decorations(false)
                .with_taskbar(false)
                .with_always_on_top()
                .with_resizable(true)
                .with_inner_size([w as f32 * scale + 16.0, h as f32 * scale + 56.0])
                .with_min_inner_size([180.0, 120.0]),
            |ctx, _| {
                if ctx.input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape))
                {
                    closed = true;
                }
                egui::TopBottomPanel::top("pinned-toolbar").show(ctx, |ui| {
                    closed |= toolbar(ui, &mut self.zoom);
                });
                egui::CentralPanel::default().show(ctx, |ui| {
                    egui::ScrollArea::both().show(ui, |ui| {
                        let size = egui::vec2(w as f32, h as f32) * scale * self.zoom;
                        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::drag());
                        ui.painter()
                            .rect_filled(rect, 0, Color32::from_rgb(37, 44, 54));
                        ui.painter().image(
                            texture.id(),
                            rect,
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                        if response.drag_started() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                    });
                });
            },
        );
        if closed {
            service.pinned = None;
            self.texture = None;
        }
    }
}

fn toolbar(ui: &mut egui::Ui, zoom: &mut f32) -> bool {
    let mut closed = false;
    ui.horizontal(|ui| {
        if ui.available_width() >= 280.0 {
            let drag = ui.add(egui::Label::new("置顶参考").sense(egui::Sense::drag()));
            if drag.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
        }
        if icons::button(ui, Icon::Minus, "缩小贴图").clicked() {
            *zoom = (*zoom / 1.25).max(0.25);
        }
        ui.label(format!("{:.0}%", *zoom * 100.0));
        if icons::button(ui, Icon::Plus, "放大贴图").clicked() {
            *zoom = (*zoom * 1.25).min(4.0);
        }
        if icons::button(ui, Icon::Cancel, "关闭贴图 · Esc").clicked() {
            closed = true;
        }
    });
    closed
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tiny_pinned_window_keeps_zoom_and_close_inside_client_area() {
        let ctx = egui::Context::default();
        crate::ui::common::theme::setup(&ctx);
        let mut zoom = 1.0;
        let out = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(180.0, 120.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::TopBottomPanel::top("pin-toolbar-test").show(ctx, |ui| {
                    assert!(!toolbar(ui, &mut zoom));
                });
            },
        );
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(180.0, 120.0));
        let mut buttons = 0;
        for shape in out.shapes {
            if let egui::Shape::Mesh(mesh) = &shape.shape {
                buttons += 1;
                assert!(
                    screen.contains_rect(mesh.calc_bounds()),
                    "toolbar icon outside tiny viewport"
                );
            }
            if let egui::Shape::Text(text) = &shape.shape {
                assert!(
                    screen.contains_rect(egui::Rect::from_min_size(text.pos, text.galley.size())),
                    "toolbar label outside tiny viewport"
                );
            }
        }
        assert!(buttons >= 3, "missing zoom/close icons");
    }
}
