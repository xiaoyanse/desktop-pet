use crate::domain::resources::{Process, Snapshot};
use crate::ui::common::icons::{self, Icon};
use eframe::egui::{self, Color32, RichText};

pub fn show(ui: &mut egui::Ui, snapshot: &Snapshot, message: &str) -> bool {
    ui.separator();
    ui.add_space(8.0);
    process_list(ui, "CPU 前五", &snapshot.cpu_top, true);
    ui.add_space(12.0);
    process_list(ui, "内存前五 · 物理工作集", &snapshot.memory_top, false);
    ui.label(
        RichText::new(&snapshot.process_status)
            .size(11.0)
            .color(Color32::GRAY),
    );
    ui.add_space(14.0);
    ui.separator();
    ui.add_space(10.0);
    let mut clean = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new("内存清理").strong());
        clean = icons::bare_button(ui, Icon::Clean, "打开清理中心").clicked();
    });
    ui.label(
        RichText::new("八种清理方式、自动规则和操作记录见设置。清理无法修复内核池泄漏。")
            .size(12.0)
            .color(Color32::GRAY),
    );
    if !message.is_empty() {
        ui.label(RichText::new(message).size(12.0));
    }
    clean
}
fn process_list(ui: &mut egui::Ui, title: &str, items: &[Process], cpu: bool) {
    ui.label(RichText::new(title).size(13.0).strong());
    if items.is_empty() {
        ui.label("等待排行样本…");
    }
    for item in items {
        ui.horizontal(|ui| {
            ui.add(egui::Label::new(RichText::new(&item.name).size(11.0)).truncate());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(if cpu {
                        format!("{:.1}%", item.cpu)
                    } else {
                        format!("{:.2} GB", item.memory)
                    })
                    .size(11.0),
                );
                ui.label(
                    RichText::new(item.pid.to_string())
                        .size(10.0)
                        .color(Color32::GRAY),
                );
            });
        });
    }
}
