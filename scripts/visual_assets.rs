use image::{Rgba, RgbaImage, imageops};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
};

pub fn build(out: &Path) {
    const ROOT: &str = "assets/visual/";
    println!("cargo:rerun-if-changed={ROOT}sprites.json");
    let spec: Value =
        serde_json::from_slice(&std::fs::read(format!("{ROOT}sprites.json")).unwrap())
            .expect("视觉素材清单必须是有效 JSON");
    assert_eq!(spec["version"], 1);
    let requested: HashSet<_> = spec["runtime"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_str().unwrap())
        .collect();
    let mut images = BTreeMap::new();
    let mut records = Vec::new();
    for atlas in spec["atlases"].as_array().unwrap() {
        let path = format!("{ROOT}{}", atlas["file"].as_str().unwrap());
        println!("cargo:rerun-if-changed={path}");
        let source = image::open(&path).expect("生成图集必须存在").to_rgba8();
        let cols = atlas["grid"][0].as_u64().unwrap() as u32;
        let rows = atlas["grid"][1].as_u64().unwrap() as u32;
        assert!(cols > 0 && rows > 0 && source.width() >= cols && source.height() >= rows);
        let names = atlas["names"].as_array().unwrap();
        assert_eq!(names.len(), (cols * rows) as usize);
        for (index, name) in names.iter().enumerate() {
            let name = name.as_str().unwrap();
            assert!(name.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'));
            let col = index as u32 % cols;
            let row = index as u32 / cols;
            let [left, right] = [
                source.width() * col / cols,
                source.width() * (col + 1) / cols,
            ];
            let [top, bottom] = [
                source.height() * row / rows,
                source.height() * (row + 1) / rows,
            ];
            let mut cell =
                imageops::crop_imm(&source, left, top, right - left, bottom - top).to_image();
            let isolate = atlas["isolate"].as_bool().unwrap_or(false)
                || atlas["isolate_names"]
                    .as_array()
                    .is_some_and(|names| names.iter().any(|entry| entry.as_str() == Some(name)));
            let discarded = if isolate { isolate_glyph(&mut cell) } else { 0 };
            let transparent = cell.pixels().filter(|p| p[3] < 16).count();
            assert!(transparent * 10 > cell.len() / 4, "{name} 缺少透明留白");
            let bounds = opaque_bounds(&cell);
            let core_edge = (0..cell.width())
                .flat_map(|x| [(x, 0), (x, cell.height() - 1)])
                .chain((0..cell.height()).flat_map(|y| [(0, y), (cell.width() - 1, y)]))
                .filter(|&(x, y)| cell.get_pixel(x, y)[3] >= 180)
                .count();
            let clear_edges = core_edge * 20 < (cell.width() + cell.height()) as usize * 2;
            assert!(
                clear_edges || !requested.contains(name),
                "{name} 将用于当前界面，但透明边缘未通过，必须重新生成/调整切片"
            );
            let trimmed = if atlas["trim"].as_bool().unwrap_or(true) {
                imageops::crop_imm(
                    &cell,
                    bounds[0],
                    bounds[1],
                    bounds[2] - bounds[0],
                    bounds[3] - bounds[1],
                )
                .to_image()
            } else {
                cell
            };
            let size = atlas["size"].as_u64().unwrap() as u32;
            let sprite = fit(&trimmed, size);
            sprite.save(out.join(format!("art-{name}.png"))).unwrap();
            std::fs::write(out.join(format!("art-{name}.rgba")), sprite.as_raw()).unwrap();
            records.push(json!({"name":name,"source":atlas["file"],"cell":[left,top,right,bottom],
                "opticalBounds":bounds,"size":[size,size],"transparentPixels":transparent,"coreEdgePixels":core_edge,
                "quality":if clear_edges {"ready"} else {"needs-review"},"runtime":requested.contains(name),"discardedPixels":discarded}));
            assert!(
                images.insert(name.to_owned(), sprite).is_none(),
                "重复素材名：{name}"
            );
        }
    }
    let background = &spec["background"];
    let path = format!("{ROOT}{}", background["file"].as_str().unwrap());
    println!("cargo:rerun-if-changed={path}");
    let size = [
        background["size"][0].as_u64().unwrap() as u32,
        background["size"][1].as_u64().unwrap() as u32,
    ];
    let image = resize_alpha(&image::open(&path).unwrap().to_rgba8(), size[0], size[1]);
    let name = background["name"].as_str().unwrap();
    std::fs::write(out.join(format!("art-{name}.rgba")), image.as_raw()).unwrap();
    images.insert(name.into(), image);
    let mut catalog = String::from("pub static CATALOG: &[Raster] = &[\n");
    let mut runtime_names = HashSet::new();
    let mut bytes = 0;
    for name in spec["runtime"].as_array().unwrap() {
        let name = name.as_str().unwrap();
        assert!(runtime_names.insert(name));
        let image = images.get(name).expect("运行目录引用了不存在的素材");
        bytes += image.len();
        catalog.push_str(&format!("Raster {{ name: \"{name}\", size: [{}, {}], rgba: include_bytes!(concat!(env!(\"OUT_DIR\"), \"/art-{name}.rgba\")) }},\n", image.width(),image.height()));
    }
    assert!(bytes <= 3 * 1024 * 1024, "运行素材总量超出 3 MiB 预算");
    catalog.push_str("];\n");
    std::fs::write(out.join("artwork_data.rs"), catalog).unwrap();
    std::fs::write(out.join("artwork-manifest.json"), serde_json::to_vec_pretty(&json!({"version":1,"sprites":records,"runtimeBytes":bytes,"runtimeCount":runtime_names.len()})).unwrap()).unwrap();
    let brand = images.get(spec["brand"].as_str().unwrap()).unwrap();
    for size in [32, 64] {
        let icon = resize_alpha(brand, size, size);
        icon.save(out.join(format!("icon-{size}.png"))).unwrap();
        std::fs::write(out.join(format!("icon-{size}.rgba")), icon.as_raw()).unwrap();
    }
    contact_sheet(out, &images);
}

// Control glyphs use strong-alpha connected silhouettes as cut boundaries.
// Retain substantial disconnected pieces (pause bars/list bullets), plus a
// narrow antialias fringe; do not package haze or detached generated speckles.
fn isolate_glyph(image: &mut RgbaImage) -> usize {
    let (w, h) = (image.width() as usize, image.height() as usize);
    let mut seen = vec![false; w * h];
    let mut components: Vec<Vec<usize>> = Vec::new();
    for seed in 0..w * h {
        if seen[seed] || image.as_raw()[seed * 4 + 3] < 200 {
            continue;
        }
        let mut component = vec![seed];
        seen[seed] = true;
        let mut next = 0;
        while next < component.len() {
            let p = component[next];
            next += 1;
            let (x, y) = (p % w, p / w);
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let (nx, ny) = (x as isize + dx, y as isize + dy);
                if nx < 0 || ny < 0 || nx >= w as isize || ny >= h as isize {
                    continue;
                }
                let q = ny as usize * w + nx as usize;
                if !seen[q] && image.as_raw()[q * 4 + 3] >= 200 {
                    seen[q] = true;
                    component.push(q);
                }
            }
        }
        components.push(component);
    }
    let largest = components
        .iter()
        .map(Vec::len)
        .max()
        .expect("控制图标缺少可切的主体");
    assert!(largest > w * h / 200, "控制图标主体太小");
    let mut mask = vec![false; w * h];
    for component in components.iter().filter(|c| c.len() * 8 >= largest) {
        for p in component {
            let (x, y) = (p % w, p / w);
            for dy in -3_isize..=3 {
                for dx in -3_isize..=3 {
                    let (nx, ny) = (x as isize + dx, y as isize + dy);
                    if dx * dx + dy * dy <= 9
                        && nx >= 0
                        && ny >= 0
                        && nx < w as isize
                        && ny < h as isize
                    {
                        mask[ny as usize * w + nx as usize] = true;
                    }
                }
            }
        }
    }
    let mut discarded = 0;
    for (i, p) in image.pixels_mut().enumerate() {
        if !mask[i] {
            discarded += usize::from(p[3] > 0);
            *p = Rgba([0; 4]);
        }
    }
    discarded
}

