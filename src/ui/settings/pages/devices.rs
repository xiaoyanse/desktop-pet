use crate::{
    domain::{
        config::Config,
        resources::{Device, Snapshot},
    },
    ui::settings::section,
};
use eframe::egui;

pub fn show(ui: &mut egui::Ui, config: &mut Config, snapshot: &Snapshot) {
    section(ui, "显卡", |ui| {
        choose(
            ui,
            "gpu",
            &mut config.gpu,
            &snapshot.gpus,
            "自动：逐块显示所有硬件显卡",
        );
    });
    section(ui, "磁盘卷", |ui| {
        choose(
            ui,
            "disk",
            &mut config.disk,
            &snapshot.disks,
            "自动：第一块本地卷",
        );
        ui.weak("按卷（C:、E: 等）统计空间、活动率与读写速度。");
    });
    section(ui, "网络接口", |ui| {
        choose(
            ui,
            "network",
            &mut config.network,
            &snapshot.networks,
            "自动：已连接物理网卡合计",
        );
        ui.weak("自动模式排除虚拟网卡，避免重复计算。可手动指定 VPN 或虚拟网卡。");
    });
}

fn choose(ui: &mut egui::Ui, id: &str, current: &mut String, devices: &[Device], automatic: &str) {
    let label = if current.is_empty() {
        automatic.to_string()
    } else {
        devices
            .iter()
            .find(|d| &d.id == current)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| format!("{current}（当前未枚举）"))
    };
    egui::ComboBox::from_id_salt(id)
        .width(ui.available_width() - 24.0)
        .selected_text(label)
        .show_ui(ui, |ui| {
            ui.selectable_value(current, String::new(), automatic);
            for device in devices {
                ui.selectable_value(current, device.id.clone(), &device.name);
            }
        });
}
