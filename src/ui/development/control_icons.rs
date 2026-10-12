use crate::ui::common::{
    icons::{self, Icon},
    theme,
};
use eframe::egui::{self, RichText};

const CONTROLS: &[(Icon, &str)] = &[
    (Icon::Settings, "设置入口"),
    (Icon::Pin, "固定"),
    (Icon::Unpin, "取消固定"),
    (Icon::Details, "进程详情"),
    (Icon::Compact, "精简视图"),
    (Icon::Restore, "还原视图"),
    (Icon::Up, "上移"),
    (Icon::Down, "下移"),
    (Icon::Cancel, "关闭"),
    (Icon::Check, "确认"),
    (Icon::Plus, "增加"),
    (Icon::Minus, "减少"),
    (Icon::Minimize, "最小化"),
    (Icon::Maximize, "最大化"),
    (Icon::Play, "开始"),
    (Icon::Pause, "暂停"),
];

/// Native preview of the exact embedded controls and disabled button path.
pub fn show(ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(theme::BACKGROUND)
                .corner_radius(16)
                .inner_margin(28),
        )
        .show(ctx, |ui| {
            ui.label(RichText::new("桌面仪表 · 控制图标").size(26.0).strong());
            ui.label(
                RichText::new("烟蓝玻璃 / 玉绿边缘 / 珍珠白高光")
                    .size(13.0)
                    .color(theme::MUTED),
            );
            ui.add_space(18.0);
            for row in CONTROLS.chunks(4) {
                ui.columns(4, |columns| {
                    for (ui, &(icon, label)) in columns.iter_mut().zip(row) {
                        theme::glass_panel(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(RichText::new(label).size(15.0).strong());
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                for size in [16.0, 20.0, 24.0] {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::Vec2::splat(size),
                                        egui::Sense::hover(),
                                    );
                                    icons::draw(ui.painter(), rect, icon, egui::Color32::WHITE);
                                }
                                ui.add_enabled_ui(false, |ui| {
                                    icons::sized_button(ui, icon, label, 30.0);
                                });
                            });
                            ui.label(
                                RichText::new("16 / 20 / 24 / 禁用")
                                    .size(11.0)
                                    .color(theme::MUTED),
                            );
                        });
                    }
                });
                ui.add_space(12.0);
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "生产切片与控件代码；设置入口沿用玻璃导航图，其余15项采用新控制图集。",
                )
                .size(13.0)
                .color(theme::MUTED),
            );
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn control_gallery_labels_and_embedded_shapes_fit_the_native_preview() {
        let ctx = egui::Context::default();
        theme::setup(&ctx);
        let out = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1040.0, 820.0),
                )),
                ..Default::default()
            },
            show,
        );
        for &(_, label) in CONTROLS {
            let visible = out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == label && s.clip_rect.contains_rect(egui::Rect::from_min_size(t.pos, t.galley.size()))));
            assert!(visible, "{label} must be fully visible");
        }
        assert!(
            out.shapes
                .iter()
                .filter(|s| matches!(&s.shape, egui::Shape::Mesh(_)))
                .count()
                >= CONTROLS.len() * 4
        );
    }
}
