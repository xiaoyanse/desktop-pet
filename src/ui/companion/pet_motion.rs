use crate::domain::desktop::PetKind;
use eframe::egui::{self, Color32};

pub fn direction(cursor: Option<[f32; 2]>) -> egui::Vec2 {
    cursor
        .map(|p| {
            egui::vec2(
                ((p[0] - 36.0) / 150.0).clamp(-1.0, 1.0),
                ((p[1] - 25.0) / 150.0).clamp(-1.0, 1.0),
            )
        })
        .unwrap_or_default()
}

pub fn paint(
    painter: &egui::Painter,
    texture: egui::TextureId,
    kind: PetKind,
    gaze: egui::Vec2,
    bounce: f32,
    tilt: f32,
    blink: bool,
) {
    painter.add(mesh(texture, kind, gaze, bounce, tilt, blink));
}
fn mesh(
    texture: egui::TextureId,
    kind: PetKind,
    gaze: egui::Vec2,
    bounce: f32,
    tilt: f32,
    blink: bool,
) -> egui::Mesh {
    let mut mesh = egui::Mesh::with_texture(texture);
    let eye_centres = match kind {
        PetKind::Cat => [egui::pos2(0.46, 0.386), egui::pos2(0.67, 0.4)],
        PetKind::Fox => [egui::pos2(0.38, 0.41), egui::pos2(0.60, 0.42)],
        PetKind::Owl => [egui::pos2(0.34, 0.4), egui::pos2(0.59, 0.41)],
    };
    let rotation = gaze.x * 0.055 + tilt;
    let (sin, cos) = rotation.sin_cos();
    for y in 0..=32 {
        for x in 0..=32 {
            let uv = egui::pos2(x as f32 / 32.0, y as f32 / 32.0);
            let weight = ((0.65 - uv.y) / 0.12).clamp(0.0, 1.0);
            let delta = egui::vec2((uv.x - 0.5) * 62.0, (uv.y - 0.56) * 62.0);
            let rotated = egui::vec2(delta.x * cos - delta.y * sin, delta.x * sin + delta.y * cos);
            let pos = egui::pos2(5.0 + uv.x * 62.0, 1.0 + uv.y * 62.0 + bounce)
                + (rotated - delta + gaze * 1.8) * weight;
            let eye_weight = if blink {
                0.0
            } else {
                eye_centres
                    .iter()
                    .map(|centre| {
                        let delta = uv - *centre;
                        (1.0 - (delta.x / 0.051).powi(2) - (delta.y / 0.061).powi(2)).max(0.0)
                    })
                    .fold(0.0_f32, f32::max)
            };
            mesh.vertices.push(egui::epaint::Vertex {
                pos,
                uv: uv - gaze * (0.018 * eye_weight),
                color: Color32::WHITE,
            });
        }
    }
    for y in 0..32 {
        for x in 0..32 {
            let index = y * 33 + x;
            mesh.add_triangle(index, index + 1, index + 33);
            mesh.add_triangle(index + 1, index + 34, index + 33);
        }
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn head_moves_without_body_and_open_eyes_shift_independently() {
        for kind in PetKind::ALL {
            let centred = mesh(
                egui::TextureId::default(),
                kind,
                egui::Vec2::ZERO,
                0.0,
                0.0,
                false,
            );
            let right = mesh(
                egui::TextureId::default(),
                kind,
                egui::vec2(1.0, 0.0),
                0.0,
                0.0,
                false,
            );
            let closed = mesh(
                egui::TextureId::default(),
                kind,
                egui::vec2(1.0, 0.0),
                0.0,
                0.0,
                true,
            );
            assert!(
                centred
                    .vertices
                    .iter()
                    .zip(&right.vertices)
                    .any(|(a, b)| a.pos != b.pos && a.uv.y < 0.55)
            );
            assert!(
                centred
                    .vertices
                    .iter()
                    .zip(&right.vertices)
                    .filter(|(a, _)| a.uv.y > 0.7)
                    .all(|(a, b)| a.pos == b.pos)
            );
            assert!(
                centred
                    .vertices
                    .iter()
                    .zip(&right.vertices)
                    .any(|(a, b)| a.uv != b.uv)
            );
            assert!(
                centred
                    .vertices
                    .iter()
                    .zip(&closed.vertices)
                    .all(|(a, b)| a.uv == b.uv)
            );
        }
    }
    #[test]
    fn gaze_tracks_all_directions_and_is_bounded() {
        assert_eq!(direction(None), egui::Vec2::ZERO);
        assert_eq!(direction(Some([36.0, 25.0])), egui::Vec2::ZERO);
        assert_eq!(direction(Some([-1000.0, -1000.0])), egui::vec2(-1.0, -1.0));
        assert_eq!(direction(Some([1000.0, 1000.0])), egui::vec2(1.0, 1.0));
    }
}
