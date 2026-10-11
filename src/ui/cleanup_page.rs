use super::{
    icons::{self, Icon},
    settings::section,
};
use crate::{
    application::cleanup::CleanupService,
    domain::{cleanup::Operation, config::Config},
};
use eframe::egui::{self, Color32, RichText};

pub fn show(
    ui: &mut egui::Ui,
    config: &mut Config,
    service: &mut CleanupService,
    confirm: &mut bool,
) {
    section(ui, "当前内存", |ui| {
        if let Some(status) = &service.status {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("{}%", status.load)).size(28.0).color(
                    if status.load >= 90 {
                        Color32::from_rgb(255, 102, 112)
                    } else {
                        Color32::from_rgb(100, 210, 195)
                    },
                ));
                ui.label(format!(
                    "可用 {:.2} / {:.2} GB",
                    status.available as f64 / 1073741824.0,
                    status.total as f64 / 1073741824.0
                ));
            });
            ui.weak(format!(
                "提交 {:.2} / {:.2} GB · 系统缓存 {:.2} GB",
                status.commit as f64 / 1073741824.0,
                status.commit_limit as f64 / 1073741824.0,
                status.cache as f64 / 1073741824.0
            ));
            ui.weak(format!(
                "低优先级待机 {} · 全部待机 {} · 修改页 {}",
                optional(status.standby_low),
                optional(status.standby_all),
                optional(status.modified)
            ));
        } else {
            ui.weak("等待内存状态…");
        }
    });
    section(ui, "清理方式", |ui| {
        for op in Operation::ALL {
            ui.push_id(op, |ui| {
                let mut enabled = config.cleanup.operations.contains(&op);
                if ui.checkbox(&mut enabled, op.label()).changed() {
                    if enabled {
                        config.cleanup.operations.push(op);
                    } else {
                        config.cleanup.operations.retain(|o| *o != op);
                    }
                }
                ui.label(
                    RichText::new(op.description())
                        .size(11.5)
                        .color(Color32::from_rgb(146, 167, 182)),
                );
                ui.add_space(7.0);
            });
        }
        ui.separator();
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled_ui(
                    !service.busy() && !config.cleanup.operations.is_empty(),
                    |ui| icons::button(ui, Icon::Clean, "执行所选清理方式"),
                )
                .inner
                .clicked()
            {
                *confirm = true;
            }
            ui.label(if service.busy() {
                "正在处理，界面仍可操作"
            } else {
                "点击清扫图标执行一次"
            });
        });
        if *confirm {
            ui.add_space(10.0);
            ui.colored_label(
                Color32::from_rgb(248, 200, 82),
                "将执行以下已选操作，部分操作可能导致应用调页或磁盘活动。",
            );
            ui.label(
                config
                    .cleanup
                    .operations
                    .iter()
                    .map(|o| o.label())
                    .collect::<Vec<_>>()
                    .join("、"),
            );
            ui.horizontal(|ui| {
                if icons::button(ui, Icon::Check, "确认执行一次；Windows 可能要求管理员授权")
                    .clicked()
                {
                    service.start(&config.cleanup, false);
                    *confirm = false;
                }
                if icons::button(ui, Icon::Cancel, "取消本次操作").clicked() {
                    *confirm = false;
                }
            });
        }
        if !service.message.is_empty() {
            ui.add_space(8.0);
            ui.label(service.message.as_str());
        }
    });
    section(ui, "自动规则", |ui| {
        if !service.elevated() {
            ui.colored_label(
                Color32::from_rgb(248, 200, 82),
                "当前为普通权限。自动清理仅在以管理员身份启动程序后可用，不会周期性弹出授权窗口。",
            );
        }
        ui.add_enabled_ui(service.elevated(), |ui| {
            ui.checkbox(&mut config.cleanup.auto_load, "物理内存占用达到阈值时清理");
            ui.add_enabled(
                config.cleanup.auto_load,
                egui::Slider::new(&mut config.cleanup.load_percent, 1..=100)
                    .text("占用阈值")
                    .suffix("%"),
            );
            ui.checkbox(&mut config.cleanup.auto_interval, "按时间间隔清理");
            ui.add_enabled(
                config.cleanup.auto_interval,
                egui::Slider::new(&mut config.cleanup.interval_minutes, 1..=1440)
                    .logarithmic(true)
                    .text("间隔")
                    .suffix(" 分钟"),
            );
            ui.add(
                egui::Slider::new(&mut config.cleanup.cooldown_minutes, 1..=60)
                    .text("最短冷却")
                    .suffix(" 分钟"),
            );
            ui.checkbox(
                &mut config.cleanup.allow_sensitive_auto,
                "允许自动清理全部待机页及修改页（可能停顿）",
            );
        });
        ui.weak("自动模式使用相同的已选清理项；默认排除全部待机页和修改页。程序运行期间生效，不创建后台服务或绕过 UAC。");
        ui.add_space(8.0);
        ui.checkbox(
            &mut config.cleanup.hotkey,
            "启用 Ctrl+Alt+C 打开快速清理面板",
        );
        ui.weak("快捷键和托盘只打开面板；需再点击清扫图标才会执行。完整清理中心另有确认步骤。");
    });
    ui.weak("清理不是结束进程，也不能修复驱动内核池泄漏。可用内存的变化受其他正在运行的工作影响。");
}
fn optional(bytes: Option<u64>) -> String {
    bytes
        .map(|v| format!("{:.1} MB", v as f64 / 1048576.0))
        .unwrap_or_else(|| "暂不可用".into())
}
pub fn history(ui: &mut egui::Ui, service: &CleanupService) {
    if service.reports.is_empty() {
        section(ui, "尚无清理记录", |ui| {
            ui.label("执行一次清理后，这里会显示各项成功或失败，以及操作前后的内存状态。");
        });
        return;
    }
    for report in service.reports.iter().rev() {
        section(
            ui,
            &format!(
                "{} · {}",
                if report.automatic {
                    "自动清理"
                } else {
                    "手动清理"
                },
                if report.time_label.is_empty() {
                    format!("UTC {}", report.at)
                } else {
                    report.time_label.clone()
                }
            ),
            |ui| {
                ui.label(report.summary());
                for result in &report.results {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(
                            if result.success {
                                Color32::from_rgb(90, 214, 145)
                            } else {
                                Color32::from_rgb(255, 102, 112)
                            },
                            if result.success { "成功" } else { "失败" },
                        );
                        ui.label(result.operation.label());
                        ui.weak(&result.detail);
                    });
                }
            },
        );
    }
    ui.weak("保留最近 50 次操作。按本机时间显示，内存变化不等同于各操作的独立收益。");
}
