use super::{
    icons::{self, Icon},
    theme,
};
use eframe::egui::{self, Color32};

pub fn primary(ui: &mut egui::Ui, icon: Icon, label: &str, enabled: bool) -> egui::Response {
    let response = ui.add_enabled(
        enabled,
        egui::Button::new("")
            .fill(if enabled {
                theme::ACCENT
            } else {
                Color32::from_rgb(43, 58, 70)
            })
            .corner_radius(10)
            .min_size(egui::vec2(ui.available_width(), 48.0)),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    let foreground = if enabled {
        Color32::from_rgb(11, 37, 42)
    } else {
        Color32::from_rgb(137, 154, 169)
    };
    icons::draw(
        ui.painter(),
        egui::Rect::from_center_size(
            egui::pos2(response.rect.left() + 25.0, response.rect.center().y),
            egui::vec2(20.0, 20.0),
        ),
        icon,
        foreground,
    );
    ui.painter().text(
        egui::pos2(response.rect.left() + 50.0, response.rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(17.0),
        foreground,
    );
    response
}

pub fn secondary(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(label)
            .corner_radius(8)
            .min_size(egui::vec2(110.0, 36.0)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn primary_is_full_width_large_and_disabled_does_not_click() {
        for enabled in [false, true] {
            let ctx = egui::Context::default();
            let mut center = egui::Pos2::ZERO;
            let mut clicked = false;
            let mut render = |ctx: &egui::Context| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let width = ui.available_width();
                    let r = primary(ui, Icon::Clean, "清理一次", enabled);
                    assert!(r.rect.height() >= 48.0);
                    assert!(r.rect.width() >= width);
                    assert_eq!(r.enabled(), enabled);
                    center = r.rect.center();
                    clicked |= r.clicked();
                });
            };
            let output = ctx.run(egui::RawInput::default(), &mut render);
            let label = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == "清理一次" => Some(t),
                    _ => None,
                })
                .unwrap();
            assert!(label.pos.x >= 40.0, "主按钮文字必须留出图标和间距");
            for pressed in [true, false] {
                let input = egui::RawInput {
                    events: vec![
                        egui::Event::PointerMoved(center),
                        egui::Event::PointerButton {
                            pos: center,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..Default::default()
                };
                let _ = ctx.run(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        clicked |= primary(ui, Icon::Clean, "清理一次", enabled).clicked();
                    });
                });
            }
            assert_eq!(clicked, enabled);
        }
    }
}
