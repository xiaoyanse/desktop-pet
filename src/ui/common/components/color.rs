use super::super::{
    icons::{self, Icon},
    theme,
};
use super::{
    fields, numeric,
    surface::{self, Role},
};
use eframe::egui::{self, Color32};

#[derive(Clone)]
struct Draft {
    text: String,
    previous: [u8; 3],
}

pub fn picker(ui: &mut egui::Ui, rgb: &mut [u8; 3]) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(
        egui::vec2(136.0, theme::CONTROL_HEIGHT),
        egui::Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, response.enabled(), "选择颜色")
    });
    surface::paint(ui, &response, Role::Secondary, theme::CONTROL_RADIUS);
    let tint = if response.enabled() {
        Color32::WHITE
    } else {
        Color32::from_white_alpha(75)
    };
    ui.painter().circle_filled(
        rect.left_center() + egui::vec2(17.0, 0.0),
        7.0,
        Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(32.0, 0.0),
        egui::Align2::LEFT_CENTER,
        hex(*rgb),
        egui::FontId::monospace(13.0),
        surface::foreground(&response),
    );
    icons::draw(
        ui.painter(),
        egui::Rect::from_center_size(
            rect.right_center() - egui::vec2(15.0, 0.0),
            egui::Vec2::splat(16.0),
        ),
        Icon::Down,
        tint,
    );
    let popup = response.id.with("palette");
    if response.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup));
    }
    if !response.enabled() && ui.memory(|m| m.is_popup_open(popup)) {
        ui.memory_mut(|m| m.close_popup());
    }
    let before = *rgb;
    egui::popup::popup_below_widget(
        ui,
        popup,
        &response,
        egui::popup::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_width(250.0);
            ui.spacing_mut().interact_size.y = theme::CONTROL_HEIGHT;
            ui.label(egui::RichText::new("选择颜色").size(14.0).strong());
            ui.label(
                egui::RichText::new("预设配色或精确输入 RGB / HEX")
                    .size(12.0)
                    .color(theme::MUTED),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for (label, color) in [
                    ("玉绿", [165, 213, 200]),
                    ("冰蓝", [113, 177, 250]),
                    ("暖橙", [245, 178, 106]),
                    ("浅紫", [185, 162, 245]),
                    ("暖白", [236, 239, 240]),
                    ("柔红", [242, 151, 147]),
                ] {
                    if swatch(ui, color, *rgb == color, label).clicked() {
                        *rgb = color;
                    }
                }
            });
            ui.add_space(4.0);
            for (index, label) in ["红 R", "绿 G", "蓝 B"].iter().enumerate() {
                ui.push_id(index, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(*label).color(theme::MUTED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(132.0, 36.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    let mut v = rgb[index] as f64;
                                    if numeric::compact(ui, &mut v, 0.0..=255.0, "") {
                                        rgb[index] = v as u8;
                                    }
                                },
                            );
                        });
                    })
                });
            }
            let draft_id = popup.with("hex-draft");
            let mut draft = ui
                .ctx()
                .data_mut(|data| data.get_temp::<Draft>(draft_id))
                .unwrap_or_else(|| Draft {
                    text: hex(*rgb),
                    previous: *rgb,
                });
            if draft.previous != *rgb {
                draft = Draft {
                    text: hex(*rgb),
                    previous: *rgb,
                };
            }
            ui.label(egui::RichText::new("HEX").size(12.0).color(theme::MUTED));
            let invalid = parse(&draft.text).is_none();
            let edit = fields::entry(
                ui,
                egui::TextEdit::singleline(&mut draft.text)
                    .char_limit(7)
                    .hint_text("#A5D5C8")
                    .desired_width(ui.available_width()),
                invalid,
            );
            if edit.changed()
                && let Some(color) = parse(&draft.text)
            {
                *rgb = color;
                draft.previous = color;
            }
            if parse(&draft.text).is_none() {
                ui.label(
                    egui::RichText::new("请输入 6 位十六进制颜色")
                        .size(12.0)
                        .color(theme::DANGER),
                );
            }
            ui.ctx().data_mut(|data| data.insert_temp(draft_id, draft));
        },
    );
    if *rgb != before {
        response.mark_changed();
    }
    response.on_hover_text("选择预设颜色，或输入精确颜色值")
}

pub fn swatch(ui: &mut egui::Ui, rgb: [u8; 3], selected: bool, label: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(32.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            response.enabled(),
            selected,
            label,
        )
    });
    surface::paint(
        ui,
        &response,
        if selected {
            Role::Selected
        } else {
            Role::Quiet
        },
        9,
    );
    let color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    ui.painter().circle_filled(
        rect.center(),
        8.0,
        if response.enabled() {
            color
        } else {
            color.gamma_multiply(0.4)
        },
    );
    if selected {
        ui.painter()
            .circle_stroke(rect.center(), 11.0, egui::Stroke::new(1.0_f32, theme::TEXT));
    }
    response.on_hover_text(label)
}
fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}
fn parse(text: &str) -> Option<[u8; 3]> {
    let s = text.strip_prefix('#').unwrap_or(text);
    if s.len() != 6 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some([
        u8::from_str_radix(&s[0..2], 16).ok()?,
        u8::from_str_radix(&s[2..4], 16).ok()?,
        u8::from_str_radix(&s[4..6], 16).ok()?,
    ])
}

#[cfg(test)]
mod tests;
