use super::*;
use egui::{
    Color32, ColorImage, ImageData, TextureId, TextureOptions,
    epaint::{ImageDelta, Mesh, Primitive},
};
use std::sync::Arc;

fn delta(id: TextureId, size: [usize; 2], pixels: Vec<Color32>) -> egui::TexturesDelta {
    egui::TexturesDelta {
        set: vec![(
            id,
            ImageDelta::full(
                ImageData::Color(Arc::new(ColorImage { size, pixels })),
                TextureOptions::LINEAR,
            ),
        )],
        free: vec![],
    }
}

fn quad(color: Color32) -> egui::ClippedPrimitive {
    let mut mesh = Mesh::with_texture(TextureId::Managed(1));
    mesh.add_rect_with_uv(
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(2.0, 2.0)),
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        color,
    );
    egui::ClippedPrimitive {
        clip_rect: egui::Rect::from_min_max(egui::pos2(-10.0, -10.0), egui::pos2(10.0, 10.0)),
        primitive: Primitive::Mesh(mesh),
    }
}

fn white() -> Painter {
    let mut painter = Painter::default();
    painter
        .update(&delta(TextureId::Managed(1), [1, 1], vec![Color32::WHITE]))
        .unwrap();
    painter
}

#[test]
fn shared_triangle_edge_is_covered_once_and_winding_does_not_change_alpha() {
    let painter = white();
    let mut forward = quad(Color32::from_rgba_premultiplied(128, 0, 0, 128));
    let mut a = vec![255; 16];
    painter
        .paint(&mut a, [2, 2], 1.0, &[forward.clone()])
        .unwrap();
    assert!(a.chunks_exact(4).all(|pixel| pixel == [0, 0, 128, 128]));
    if let Primitive::Mesh(mesh) = &mut forward.primitive {
        for triangle in mesh.indices.chunks_exact_mut(3) {
            triangle.swap(0, 2);
        }
    }
    let mut b = vec![0; 16];
    painter.paint(&mut b, [2, 2], 1.0, &[forward]).unwrap();
    assert_eq!(a, b);
}

#[test]
fn premultiplied_layers_blend_without_black_borders() {
    let mut pixels = vec![0; 16];
    white()
        .paint(
            &mut pixels,
            [2, 2],
            1.0,
            &[
                quad(Color32::from_rgb(0, 0, 255)),
                quad(Color32::from_rgba_premultiplied(128, 0, 0, 128)),
            ],
        )
        .unwrap();
    assert!(
        pixels
            .chunks_exact(4)
            .all(|pixel| pixel == [127, 0, 128, 255])
    );
}

#[test]
fn clipping_and_physical_scale_keep_pixels_in_their_region() {
    let mut item = quad(Color32::WHITE);
    item.clip_rect = egui::Rect::from_min_max(egui::pos2(1.0, 0.0), egui::pos2(2.0, 2.0));
    let mut pixels = vec![0; 64];
    white().paint(&mut pixels, [4, 4], 2.0, &[item]).unwrap();
    for (index, pixel) in pixels.chunks_exact(4).enumerate() {
        assert_eq!(pixel, if index % 4 >= 2 { &[255; 4] } else { &[0; 4] });
    }
}

#[test]
fn bilinear_sampling_uses_texel_centres_and_clamps_transparent_edges() {
    let mut painter = Painter::default();
    painter
        .update(&delta(
            TextureId::Managed(1),
            [2, 2],
            vec![
                Color32::WHITE,
                Color32::TRANSPARENT,
                Color32::TRANSPARENT,
                Color32::WHITE,
            ],
        ))
        .unwrap();
    let texture = painter.textures.get(TextureId::Managed(1));
    assert_eq!(texture.sample(egui::pos2(0.5, 0.5)), [127.5; 4]);
    assert_eq!(texture.sample(egui::pos2(-5.0, -5.0)), [255.0; 4]);
    assert_eq!(texture.sample(egui::pos2(10.0, 0.0)), [0.0; 4]);
}

#[test]
fn invalid_input_fails_before_clearing_or_allocating_surface() {
    let painter = white();
    let mut pixels = vec![17; 16];
    let mut item = quad(Color32::WHITE);
    if let Primitive::Mesh(mesh) = &mut item.primitive {
        mesh.indices[0] = u32::MAX;
    }
    assert!(painter.paint(&mut pixels, [2, 2], 1.0, &[item]).is_err());
    assert_eq!(pixels, vec![17; 16]);
    assert!(
        painter
            .paint(&mut pixels, [usize::MAX, 2], 1.0, &[])
            .is_err()
    );
    assert!(painter.paint(&mut pixels, [2, 2], f32::NAN, &[]).is_err());
    let mut item = quad(Color32::WHITE);
    if let Primitive::Mesh(mesh) = &mut item.primitive {
        mesh.vertices[0].uv.x = f32::INFINITY;
    }
    assert!(painter.paint(&mut pixels, [2, 2], 1.0, &[item]).is_err());
}

#[test]
fn partial_font_update_preserves_gamma_and_rejects_an_invalid_batch_atomically() {
    let id = TextureId::Managed(1);
    let mut painter = Painter::default();
    painter
        .update(&delta(id, [4, 4], vec![Color32::TRANSPARENT; 16]))
        .unwrap();
    let font = egui::FontImage {
        size: [2, 2],
        pixels: vec![0.0, 0.25, 0.5, 1.0],
    };
    let expected: Vec<_> = font.srgba_pixels(None).collect();
    let patch = ImageDelta::partial([1, 1], ImageData::Font(font), TextureOptions::LINEAR);
    let partial = egui::TexturesDelta {
        set: vec![(id, patch.clone())],
        free: vec![],
    };
    painter.update(&partial).unwrap();
    assert_eq!(&painter.textures.get(id).image.pixels[5..7], &expected[..2]);
    assert_eq!(
        &painter.textures.get(id).image.pixels[9..11],
        &expected[2..]
    );
    let before = painter.textures.get(id).image.clone();
    let mut bad = patch;
    bad.pos = Some([usize::MAX, 0]);
    assert!(
        painter
            .update(&egui::TexturesDelta {
                set: vec![
                    (
                        id,
                        ImageDelta::full(
                            ImageData::Color(Arc::new(ColorImage::new([1, 1], Color32::WHITE))),
                            TextureOptions::LINEAR
                        )
                    ),
                    (id, bad)
                ],
                free: vec![]
            })
            .is_err()
    );
    assert!(Arc::ptr_eq(&before, &painter.textures.get(id).image));
    painter.free(&egui::TexturesDelta {
        set: vec![],
        free: vec![id],
    });
    assert_eq!(painter.texture_bytes(), 0);
}
