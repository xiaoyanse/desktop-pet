use crate::{
    application::Services,
    domain::config::Config,
    ui::{common::widgets, settings::section},
};
use eframe::egui;

pub fn show(ui: &mut egui::Ui, config: &mut Config, message: &mut String, services: &mut Services) {
    if let Some(updates) = &mut services.updates {
        super::updates::show(ui, &mut config.updates, updates);
    }
    section(ui, "刷新", |ui| {
        ui.horizontal(|ui| {
            let width = (ui.available_width().min(320.0) - 16.0) / 3.0;
            for seconds in [1, 2, 5] {
                if widgets::tab(
                    ui,
                    &format!("{seconds} 秒"),
                    config.seconds == seconds,
                    width,
                )
                .clicked()
                {
                    config.seconds = seconds;
                }
            }
        });
        ui.weak("建议 2 秒；5 秒进一步减少常驻开销。");
    });
    section(ui, "压力提示与贴边", |ui| {
        widgets::switch(ui, &mut config.warnings, "连续超限 10 秒显示压力标记");
        widgets::switch(ui, &mut config.snap_edges, "靠近屏幕四边自动贴边");
        ui.weak("无论是否贴边，整个窗口均限制在屏幕工作区内；切换视图后仍保持可见。");
    });
    section(ui, "启动与恢复", |ui| {
        let mut autostart = services.desktop.autostart();
        if widgets::switch(ui, &mut autostart, "登录 Windows 时启动").changed() {
            *message = match services.desktop.set_autostart(autostart) {
                Ok(()) => "开机启动设置已更新".into(),
                Err(e) => e,
            };
        }
        ui.weak("首次默认关闭。保持可执行文件位置不变。");
        ui.add_space(10.0);
        ui.label("悬停显示图标操作栏；拖动数据区域移动。固定仅限制位置。鼠标穿透在托盘开启，单击托盘或 Ctrl+Alt+M 恢复完整视图。");
    });
}
