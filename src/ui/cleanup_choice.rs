use super::{actions, icons::Icon, theme};
use crate::domain::{
    cleanup::CleanupConfig,
    cleanup_profile::{Profile, Review, operation_names},
};
use eframe::egui::{self, Color32, RichText};

pub fn profiles(ui: &mut egui::Ui, config: &mut CleanupConfig, enabled: bool) -> bool {
    let current = Profile::detect(&config.operations);
    let mut changed = false;
    ui.add_enabled_ui(enabled, |ui| {
        ui.columns(3, |columns| {
            for (column, profile) in columns.iter_mut().zip(Profile::ALL) {
                let selected = current == Some(profile);
                let color = match profile {
                    Profile::Light => theme::ACCENT,
                    Profile::Medium => Color32::from_rgb(248, 200, 82),
                    Profile::Heavy => Color32::from_rgb(255, 128, 137),
                };
                let response = column
                    .add_sized(
                        [column.available_width(), 42.0],
                        egui::Button::new(
                            RichText::new(format!(
                                "{}{}",
                                profile.label(),
                                if selected { " · 已选" } else { "" }
                            ))
                            .color(color)
                            .size(14.0),
                        )
                        .selected(selected)
                        .corner_radius(6)
                        .wrap(),
                    )
                    .on_hover_text(profile.description());
                if response.clicked() {
                    config.operations = profile.operations().to_vec();
                    changed = true;
                }
            }
        });
    });
    ui.add_space(14.0);
    let selected = Profile::detect(&config.operations);
    ui.label(
        RichText::new(format!(
            "{} · {} 项操作",
            selected.map_or("自定义清理", Profile::label),
            config.operations.len()
        ))
        .strong(),
    );
    if let Some(p) = selected {
        ui.weak(p.description());
    }
    ui.weak(format!(
        "包含：{}",
        if config.operations.is_empty() {
            "尚未选择".into()
        } else {
            operation_names(&config.operations)
        }
    ));
    changed
}

pub fn review(ui: &mut egui::Ui, review: &mut Review, enabled: bool) -> Option<CleanupConfig> {
    let operations = review.operations()?;
    ui.add_space(8.0);
    let low_impact = operations.iter().all(|o| !o.auto_sensitive());
    egui::Frame::new()
        .fill(theme::BACKGROUND)
        .corner_radius(10)
        .inner_margin(12)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("确认本次清理").size(16.0).strong());
            ui.label(operation_names(operations));
            ui.colored_label(
                Color32::from_rgb(248, 200, 82),
                if operations == [crate::domain::cleanup::Operation::LowStandby] {
                    "缓存原本也属于可用内存，效果取决于当前状态。"
                } else if low_impact {
                    "内存页合并取决于当前内存内容，可能增加处理器活动。"
                } else {
                    "可能引起应用调页、重新读取磁盘或短暂停顿。建议等待工作告一段落。"
                },
            );
            ui.weak("Windows 可能要求管理员授权。不会结束进程，也无法修复驱动内存泄漏。");
        });
    ui.add_space(8.0);
    if actions::primary(ui, Icon::Check, "确认清理一次", enabled).clicked() {
        return review.take();
    }
    if actions::secondary(ui, "取消").clicked() {
        review.cancel();
    }
    None
}
