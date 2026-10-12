use super::{Page, preview};
use crate::{
    application::screenshot::ScreenshotService,
    domain::screenshot::{Mark, Tool},
};
use eframe::egui::{self, Color32};

pub fn show(ui: &mut egui::Ui, page: &mut Page, service: &mut ScreenshotService) {
    let busy = service.busy();
    let Some(document) = service.document.as_mut() else {
        return;
    };
    let image_size = document.image.size();
    let texture = page
        .texture
        .get_or_insert_with(|| preview::texture(ui.ctx(), &document.image, "screenshot-preview"));
    let available = egui::vec2(
        ui.available_width().max(1.0),
        ui.available_height().max(1.0),
    );
    let scale = (available.x / image_size[0] as f32).min(available.y / image_size[1] as f32);
    let size = egui::vec2(image_size[0] as f32, image_size[1] as f32) * scale;
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    ui.painter()
        .rect_filled(rect, 4, Color32::from_rgb(37, 44, 54));
    ui.painter().image(
        texture.id(),
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
    let painter = ui.painter().with_clip_rect(rect);
    for mark in document.marks() {
        draw(&painter, rect, scale, mark);
    }
    if !page.stroke.is_empty() {
        draw(
            &painter,
            rect,
            scale,
            &Mark {
                tool: page.tool,
                points: page.stroke.clone(),
                color: page.color,
                width: page.width,
            },
        );
    }
    if busy {
        page.stroke.clear();
        return;
    }
    if response.drag_started() {
        page.stroke.clear();
        if let Some(position) = ui.input(|i| i.pointer.press_origin()) {
            page.stroke.push(point(rect, image_size, position));
        }
    }
    if response.dragged() || response.drag_stopped() {
        if let Some(position) = response.interact_pointer_pos() {
            let next = point(rect, image_size, position);
            if page.tool == Tool::Pen {
                if page.stroke.len() < 1024 && page.stroke.last() != Some(&next) {
                    page.stroke.push(next);
                }
            } else if page.stroke.len() == 1 {
                page.stroke.push(next);
            } else if let Some(last) = page.stroke.last_mut() {
                *last = next;
            }
        }
        if response.drag_stopped() && page.stroke.len() >= 2 {
            let mark = Mark {
                tool: page.tool,
                points: std::mem::take(&mut page.stroke),
                color: page.color,
                width: page.width,
            };
            if let Err(error) = service.document.as_mut().unwrap().add(mark) {
                service.message = error;
            }
        }
    }
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        page.stroke.clear();
    }
}
fn point(rect: egui::Rect, size: [u32; 2], position: egui::Pos2) -> [u32; 2] {
    [
        (((position.x - rect.left()) / rect.width() * size[0] as f32).floor() as i64)
            .clamp(0, size[0] as i64 - 1) as u32,
        (((position.y - rect.top()) / rect.height() * size[1] as f32).floor() as i64)
            .clamp(0, size[1] as i64 - 1) as u32,
    ]
}
fn draw(painter: &egui::Painter, rect: egui::Rect, scale: f32, mark: &Mark) {
    if mark.points.len() < 2 {
        return;
    }
    let point = |p: [u32; 2]| rect.min + egui::vec2(p[0] as f32 + 0.5, p[1] as f32 + 0.5) * scale;
    let a = point(mark.points[0]);
    let b = point(*mark.points.last().unwrap());
    let color = Color32::from_rgb(mark.color[0], mark.color[1], mark.color[2]);
    let stroke = egui::Stroke::new((mark.width as f32 * scale).max(0.5), color);
    match mark.tool {
        Tool::Rectangle => {
            painter.rect_stroke(
                egui::Rect::from_two_pos(a, b),
                0,
                stroke,
                egui::StrokeKind::Middle,
            );
        }
        Tool::Cover => {
            painter.rect_filled(egui::Rect::from_two_pos(a, b).expand(scale * 0.5), 0, color);
        }
        Tool::Pen => {
            painter.add(egui::Shape::line(
                mark.points.iter().map(|p| point(*p)).collect(),
                stroke,
            ));
        }
        Tool::Arrow => {
            painter.line_segment([a, b], stroke);
            let delta = b - a;
            let length = delta.length();
            if length > 0.0 {
                let direction = delta / length;
                let wing = (mark.width as f32 * 4.0)
                    .max(10.0)
                    .min(length / scale * 0.45)
                    * scale;
                for sign in [-1.0, 1.0] {
                    let rotated = egui::vec2(
                        direction.x * 0.8660254 - direction.y * 0.5 * sign,
                        direction.x * 0.5 * sign + direction.y * 0.8660254,
                    );
                    painter.line_segment([b, b - rotated * wing], stroke);
                }
            }
        }
    }
}
