use super::textures::Texture;
use eframe::egui::{Pos2, Rect, epaint};

fn edge(a: Pos2, b: Pos2, p: Pos2) -> f32 {
    (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
}

fn top_left(a: Pos2, b: Pos2) -> bool {
    b.y < a.y || (b.y == a.y && b.x > a.x)
}

pub(super) fn paint(
    pixels: &mut [u8],
    size: [usize; 2],
    scale: f32,
    clip: Rect,
    mesh: &epaint::Mesh,
    texture: &Texture,
) {
    let clip =
        Rect::from_min_max(clip.min * scale, clip.max * scale).intersect(Rect::from_min_size(
            Pos2::ZERO,
            eframe::egui::vec2(size[0] as f32, size[1] as f32),
        ));
    if !clip.is_positive() {
        return;
    }
    for triangle in mesh.indices.chunks_exact(3) {
        let mut v =
            [triangle[0], triangle[1], triangle[2]].map(|index| mesh.vertices[index as usize]);
        let mut p = v.map(|vertex| vertex.pos * scale);
        let mut area = edge(p[0], p[1], p[2]);
        if area < 0.0 {
            p.swap(1, 2);
            v.swap(1, 2);
            area = -area;
        }
        if area <= 0.0 || !area.is_finite() {
            continue;
        }
        let bounds = Rect::from_min_max(
            Pos2::new(
                p[0].x.min(p[1].x).min(p[2].x),
                p[0].y.min(p[1].y).min(p[2].y),
            ),
            Pos2::new(
                p[0].x.max(p[1].x).max(p[2].x),
                p[0].y.max(p[1].y).max(p[2].y),
            ),
        )
        .intersect(clip);
        if !bounds.is_positive() {
            continue;
        }
        let x0 = (bounds.min.x - 0.5).ceil().max(0.0) as usize;
        let x1 = (bounds.max.x - 0.5).ceil().min(size[0] as f32) as usize;
        let y0 = (bounds.min.y - 0.5).ceil().max(0.0) as usize;
        let y1 = (bounds.max.y - 0.5).ceil().min(size[1] as f32) as usize;
        let edges = [(p[1], p[2]), (p[2], p[0]), (p[0], p[1])];
        let inclusive = edges.map(|(a, b)| top_left(a, b));
        let colors = v.map(|vertex| vertex.color.to_array().map(f32::from));
        for y in y0..y1 {
            for x in x0..x1 {
                let at = Pos2::new(x as f32 + 0.5, y as f32 + 0.5);
                let e = edges.map(|(a, b)| edge(a, b, at));
                if (0..3).any(|i| e[i] < 0.0 || (e[i] == 0.0 && !inclusive[i])) {
                    continue;
                }
                let weight = e.map(|e| e / area);
                let uv = Pos2::new(
                    (0..3).map(|i| v[i].uv.x * weight[i]).sum(),
                    (0..3).map(|i| v[i].uv.y * weight[i]).sum(),
                );
                let sample = texture.sample(uv);
                let source: [f32; 4] = std::array::from_fn(|c| {
                    (0..3).map(|i| colors[i][c] * weight[i]).sum::<f32>() * sample[c] / 255.0
                });
                let dest = &mut pixels[(y * size[0] + x) * 4..][..4];
                for (channel, src) in [source[2], source[1], source[0], source[3]]
                    .into_iter()
                    .enumerate()
                {
                    dest[channel] = (src + dest[channel] as f32 * (1.0 - source[3] / 255.0))
                        .round()
                        .clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}