fn opaque_bounds(image: &RgbaImage) -> [u32; 4] {
    let mut bounds = [image.width(), image.height(), 0, 0];
    for (x, y, _) in image.enumerate_pixels().filter(|(_, _, p)| p[3] >= 128) {
        bounds = [
            bounds[0].min(x),
            bounds[1].min(y),
            bounds[2].max(x + 1),
            bounds[3].max(y + 1),
        ];
    }
    assert!(
        bounds[2] > bounds[0] && bounds[3] > bounds[1],
        "图集包含空单元"
    );
    let margin = image.width().min(image.height()) / 20;
    [
        bounds[0].saturating_sub(margin),
        bounds[1].saturating_sub(margin),
        (bounds[2] + margin).min(image.width()),
        (bounds[3] + margin).min(image.height()),
    ]
}

fn fit(source: &RgbaImage, size: u32) -> RgbaImage {
    let inner = size - size / 8;
    let ratio = inner as f32 / source.width().max(source.height()) as f32;
    let sprite = resize_alpha(
        source,
        (source.width() as f32 * ratio).round().max(1.0) as u32,
        (source.height() as f32 * ratio).round().max(1.0) as u32,
    );
    let mut canvas = RgbaImage::new(size, size);
    imageops::overlay(
        &mut canvas,
        &sprite,
        ((size - sprite.width()) / 2) as i64,
        ((size - sprite.height()) / 2) as i64,
    );
    canvas
}

