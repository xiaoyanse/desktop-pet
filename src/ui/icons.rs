use eframe::egui::{self, Color32, Stroke, Vec2};

#[derive(Clone, Copy)]
pub enum Icon {
    Settings,
    Pin,
    Unpin,
    Details,
    Compact,
    Restore,
    Up,
    Down,
    Clean,
    Cancel,
    Check,
    Monitor,
    Palette,
    Devices,
    General,
    History,
    Exit,
    Pet,
}

pub fn button(ui: &mut egui::Ui, icon: Icon, tip: &str) -> egui::Response {
    bare_button(ui, icon, tip).on_hover_text(tip)
}
pub fn bare_button(ui: &mut egui::Ui, icon: Icon, tip: &str) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::hover());
    let response = ui.interact(
        rect,
        ui.id().with(("icon-button", tip)),
        egui::Sense::click(),
    );
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tip));
    if response.hovered() || response.has_focus() {
        ui.painter()
            .rect_filled(rect, 6, Color32::from_rgb(49, 65, 79));
    }
    draw(
        ui.painter(),
        rect.shrink(6.0),
        icon,
        if ui.is_enabled() {
            Color32::from_rgb(207, 227, 233)
        } else {
            Color32::from_rgb(76, 88, 101)
        },
    );
    response
}

pub fn draw(p: &egui::Painter, rect: egui::Rect, icon: Icon, color: Color32) {
    let point = |x: f32, y: f32| rect.min + egui::vec2(x * rect.width(), y * rect.height());
    let s = Stroke::new(1.5_f32, color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment([point(a.0, a.1), point(b.0, b.1)], s);
    };
    let box_ = |a: (f32, f32), b: (f32, f32)| {
        p.rect_stroke(
            egui::Rect::from_min_max(point(a.0, a.1), point(b.0, b.1)),
            2,
            s,
            egui::StrokeKind::Inside,
        );
    };
    match icon {
        Icon::Pet => {
            line((0.1, 0.45), (0.1, 0.02));
            line((0.1, 0.02), (0.35, 0.25));
            line((0.35, 0.25), (0.65, 0.25));
            line((0.65, 0.25), (0.9, 0.02));
            line((0.9, 0.02), (0.9, 0.55));
            line((0.9, 0.55), (0.7, 0.95));
            line((0.7, 0.95), (0.3, 0.95));
            line((0.3, 0.95), (0.1, 0.55));
            p.circle_filled(point(0.3, 0.5), 1.2, color);
            p.circle_filled(point(0.7, 0.5), 1.2, color);
        }
        Icon::Settings => {
            p.circle_stroke(rect.center(), rect.width() * 0.31, s);
            p.circle_stroke(rect.center(), rect.width() * 0.10, s);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let d = egui::vec2(a.cos(), a.sin());
                p.line_segment(
                    [
                        rect.center() + d * rect.width() * 0.30,
                        rect.center() + d * rect.width() * 0.46,
                    ],
                    s,
                );
            }
        }
        Icon::Pin | Icon::Unpin => {
            box_((0.25, 0.05), (0.75, 0.55));
            line((0.1, 0.60), (0.9, 0.60));
            line((0.5, 0.60), (0.5, 1.0));
            if matches!(icon, Icon::Unpin) {
                line((0.0, 1.0), (1.0, 0.0));
            }
        }
        Icon::Compact => {
            box_((0.05, 0.28), (0.95, 0.72));
            line((0.27, 0.42), (0.27, 0.58));
            line((0.5, 0.42), (0.5, 0.58));
            line((0.73, 0.42), (0.73, 0.58));
        }
        Icon::Restore => {
            box_((0.05, 0.05), (0.95, 0.95));
            line((0.20, 0.35), (0.8, 0.35));
            line((0.20, 0.60), (0.8, 0.60));
        }
        Icon::Details | Icon::Monitor | Icon::Devices => {
            box_((0.02, 0.05), (0.98, 0.75));
            line((0.25, 0.95), (0.75, 0.95));
            line((0.5, 0.75), (0.5, 0.95));
            line((0.22, 0.57), (0.22, 0.42));
            line((0.48, 0.57), (0.48, 0.28));
            line((0.74, 0.57), (0.74, 0.18));
        }
        Icon::Up | Icon::Down => {
            let (a, b) = if matches!(icon, Icon::Up) {
                (0.25, 0.75)
            } else {
                (0.75, 0.25)
            };
            line((0.15, b), (0.5, a));
            line((0.5, a), (0.85, b));
        }
        Icon::Clean => {
            line((0.7, 0.02), (0.35, 0.50));
            box_((0.12, 0.52), (0.72, 0.93));
            line((0.32, 0.72), (0.32, 0.93));
            line((0.52, 0.72), (0.52, 0.93));
        }
        Icon::Cancel | Icon::Exit => {
            line((0.15, 0.15), (0.85, 0.85));
            line((0.85, 0.15), (0.15, 0.85));
        }
        Icon::Check => {
            line((0.05, 0.5), (0.38, 0.8));
            line((0.38, 0.8), (0.95, 0.15));
        }
        Icon::Palette => {
            p.circle_stroke(rect.center(), rect.width() * 0.45, s);
            for (x, y) in [(0.3, 0.35), (0.6, 0.25), (0.75, 0.5)] {
                p.circle_filled(point(x, y), 1.5, color);
            }
        }
        Icon::General => {
            for (y, x) in [(0.2, 0.3), (0.5, 0.7), (0.8, 0.4)] {
                line((0.05, y), (0.95, y));
                p.circle_filled(point(x, y), 2.5, color);
            }
        }
        Icon::History => {
            p.circle_stroke(rect.center(), rect.width() * 0.43, s);
            line((0.5, 0.18), (0.5, 0.5));
            line((0.5, 0.5), (0.75, 0.6));
        }
    }
}
