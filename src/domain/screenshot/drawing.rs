use super::{Mark, Tool};

pub(super) fn render(pixels: &mut [u8], size: [u32; 2], marks: &[Mark]) {
    for mark in marks {
        let a = mark.points[0].map(|v| v as i32);
        let b = mark.points[mark.points.len() - 1].map(|v| v as i32);
        match mark.tool {
            Tool::Cover => {
                for y in a[1].min(b[1])..=a[1].max(b[1]) {
                    for x in a[0].min(b[0])..=a[0].max(b[0]) {
                        pixel(pixels, size, [x, y], mark.color);
                    }
                }
            }
            Tool::Rectangle => {
                for (a, b) in [
                    (a, [b[0], a[1]]),
                    ([b[0], a[1]], b),
                    (b, [a[0], b[1]]),
                    ([a[0], b[1]], a),
                ] {
                    line(pixels, size, a, b, mark);
                }
            }
            Tool::Pen => {
                for points in mark.points.windows(2) {
                    line(
                        pixels,
                        size,
                        points[0].map(|v| v as i32),
                        points[1].map(|v| v as i32),
                        mark,
                    );
                }
            }
            Tool::Arrow => {
                line(pixels, size, a, b, mark);
                let dx = (a[0] - b[0]) as f32;
                let dy = (a[1] - b[1]) as f32;
                let len = dx.hypot(dy);
                if len > 1.0 {
                    let length = (mark.width as f32 * 4.0).max(10.0).min(len * 0.45);
                    for sign in [-1.0, 1.0] {
                        let end = [
                            (b[0] as f32 + (dx * 0.866 - dy * 0.5 * sign) / len * length).round()
                                as i32,
                            (b[1] as f32 + (dy * 0.866 + dx * 0.5 * sign) / len * length).round()
                                as i32,
                        ];
                        line(pixels, size, b, end, mark);
                    }
                }
            }
        }
    }
}
fn pixel(pixels: &mut [u8], size: [u32; 2], p: [i32; 2], color: [u8; 3]) {
    if p[0] >= 0 && p[1] >= 0 && (p[0] as u32) < size[0] && (p[1] as u32) < size[1] {
        let index = (p[1] as usize * size[0] as usize + p[0] as usize) * 4;
        pixels[index..index + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
    }
}
fn line(pixels: &mut [u8], size: [u32; 2], mut a: [i32; 2], b: [i32; 2], mark: &Mark) {
    let dx = (b[0] - a[0]).abs();
    let sx = if a[0] < b[0] { 1 } else { -1 };
    let dy = -(b[1] - a[1]).abs();
    let sy = if a[1] < b[1] { 1 } else { -1 };
    let mut error = dx + dy;
    let low = -(mark.width as i32) / 2;
    let high = (mark.width as i32 - 1) / 2;
    let offset = if mark.width.is_multiple_of(2) {
        0.5
    } else {
        0.0
    };
    let radius = mark.width as f64 / 2.0;
    loop {
        for y in low..=high {
            for x in low..=high {
                if (x as f64 + offset).powi(2) + (y as f64 + offset).powi(2) <= radius * radius {
                    pixel(pixels, size, [a[0] + x, a[1] + y], mark.color);
                }
            }
        }
        if a == b {
            break;
        }
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            a[0] += sx;
        }
        if twice <= dx {
            error += dx;
            a[1] += sy;
        }
    }
}
