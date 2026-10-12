use crate::{
    application::report::ReportService,
    domain::config::Config,
    ui::common::{actions, icons::Icon, theme},
};
use eframe::egui::{self, RichText};

pub fn show(ui: &mut egui::Ui, config: &Config, service: &mut ReportService) {
    let busy = service.busy();
    theme::glass_panel(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.allocate_ui(egui::vec2(180.0, 48.0), |ui| {
                let label = if service.document.is_some() {
                    "重新生成"
                } else {
                    "生成诊断报告"
                };
                if actions::primary(ui, Icon::Report, label, !busy).clicked() {
                    service.collect(config);
                }
            });
            if service.can_cancel() {
                if actions::secondary(ui, Icon::Cancel, "取消采集").clicked() {
                    service.cancel();
                }
            } else {
                ui.add_enabled_ui(!busy && service.document.is_some(), |ui| {
                    if actions::secondary(ui, Icon::Copy, "复制报告").clicked() {
                        if let Some(document) = &service.document {
                            ui.ctx().copy_text(document.markdown.clone());
                        }
                        service.message = "已复制当前脱敏报告".into();
                    }
                    if actions::secondary(ui, Icon::Save, "保存到本地").clicked() {
                        service.save();
                    }
                });
            }
        });
        ui.add_space(8.0);
        let message = if service.message.is_empty() {
            "手动采集 · 默认脱敏 · 不含启动参数"
        } else {
            &service.message
        };
        ui.label(RichText::new(message).size(12.0).color(theme::MUTED));
    });
}
