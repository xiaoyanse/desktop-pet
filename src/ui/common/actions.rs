use super::{
    components::surface::{self, Role},
    icons::{self, Icon},
    theme,
};
use eframe::egui::{self, Color32};

pub fn primary(ui: &mut egui::Ui, icon: Icon, label: &str, enabled: bool) -> egui::Response {
    primary_role(ui, icon, label, enabled, Role::Primary)
}

pub fn danger(ui: &mut egui::Ui, icon: Icon, label: &str, enabled: bool) -> egui::Response {
    primary_role(ui, icon, label, enabled, Role::Danger)
}

fn primary_role(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    enabled: bool,
    role: Role,
) -> egui::Response {
    let response = ui.add_enabled(
        enabled,
        egui::Button::new("")
            .frame(false)
            .min_size(egui::vec2(ui.available_width(), 48.0)),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    surface::paint(ui, &response, role, theme::CONTROL_RADIUS);
    let foreground = surface::foreground(&response);
    icons::draw(
        ui.painter(),
        egui::Rect::from_center_size(
            egui::pos2(response.rect.left() + 25.0, response.rect.center().y),
            egui::vec2(28.0, 28.0),
        ),
        icon,
        if enabled {
            Color32::WHITE
        } else {
            Color32::from_white_alpha(75)
        },
    );
    ui.painter().with_clip_rect(response.rect).text(
        egui::pos2(response.rect.left() + 50.0, response.rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(17.0),
        foreground,
    );
    response
}

pub fn secondary(ui: &mut egui::Ui, icon: Icon, label: &str) -> egui::Response {
    let text = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(13.0),
        if ui.is_enabled() {
            theme::TEXT
        } else {
            theme::MUTED
        },
    );
    let width = (text.size().x + 50.0).max(110.0).min(ui.available_width());
    let response = ui.add(
        egui::Button::new("")
            .frame(false)
            .min_size(egui::vec2(width, 36.0)),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    surface::paint(ui, &response, Role::Secondary, theme::CONTROL_RADIUS);
    icons::draw(
        ui.painter(),
        egui::Rect::from_center_size(
            response.rect.left_center() + egui::vec2(20.0, 0.0),
            egui::Vec2::splat(18.0),
        ),
        icon,
        if ui.is_enabled() {
            Color32::WHITE
        } else {
            Color32::from_white_alpha(75)
        },
    );
    ui.painter().with_clip_rect(response.rect).galley(
        response.rect.left_center() + egui::vec2(38.0, -text.size().y * 0.5),
        text,
        theme::TEXT,
    );
    response
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
