use eframe::egui::{self, Color32, Rect};

pub fn show<R>(
    ui: &mut egui::Ui,
    frame: egui::Frame,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let mut prepared = frame.begin(ui);
    let decoration = prepared.content_ui.painter().add(egui::Shape::Noop);
    let inner = contents(&mut prepared.content_ui);
    let rect = frame.widget_rect(prepared.content_ui.min_rect());
    let response = prepared.end(ui);
    // Reserve before the contents so translucency never washes out text or controls.
    ui.painter()
        .set(decoration, finish(rect.shrink(1.5), 14.0, false));
    egui::InnerResponse::new(inner, response)
}

pub fn finish(rect: Rect, radius: f32, emphasized: bool) -> egui::Shape {
    let top = if emphasized {
        Color32::from_rgba_premultiplied(17, 21, 26, 38)
    } else {
        Color32::from_rgba_premultiplied(5, 8, 11, 20)
    };
    let bottom = Color32::from_black_alpha(10);
    let mesh = gradient(rect, radius, top, bottom);
    let outline = egui::epaint::RectShape::stroke(
        rect,
        radius as u8,
        egui::Stroke::new(
            1.0_f32,
            if emphasized {
                Color32::from_rgba_premultiplied(43, 46, 49, 65)
            } else {
                Color32::from_rgba_premultiplied(16, 19, 22, 35)
            },
        ),
        egui::StrokeKind::Inside,
    );
    egui::Shape::Vec(vec![egui::Shape::mesh(mesh), egui::Shape::Rect(outline)])
}

fn gradient(rect: Rect, radius: f32, top: Color32, bottom: Color32) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    if !rect.is_positive() || !rect.is_finite() {
        return mesh;
    }
    let radius = radius
        .min(rect.width() * 0.5)
        .min(rect.height() * 0.5)
        .max(0.0);
    let color = |pos: egui::Pos2| {
        let t = ((pos.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
        Color32::from(egui::lerp(
            egui::Rgba::from(top)..=egui::Rgba::from(bottom),
            t,
        ))
    };
    mesh.colored_vertex(rect.center(), color(rect.center()));
    for (center, start) in [
        (
            egui::pos2(rect.right() - radius, rect.top() + radius),
            -90.0f32,
        ),
        (
            egui::pos2(rect.right() - radius, rect.bottom() - radius),
            0.0,
        ),
        (
            egui::pos2(rect.left() + radius, rect.bottom() - radius),
            90.0,
        ),
        (egui::pos2(rect.left() + radius, rect.top() + radius), 180.0),
    ] {
        for step in 0..=6 {
            let angle = (start + step as f32 * 15.0).to_radians();
            let pos = center + egui::vec2(angle.cos(), angle.sin()) * radius;
            mesh.colored_vertex(pos, color(pos));
        }
    }
    let count = mesh.vertices.len() as u32 - 1;
    for index in 1..=count {
        mesh.add_triangle(0, index, index % count + 1);
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glass_decoration_stays_behind_content_and_inside_the_frame() {
        let ctx = egui::Context::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let panel = show(ui, super::super::theme::glass_frame(), |ui| {
                    ui.label("Glass content");
                    7
                });
                assert_eq!(panel.inner, 7);
            });
        });
        let decoration = output
            .shapes
            .iter()
            .position(|s| matches!(s.shape, egui::Shape::Vec(_)))
            .unwrap();
        let text = output
            .shapes
            .iter()
            .position(
                |s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "Glass content"),
            )
            .unwrap();
        // A frame shadow is also a Vec; use the actual gradient slot for this assertion.
        let gradient = output
            .shapes
            .iter()
            .position(|s| match &s.shape {
                egui::Shape::Vec(shapes) => {
                    shapes.iter().any(|s| matches!(s, egui::Shape::Mesh(_)))
                }
                _ => false,
            })
            .unwrap();
        assert!(decoration < text && gradient < text);
        if let egui::Shape::Vec(shapes) = &output.shapes[gradient].shape {
            let egui::Shape::Mesh(mesh) = &shapes[0] else {
                panic!("缺少实际面层")
            };
            assert!(
                mesh.vertices
                    .iter()
                    .all(|v| (0..3).all(|channel| v.color[channel] <= 48)),
                "深色玻璃面层不能用高亮白色覆盖整张卡片"
            );
        }
        let rect = Rect::from_min_size(egui::pos2(10.0, 12.0), egui::vec2(80.0, 32.0));
        let mesh = gradient_mesh_for_test(rect);
        assert!(mesh.is_valid());
        assert!(
            mesh.vertices
                .iter()
                .all(|v| rect.expand(0.01).contains(v.pos))
        );
        assert!(mesh.vertices.iter().all(|v| {
            let color = egui::Rgba::from(v.color);
            (0..3).all(|channel| color[channel] <= color[3] + 0.004)
        }));
        assert_eq!(mesh.vertices.len(), 29);
    }

    fn gradient_mesh_for_test(rect: Rect) -> egui::Mesh {
        gradient(
            rect,
            14.0,
            Color32::from_white_alpha(19),
            Color32::from_black_alpha(12),
        )
    }
}
