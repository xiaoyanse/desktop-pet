use super::{
    icons::{self, Icon},
    pet::Sprites,
    settings::section,
};
use crate::domain::config::Config;
use eframe::egui::{self, Color32};

pub fn show(ui: &mut egui::Ui, config: &mut Config, pets: &mut Sprites) {
    section(ui, "桌面视图", |ui| {
        ui.horizontal(|ui| {
            for (icon, label, compact, pet) in [
                (Icon::Restore, "完整卡片", false, false),
                (Icon::Compact, "精简视图", true, false),
                (Icon::Pet, "桌面宠物", false, true),
            ] {
                if icons::button(ui, icon, label).clicked() {
                    config.compact = compact;
                    config.pet_mode = pet;
                }
                ui.label(format!(
                    "{label}{}",
                    if config.compact == compact && config.pet_mode == pet {
                        " · 当前"
                    } else {
                        ""
                    }
                ));
                ui.add_space(6.0);
            }
        });
        ui.weak(
            "精简保留名称与数值；宠物保留状态数值。悬停快捷面板、双击宠物或单击托盘恢复完整卡片。",
        );
    });
    section(ui, "宠物形象与交互", |ui| pets.selection(ui, config));
    section(ui, "表面与透明度", |ui| {
        ui.horizontal(|ui| {
            ui.checkbox(&mut config.background, "显示背景");
            ui.checkbox(&mut config.border, "显示边框");
        });
        ui.add_enabled(
            config.background,
            egui::Slider::new(&mut config.opacity, 0.15..=1.0)
                .text("背景不透明度")
                .fixed_decimals(2),
        );
        ui.weak("文字保持清晰。两项都关闭时，只绘制数据和指示标志；宠物始终使用透明底色。");
    });
    section(ui, "主题色", |ui| {
        ui.horizontal(|ui| {
            ui.color_edit_button_srgb(&mut config.color);
            ui.label("速度、无分级指标和操作强调色");
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
        ui.checkbox(&mut config.status_colors, "使用绿 / 黄 / 红分级颜色");
    });
}
