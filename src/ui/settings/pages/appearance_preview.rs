use crate::domain::{config::Config, resources::Snapshot};
use crate::ui::{
    common::theme,
    companion::pet::{self, Sprites},
    monitoring::{panel, surface},
};
use eframe::egui::{self, Color32, RichText};

pub fn show(ui: &mut egui::Ui, config: &Config, snapshot: &Snapshot, pets: &mut Sprites) {
    crate::ui::common::glass::show(ui, theme::glass_frame().inner_margin(16), |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new("即时预览").size(14.0).strong());
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
        let canvas_height = (ui.available_height() - 32.0).clamp(24.0, 320.0);
        let rows = panel::rows(config, snapshot);
        let monitor_height = ui
            .ctx()
            .input(|i| i.viewport().monitor_size.map(|s| s.y))
            .unwrap_or(900.0);
        let size = if config.pet_mode {
            pet::SIZE * ((canvas_height - 16.0) / pet::SIZE.y).min(3.2)
        } else {
            surface::size(config, &rows, false, monitor_height)
        };
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), canvas_height),
            egui::Sense::hover(),
        );
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 12, Color32::from_rgba_unmultiplied(15, 27, 40, 65));
        painter.rect_stroke(
            rect,
            12,
            egui::Stroke::new(1.0_f32, theme::LINE),
            egui::StrokeKind::Inside,
        );
        let preview = egui::Rect::from_center_size(
            rect.center(),
            egui::vec2(
                size.x.min(rect.width() - 16.0),
                size.y.min(canvas_height - 16.0),
            ),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("appearance-preview")
                .max_rect(preview),
        );
        let color = Color32::from_rgb(config.color[0], config.color[1], config.color[2]);
        if config.pet_mode {
            let scale = size.y / pet::SIZE.y;
            let texture = pets.texture(ui.ctx(), config.pet_kind, false).id();
            child.painter().image(
                texture,
                egui::Rect::from_min_size(
                    preview.min + egui::vec2(5.0, 1.0) * scale,
                    egui::vec2(62.0, 62.0) * scale,
                ),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            let badge = egui::Rect::from_min_size(
                preview.min + egui::vec2(5.0, 64.0) * scale,
                egui::vec2(62.0, 18.0) * scale,
            );
            child.painter().rect_filled(badge, 8, theme::BACKGROUND);
            let (text, tint) = pet::badge(config, snapshot);
            child.painter().text(
                badge.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional((10.0 * scale).clamp(10.0, 14.0)),
                tint,
            );
        } else {
            surface::frame(config).show(&mut child, |ui| {
                ui.set_width(preview.width() - if config.compact { 12.0 } else { 18.0 });
                ui.spacing_mut().item_spacing.y = 2.0;
                if config.compact {
                    egui::ScrollArea::vertical()
                        .max_height((preview.height() - 18.0).max(40.0))
                        .show(ui, |ui| {
                            panel::compact(ui, &rows, config.status_colors, color)
                        });
                } else {
                    egui::ScrollArea::vertical()
                        .max_height((preview.height() - 18.0).max(40.0))
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
        ui.label(
            RichText::new("实时数值 · 外观预览")
                .size(12.0)
                .color(theme::MUTED),
        );
    });
}

pub fn tile(
    ui: &egui::Ui,
    rect: egui::Rect,
    config: &Config,
    snapshot: &Snapshot,
    pets: &mut Sprites,
    compact: bool,
    pet_mode: bool,
) {
    let p = ui.painter().with_clip_rect(rect);
    if pet_mode {
        p.image(
            pets.texture(ui.ctx(), config.pet_kind, false).id(),
            egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(rect.height())),
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        return;
    }
    p.rect_filled(rect, 6, Color32::from_rgba_unmultiplied(12, 23, 34, 130));
    let rows = panel::rows(config, snapshot);
    let base = Color32::from_rgb(config.color[0], config.color[1], config.color[2]);
    if rect.width() < 140.0
        && let Some(row) = rows.first()
    {
        p.text(
            rect.center_top() + egui::vec2(0.0, rect.height() * 0.25),
            egui::Align2::CENTER_CENTER,
            &row.label,
            egui::FontId::proportional(10.0),
            theme::TEXT,
        );
        p.text(
            rect.center_top() + egui::vec2(0.0, rect.height() * 0.68),
            egui::Align2::CENTER_CENTER,
            &row.reading.text,
            egui::FontId::proportional(13.0),
            panel::tint(panel::level(row), config.status_colors, base),
        );
        return;
    }
    let row_height = rect.height() * 0.5;
    for (index, row) in rows.iter().take(2).enumerate() {
        let y = rect.top() + row_height * index as f32 + row_height * 0.4;
        let color = panel::tint(panel::level(row), config.status_colors, base);
        p.text(
            egui::pos2(rect.left() + 6.0, y),
            egui::Align2::LEFT_CENTER,
            &row.label,
            egui::FontId::proportional(9.0),
            theme::TEXT,
        );
        p.text(
            egui::pos2(rect.right() - 6.0, y),
            egui::Align2::RIGHT_CENTER,
            &row.reading.text,
            egui::FontId::proportional(9.0),
            color,
        );
        if !compact && let Some(fraction) = row.reading.fraction {
            let bar = egui::Rect::from_min_size(
                egui::pos2(rect.left() + 6.0, y + row_height * 0.35),
                egui::vec2(rect.width() - 12.0, 2.0),
            );
            p.rect_filled(bar, 1, theme::HOVER);
            p.rect_filled(
                egui::Rect::from_min_size(
                    bar.min,
                    egui::vec2(bar.width() * fraction.clamp(0.0, 1.0), 2.0),
                ),
                1,
                color,
            );
        }
    }
    if rows.is_empty() {
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "未启用指标",
            egui::FontId::proportional(10.0),
            theme::MUTED,
        );
    }
}
