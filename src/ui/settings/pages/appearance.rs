use crate::domain::{config::Config, resources::Snapshot};
use crate::ui::{
    common::{theme, widgets},
    companion::pet::Sprites,
};
use eframe::egui::{self, Color32, RichText};

pub fn show(ui: &mut egui::Ui, config: &mut Config, pets: &mut Sprites, snapshot: &Snapshot) {
    theme::glass_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        section(ui, "桌面视图", |ui| {
            view_choices(ui, config, snapshot, pets)
        });
        let width = ui.available_width();
        let height = ui.available_height().max(100.0);
        let preview_width = (width * 0.43).clamp(230.0, 330.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(width - preview_width - 16.0, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("appearance-options")
                        .auto_shrink([false, false])
                        .max_height(height)
                        .show(ui, |ui| options(ui, config, pets));
                },
            );
            ui.add_space(8.0);
            ui.allocate_ui_with_layout(
                egui::vec2(preview_width, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("appearance-preview-scroll")
                        .auto_shrink([false, false])
                        .max_height(height)
                        .show(ui, |ui| {
                            super::appearance_preview::show(ui, config, snapshot, pets)
                        });
                },
            );
        });
    });
}

fn section(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
    ui.label(RichText::new(title).size(14.0).strong().color(theme::TEXT));
    ui.add_space(12.0);
    content(ui);
    ui.add_space(16.0);
    ui.separator();
    ui.add_space(16.0);
}

fn options(ui: &mut egui::Ui, config: &mut Config, pets: &mut Sprites) {
    section(ui, "宠物形象与交互", |ui| pets.selection(ui, config));
    section(ui, "表面与透明度", |ui| {
        ui.horizontal(|ui| {
            widgets::switch(ui, &mut config.background, "显示背景");
            widgets::switch(ui, &mut config.border, "显示边框");
        });
        ui.add_enabled_ui(config.background && !config.pet_mode, |ui| {
            let mut opacity = (config.opacity as f64 * 100.0).round();
            if widgets::number(ui, "背景不透明度", &mut opacity, 15.0..=100.0, "%") {
                config.opacity = opacity as f32 / 100.0;
            }
        });
        ui.weak("文字保持清晰。两项都关闭时，只绘制数据和指示标志；宠物始终使用透明底色。");
    });
    section(ui, "主题色", |ui| {
        ui.horizontal(|ui| {
            ui.color_edit_button_srgb(&mut config.color);
            ui.label("监测指标的主题色");
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            for (name, color) in [
                ("青绿", [100, 210, 195]),
                ("冰蓝", [113, 177, 250]),
                ("暖橙", [245, 178, 106]),
                ("浅紫", [185, 162, 245]),
            ] {
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(34.0, 30.0), egui::Sense::click());
                ui.painter().circle_filled(
                    rect.center(),
                    11.0,
                    Color32::from_rgb(color[0], color[1], color[2]),
                );
                if config.color == color {
                    ui.painter().circle_stroke(
                        rect.center(),
                        14.0,
                        egui::Stroke::new(1.0_f32, Color32::WHITE),
                    );
                }
                if response.on_hover_text(name).clicked() {
                    config.color = color;
                }
            }
        });
        ui.add_space(12.0);
        widgets::switch(ui, &mut config.status_colors, "使用绿 / 黄 / 红分级颜色");
    });
}

fn view_choices(ui: &mut egui::Ui, config: &mut Config, snapshot: &Snapshot, pets: &mut Sprites) {
    let short = ui.available_height() < 400.0;
    let width = (ui.available_width() - 24.0) / 3.0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        for (label, description, compact, pet) in [
            ("完整卡片", "全部指标与进程详情", false, false),
            ("精简视图", "名称和数值，少占空间", true, false),
            ("桌面宠物", "陪伴工作，悬停快捷操作", false, true),
        ] {
            let selected = config.compact == compact && config.pet_mode == pet;
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(width, if short { 72.0 } else { 100.0 }),
                egui::Sense::click(),
            );
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, label)
            });
            ui.painter().rect_filled(
                rect,
                12,
                Color32::from_rgba_unmultiplied(21, 34, 46, if selected { 90 } else { 55 }),
            );
            if selected || response.hovered() || response.has_focus() {
                ui.painter().rect_stroke(
                    rect,
                    12,
                    egui::Stroke::new(if selected { 1.5_f32 } else { 1.0_f32 }, theme::CHAMPAGNE),
                    egui::StrokeKind::Inside,
                );
            }
            super::appearance_preview::tile(
                ui,
                egui::Rect::from_center_size(
                    rect.center_top() + egui::vec2(0.0, if short { 23.0 } else { 30.0 }),
                    egui::vec2((width - 24.0).min(170.0), if short { 32.0 } else { 46.0 }),
                ),
                config,
                snapshot,
                pets,
                compact,
                pet,
            );
            ui.painter().text(
                rect.center_top() + egui::vec2(0.0, if short { 55.0 } else { 64.0 }),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(14.0),
                theme::TEXT,
            );
            if width > 180.0 && !short {
                ui.painter().text(
                    rect.center_top() + egui::vec2(0.0, 84.0),
                    egui::Align2::CENTER_CENTER,
                    description,
                    egui::FontId::proportional(11.0),
                    theme::MUTED,
                );
            }
            if response.clicked() {
                config.compact = compact;
                config.pet_mode = pet;
            }
        }
    });
    if !short {
        ui.add_space(10.0);
        ui.label(
            RichText::new("三种视图随时切换；双击宠物、托盘或还原图标可回到完整卡片。")
                .size(12.0)
                .color(theme::MUTED),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn visual_tiles_switch_real_modes_at_minimum_window_width() {
        for (label, compact, pet_mode) in [
            ("完整卡片", false, false),
            ("精简视图", true, false),
            ("桌面宠物", false, true),
        ] {
            let ctx = egui::Context::default();
            let mut config = Config {
                compact: true,
                pet_mode: false,
                ..Default::default()
            };
            let render = |events, config: &mut Config| {
                ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(532.0, 200.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            view_choices(ui, config, &Snapshot::default(), &mut Sprites::default())
                        });
                    },
                )
            };
            let out = render(Vec::new(), &mut config);
            let point = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == label => {
                        Some(t.pos + t.galley.size() * 0.5)
                    }
                    _ => None,
                })
                .unwrap();
            for pressed in [true, false] {
                render(
                    vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    &mut config,
                );
            }
            assert_eq!((config.compact, config.pet_mode), (compact, pet_mode));
        }
    }
}
