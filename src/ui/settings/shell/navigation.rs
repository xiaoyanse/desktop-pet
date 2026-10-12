use crate::domain::{config::Config, resources::Snapshot};
use crate::ui::common::{
    artwork,
    components::surface::{self, Role},
    icons::{self, Icon},
    theme,
};
use eframe::egui::{self, Color32, RichText};
pub const WIDTH: f32 = 228.0;

pub fn navigation(ctx: &egui::Context, tab: usize, config: &Config) -> Option<usize> {
    let mut next = None;
    egui::SidePanel::left("settings-navigation")
        .show_separator_line(false)
        .exact_width(WIDTH)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .stroke(egui::Stroke::NONE)
                .inner_margin(18),
        )
        .show(ctx, |ui| {
            let (brand, _) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 72.0), egui::Sense::hover());
            artwork::draw(
                ui.painter(),
                egui::Rect::from_center_size(
                    brand.min + egui::vec2(25.0, 30.0),
                    egui::vec2(54.0, 54.0),
                ),
                "brand-planet",
                Color32::WHITE,
            );
            ui.painter().text(
                brand.min + egui::vec2(56.0, 13.0),
                egui::Align2::LEFT_TOP,
                "桌面仪表",
                egui::FontId::proportional(22.0),
                theme::TEXT,
            );
            ui.painter().text(
                brand.min + egui::vec2(56.0, 43.0),
                egui::Align2::LEFT_TOP,
                "陪伴高效的每一天",
                egui::FontId::proportional(11.0),
                theme::MUTED,
            );
            ui.add_space(20.0);
            let spacing = ui.spacing().item_spacing.y;
            ui.spacing_mut().item_spacing.y = 0.0;
            let row_height = ((ui.available_height() - 68.0 - 10.0 * 8.0) / 10.0).clamp(24.0, 60.0);
            for (index, icon, name) in [
                (0, Icon::Monitor, "监测项目"),
                (1, Icon::Palette, "外观与宠物"),
                (2, Icon::Devices, "监测设备"),
                (3, Icon::General, "常规设置"),
                (4, Icon::Clean, "内存清理"),
                (5, Icon::History, "清理记录"),
                (6, Icon::Timer, "工作计时"),
                (7, Icon::Report, "诊断报告"),
                (8, Icon::Screenshot, "截图与贴图"),
                (9, Icon::Note, "便签与待办"),
            ] {
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), row_height),
                    egui::Sense::hover(),
                );
                let response = ui.interact(
                    rect,
                    ui.id().with(("navigation", index)),
                    egui::Sense::click(),
                );
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name)
                });
                let selected = tab == index;
                surface::paint(
                    ui,
                    &response,
                    if selected {
                        Role::Selected
                    } else {
                        Role::Quiet
                    },
                    12,
                );
                let color = if selected {
                    Color32::WHITE
                } else {
                    Color32::from_white_alpha(190)
                };
                icons::draw(
                    ui.painter(),
                    egui::Rect::from_center_size(
                        rect.min + egui::vec2(26.0, row_height * 0.5),
                        egui::Vec2::splat((row_height - 8.0).min(42.0)),
                    ),
                    icon,
                    color,
                );
                ui.painter().text(
                    rect.min + egui::vec2(54.0, row_height * 0.5),
                    egui::Align2::LEFT_CENTER,
                    name,
                    egui::FontId::proportional(15.0),
                    if selected { theme::TEXT } else { theme::MUTED },
                );
                if response.clicked() {
                    next = Some(index);
                }
                ui.add_space(8.0);
            }
            ui.spacing_mut().item_spacing.y = spacing;
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.weak(format!("每 {} 秒更新", config.seconds));
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                    ui.painter().circle_filled(r.center(), 3.0, theme::ACCENT);
                    ui.label(RichText::new("本机监测").size(11.0).color(theme::MUTED));
                });
            });
        });
    next
}

pub fn header(ui: &mut egui::Ui, tab: usize, config: &Config, snapshot: &Snapshot) {
    let (title, description) = match tab {
        0 => ("监测项目", "选择需要的数据，按你的工作习惯排列。"),
        1 => ("外观与宠物", "让桌面仪表融入你的工作空间。"),
        2 => ("监测设备", "选择显卡、磁盘卷与网络接口。"),
        3 => ("常规设置", "平衡刷新速度、资源开销与桌面体验。"),
        4 => ("内存清理", "按工作状态选择档位，每次操作都明确可查。"),
        6 => ("工作计时", "专注、休息与记录时长，按自己的节奏工作。"),
        7 => (
            "诊断报告",
            "整理资源现场，查看进程来源，保留一份可分享的快照。",
        ),
        8 => ("截图与贴图", "选择画面、标出重点，让参考随手可见。"),
        9 => ("便签与待办", "把想法留在本地，让下一步按时出现。"),
        _ => ("清理记录", "查看实际结果与前后变化。"),
    };
    ui.label(RichText::new(title).size(34.0).strong());
    ui.label(RichText::new(description).size(14.0).color(theme::MUTED));
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        if tab == 9 {
            ui.label(
                RichText::new("本地保存 · 到期提醒 · 随手查找")
                    .size(12.0)
                    .color(theme::CHAMPAGNE),
            );
            return;
        }
        if tab == 8 {
            ui.label(
                RichText::new("四种选区 · 本地批注 · 置顶参考")
                    .size(12.0)
                    .color(theme::CHAMPAGNE),
            );
            return;
        }
        if tab == 7 {
            ui.label(
                RichText::new("按需生成 · 路径脱敏 · 本地保存")
                    .size(12.0)
                    .color(theme::CHAMPAGNE),
            );
            return;
        }
        if tab == 6 {
            ui.label(
                RichText::new("专注 · 休息 · 正计时")
                    .size(11.0)
                    .color(theme::ACCENT),
            );
            return;
        }
        if tab == 1 {
            ui.label(
                RichText::new("外观随你 · 数据常在")
                    .size(12.0)
                    .color(theme::CHAMPAGNE),
            );
            return;
        }
        ui.label(
            RichText::new(format!(
                "{} 项已启用",
                config.rows.iter().filter(|r| r.enabled).count()
            ))
            .size(11.0)
            .color(theme::ACCENT),
        );
        ui.weak("/ ");
        ui.label(
            RichText::new(if snapshot.sampled_at == 0 {
                "等待采样"
            } else {
                "更改即时预览"
            })
            .size(11.0)
            .color(theme::MUTED),
        );
    });
    ui.add_space(24.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimum_window_keeps_last_navigation_item_above_footer() {
        let ctx = egui::Context::default();
        theme::setup(&ctx);
        let out = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(840.0, 600.0),
                )),
                ..Default::default()
            },
            |ctx| {
                super::super::window_chrome::show(ctx);
                navigation(ctx, 5, &Config::default());
            },
        );
        let bounds = |text: &str| {
            out.shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == text => {
                        Some(egui::Rect::from_min_size(t.pos, t.galley.size()))
                    }
                    _ => None,
                })
                .unwrap()
        };
        let last = bounds("便签与待办");
        let footer = bounds("本机监测");
        assert!(
            last.bottom() + 12.0 < footer.top(),
            "last={last:?},footer={footer:?}"
        );
        assert!(footer.bottom() < 600.0);
    }
}