fn resize_alpha(source: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    let mut premultiplied = source.clone();
    for p in premultiplied.pixels_mut() {
        for channel in 0..3 {
            p[channel] = (p[channel] as u32 * p[3] as u32 / 255) as u8;
        }
    }
    let mut resized = imageops::resize(
        &premultiplied,
        width,
        height,
        imageops::FilterType::Lanczos3,
    );
    for p in resized.pixels_mut() {
        for channel in 0..3 {
            p[channel] = if p[3] == 0 {
                0
            } else {
                (p[channel] as u32 * 255 / p[3] as u32).min(255) as u8
            };
        }
    }
    resized
}

fn contact_sheet(out: &Path, images: &BTreeMap<String, RgbaImage>) {
    let count = images.keys().filter(|name| *name != "workspace").count() as u32;
    let mut sheet =
        RgbaImage::from_pixel(640, count.div_ceil(8).max(1) * 80, Rgba([18, 27, 38, 255]));
    let mut index = 0;
    for (name, image) in images {
        if name == "workspace" {
            continue;
        }
        let x = (index % 8) * 80;
        let y = (index / 8) * 80;
        imageops::overlay(&mut sheet, &resize_alpha(image, 32, 32), x + 4, y + 12);
        imageops::overlay(&mut sheet, &resize_alpha(image, 20, 20), x + 44, y + 18);
        index += 1;
    }
    sheet.save(out.join("artwork-preview.png")).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cutting_keeps_disconnected_pause_bars_and_antialias_but_removes_stray_pixels() {
        let mut source = RgbaImage::new(64, 64);
        for x in [20, 36] {
            for y in 18..46 {
                for dx in 0..8 {
                    source.put_pixel(x + dx, y, Rgba([240, 240, 240, 255]));
                }
            }
        }
        source.put_pixel(19, 25, Rgba([240, 240, 240, 80]));
        source.put_pixel(4, 4, Rgba([255, 255, 255, 255]));
        source.put_pixel(56, 55, Rgba([255, 255, 255, 80]));
        assert_eq!(isolate_glyph(&mut source), 2);
        assert_eq!(source.get_pixel(19, 25)[3], 80);
        assert_eq!(source.get_pixel(22, 30)[3], 255);
        assert_eq!(source.get_pixel(38, 30)[3], 255);
        assert_eq!(source.get_pixel(4, 4)[3], 0);
        assert_eq!(source.get_pixel(56, 55)[3], 0);
    }
    #[test]
    fn transparent_resize_preserves_a_white_glyph_without_black_fringe() {
        let mut source = RgbaImage::new(64, 64);
        for y in 12..52 {
            for x in 24..40 {
                source.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
        let resized = fit(&source, 20);
        assert!(resized.pixels().any(|p| p[3] > 0 && p[3] < 255));
        for pixel in resized.pixels().filter(|p| p[3] >= 32) {
            assert!(pixel[0] >= 248 && pixel[1] >= 248 && pixel[2] >= 248);
        }
    }
}
