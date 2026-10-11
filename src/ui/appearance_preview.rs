use super::{panel, pet::Sprites, settings::section, surface, theme};
use crate::domain::{config::Config, resources::Snapshot};
use eframe::egui::{self, Color32, RichText};

pub fn show(ui: &mut egui::Ui, config: &Config, snapshot: &Snapshot, pets: &mut Sprites) {
    section(ui, "即时预览", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.weak("使用当前采样数据，颜色和透明度即时更新。");
            ui.label(
                RichText::new(if config.pet_mode {
                    "宠物"
                } else if config.compact {
                    "精简"
                } else {
                    "完整"
                })
                .size(11.0)
                .color(theme::ACCENT),
            );
        });
        ui.add_space(12.0);
        let rows = panel::rows(config, snapshot);
        let monitor_height = ui
            .ctx()
            .input(|i| i.viewport().monitor_size.map(|s| s.y))
            .unwrap_or(900.0);
        let size = surface::size(config, &rows, false, monitor_height);
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), size.y.clamp(100.0, 190.0) + 24.0),
            egui::Sense::hover(),
        );
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 8, theme::BACKGROUND);
        for y in 0..=(rect.height() / 20.0) as usize {
            for x in 0..=(rect.width() / 20.0) as usize {
                if (x + y) % 2 == 0 {
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            rect.min + egui::vec2(x as f32 * 20.0, y as f32 * 20.0),
                            egui::vec2(20.0, 20.0),
                        ),
                        0,
                        Color32::from_rgb(34, 40, 43),
                    );
                }
            }
        }
        let preview =
            egui::Rect::from_center_size(rect.center(), egui::vec2(size.x, size.y.min(190.0)));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("appearance-preview")
                .max_rect(preview),
        );
        let color = Color32::from_rgb(config.color[0], config.color[1], config.color[2]);
        if config.pet_mode {
            let texture = pets.texture(ui.ctx(), config.pet_kind, false).id();
            child.painter().image(
                texture,
                egui::Rect::from_min_size(
                    preview.min + egui::vec2(5.0, 1.0),
                    egui::vec2(62.0, 62.0),
                ),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            let badge = egui::Rect::from_min_size(
                preview.min + egui::vec2(5.0, 64.0),
                egui::vec2(62.0, 18.0),
            );
            child.painter().rect_filled(badge, 8, theme::BACKGROUND);
            let (text, tint) = super::pet::badge(config, snapshot);
            child.painter().text(
                badge.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(10.0),
                tint,
            );
        } else {
            surface::frame(config).show(&mut child, |ui| {
                ui.set_width(size.x - if config.compact { 12.0 } else { 18.0 });
                ui.spacing_mut().item_spacing.y = 2.0;
                if config.compact {
                    egui::ScrollArea::vertical()
                        .max_height(176.0)
                        .show(ui, |ui| {
                            panel::compact(ui, &rows, config.status_colors, color)
                        });
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(172.0)
                        .show(ui, |ui| {
                            let history = panel::History::new();
                            for row in &rows {
                                panel::metric(
                                    ui,
                                    row,
                                    &history,
                                    false,
                                    config.status_colors,
                                    color,
                                );
                            }
                            if rows.is_empty() {
                                ui.weak("在监测项目中启用指标");
                            }
                        });
                }
            });
        }
        ui.add_space(8.0);
        ui.weak("棋盘格用于辨认透明区域。这里预览静态外观，趋势与动态交互在桌面查看。");
    });
}
