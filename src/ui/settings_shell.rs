use super::{
    icons::{self, Icon},
    theme,
};
use crate::domain::{config::Config, resources::Snapshot};
use eframe::egui::{self, Color32, RichText};

pub fn navigation(
    ctx: &egui::Context,
    tab: usize,
    logo: Option<&egui::TextureHandle>,
    config: &Config,
) -> Option<usize> {
    let mut next = None;
    egui::SidePanel::left("settings-navigation")
        .exact_width(160.0)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(Color32::from_rgb(16, 20, 23))
                .inner_margin(16),
        )
        .show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if let Some(logo) = logo {
                    ui.add(egui::Image::new(logo).fit_to_exact_size(egui::vec2(30.0, 30.0)));
                }
                ui.label(RichText::new("桌面仪表").size(17.0).strong());
            });
            ui.add_space(8.0);
            ui.label(
                RichText::new("DESKTOP METER")
                    .size(10.0)
                    .color(theme::MUTED),
            );
            ui.add_space(32.0);
            ui.label(RichText::new("工作台").size(11.0).color(theme::MUTED));
            ui.add_space(10.0);
            for (index, icon, name) in [
                (0, Icon::Monitor, "监测项目"),
                (1, Icon::Palette, "外观与宠物"),
                (2, Icon::Devices, "监测设备"),
                (3, Icon::General, "常规设置"),
                (4, Icon::Clean, "内存清理"),
                (5, Icon::History, "清理记录"),
            ] {
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 44.0),
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
                if selected || response.hovered() || response.has_focus() {
                    ui.painter().rect_filled(
                        rect,
                        8,
                        if selected {
                            Color32::from_rgb(30, 53, 46)
                        } else {
                            theme::SURFACE
                        },
                    );
                }
                if selected {
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            rect.min + egui::vec2(0.0, 13.0),
                            egui::vec2(3.0, 18.0),
                        ),
                        2,
                        theme::ACCENT,
                    );
                }
                let color = if selected {
                    theme::ACCENT
                } else {
                    theme::MUTED
                };
                icons::draw(
                    ui.painter(),
                    egui::Rect::from_center_size(
                        rect.min + egui::vec2(22.0, 22.0),
                        egui::vec2(18.0, 18.0),
                    ),
                    icon,
                    color,
                );
                ui.painter().text(
                    rect.min + egui::vec2(40.0, 22.0),
                    egui::Align2::LEFT_CENTER,
                    name,
                    egui::FontId::proportional(13.0),
                    if selected { theme::TEXT } else { theme::MUTED },
                );
                if response.clicked() {
                    next = Some(index);
                }
                ui.add_space(6.0);
            }
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

pub fn header(ui: &mut egui::Ui, tab: usize, config: &Config, snapshot: &Snapshot) -> bool {
    let (title, description) = match tab {
        0 => ("监测项目", "选择需要的数据，按你的工作习惯排列。"),
        1 => ("外观与宠物", "让桌面仪表融入你的工作空间。"),
        2 => ("监测设备", "选择显卡、磁盘卷与网络接口。"),
        3 => ("常规设置", "平衡刷新速度、资源开销与桌面体验。"),
        4 => ("内存清理", "按工作状态选择档位，每次操作都明确可查。"),
        _ => ("清理记录", "查看实际结果与前后变化。"),
    };
    let mut close = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(26.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            close = icons::button(ui, Icon::Cancel, "关闭设置").clicked();
        });
    });
    ui.label(RichText::new(description).size(13.0).color(theme::MUTED));
    ui.add_space(16.0);
    ui.horizontal_wrapped(|ui| {
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
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(18.0);
    close
}
