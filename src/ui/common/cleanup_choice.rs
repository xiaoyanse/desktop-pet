use super::{
    actions, artwork,
    components::layout,
    icons::{self, Icon},
    theme,
};
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
                    Profile::Medium => theme::WARNING,
                    Profile::Heavy => theme::DANGER,
                };
                let response = profile_card(column, profile, selected, color);
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

fn profile_card(
    ui: &mut egui::Ui,
    profile: Profile,
    selected: bool,
    color: Color32,
) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 126.0),
        egui::Sense::hover(),
    );
    let response = ui.interact(
        rect,
        ui.id().with(("cleanup-profile", profile)),
        egui::Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            profile.label(),
        )
    });
    ui.painter().rect_filled(
        rect,
        12,
        Color32::from_rgba_unmultiplied(17, 30, 43, if selected { 120 } else { 55 }),
    );
    ui.painter().rect_stroke(
        rect,
        12,
        egui::Stroke::new(
            if selected { 1.5_f32 } else { 1.0_f32 },
            if selected {
                color
            } else if response.has_focus() {
                theme::CHAMPAGNE
            } else if response.hovered() {
                theme::CHAMPAGNE.gamma_multiply(0.6)
            } else {
                theme::LINE
            },
        ),
        egui::StrokeKind::Inside,
    );
    let (asset, title, context) = match profile {
        Profile::Light => ("cleanup-light", "轻度", "工作中 · 默认推荐"),
        Profile::Medium => ("cleanup-medium", "中度", "缓存整理"),
        Profile::Heavy => ("cleanup-heavy", "重度", "工作后 · 临时缓解"),
    };
    let tint = if ui.is_enabled() {
        Color32::WHITE
    } else {
        Color32::from_white_alpha(75)
    };
    artwork::draw(
        ui.painter(),
        egui::Rect::from_center_size(
            rect.center_top() + egui::vec2(0.0, 34.0),
            egui::Vec2::splat(52.0),
        ),
        asset,
        tint,
    );
    if selected {
        icons::draw(
            ui.painter(),
            egui::Rect::from_center_size(
                rect.right_top() + egui::vec2(-15.0, 15.0),
                egui::Vec2::splat(16.0),
            ),
            Icon::Check,
            tint,
        );
    }
    for (y, text, font, text_color) in [
        (72.0, title.to_owned(), 15.0, color),
        (94.0, context.to_owned(), 11.0, theme::MUTED),
        (
            113.0,
            format!("{} 项操作", profile.operations().len()),
            11.0,
            theme::MUTED,
        ),
    ] {
        ui.painter().text(
            rect.center_top() + egui::vec2(0.0, y),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(font),
            if ui.is_enabled() {
                text_color
            } else {
                theme::MUTED.gamma_multiply(0.5)
            },
        );
    }
    response.on_hover_text(profile.description())
}

pub fn review(ui: &mut egui::Ui, review: &mut Review, enabled: bool) -> Option<CleanupConfig> {
    let operations = review.operations()?;
    ui.add_space(8.0);
    let low_impact = operations.iter().all(|o| !o.auto_sensitive());
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(22, 36, 48, 160))
        .corner_radius(10)
        .inner_margin(12)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("确认本次清理").size(16.0).strong());
            ui.label(operation_names(operations));
            ui.colored_label(
                theme::WARNING,
                if operations == [crate::domain::cleanup::Operation::LowStandby] {
                    "缓存原本也属于可用内存，效果取决于当前状态。"
                } else if low_impact {
                    "内存页合并取决于当前内存内容，可能增加处理器活动。"
                } else {
                    "可能引起应用调页、重新读取磁盘或短暂停顿。建议等待工作告一段落。"
                },
            );
            ui.label(
                RichText::new("Windows 可能要求管理员授权。不会结束进程，也无法修复驱动内存泄漏。")
                    .size(12.0)
                    .color(theme::MUTED),
            );
        });
    ui.add_space(8.0);
    if actions::primary(ui, Icon::Check, "确认清理一次", enabled).clicked() {
        return review.take();
    }
    if actions::secondary(ui, Icon::Cancel, "取消").clicked() {
        review.cancel();
    }
    None
}

pub fn modal(ctx: &egui::Context, pending: &mut Review, enabled: bool) -> Option<CleanupConfig> {
    pending.operations()?;
    let dialog = egui::Modal::new(egui::Id::new("cleanup-confirmation"))
        .frame(layout::dialog_frame())
        .backdrop_color(Color32::from_black_alpha(145))
        .show(ctx, |ui| {
            ui.set_width((ctx.screen_rect().width() - 96.0).min(480.0));
            egui::ScrollArea::vertical()
                .id_salt("cleanup-confirmation-scroll")
                .max_height((ctx.screen_rect().height() - 100.0).max(100.0))
                .show(ui, |ui| review(ui, pending, enabled))
                .inner
        });
    if dialog.should_close() {
        pending.cancel();
        None
    } else {
        dialog.inner
    }
}

#[cfg(test)]
mod tests;
