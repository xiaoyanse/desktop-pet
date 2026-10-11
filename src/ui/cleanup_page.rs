use super::{actions, cleanup_choice, icons::Icon, panel, settings::section, theme};
use crate::{
    application::cleanup::CleanupService,
    domain::{
        cleanup::Operation,
        cleanup_profile::{Review, operation_names},
        config::{Config, Metric, Row},
        resources::Reading,
    },
};
use eframe::egui::{self, Color32, RichText};

pub fn show(
    ui: &mut egui::Ui,
    config: &mut Config,
    service: &mut CleanupService,
    confirm: &mut Review,
) {
    section(ui, "当前内存", |ui| {
        if let Some(status) = &service.status {
            ui.columns(3, |columns| {
                columns[0].weak("物理内存占用");
                columns[0].label(
                    RichText::new(format!("{}%", status.load))
                        .size(28.0)
                        .color(memory_tint(config, status.load)),
                );
                columns[1].weak("可用内存");
                columns[1].label(
                    RichText::new(format!("{:.2} GB", status.available as f64 / 1073741824.0))
                        .size(24.0),
                );
                columns[2].weak("物理内存总量");
                columns[2].label(
                    RichText::new(format!("{:.1} GB", status.total as f64 / 1073741824.0))
                        .size(24.0),
                );
            });
            ui.add_space(8.0);
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
    section(ui, "选择清理档位", |ui| {
        if cleanup_choice::profiles(ui, &mut config.cleanup, !service.busy()) {
            confirm.cancel();
        }
        ui.add_space(8.0);
        ui.weak("轻度默认只清 1 项，优先减少对工作的干扰。档位不保证释放容量。");
        ui.add_space(10.0);
        let beginning = confirm.operations().is_none()
            && actions::primary(
                ui,
                Icon::Clean,
                if service.busy() {
                    "正在处理…"
                } else {
                    "清理一次"
                },
                !service.busy() && !config.cleanup.operations.is_empty(),
            )
            .clicked();
        if beginning {
            confirm.begin(&config.cleanup);
        }
        if let Some(request) = cleanup_choice::review(ui, confirm, !service.busy()) {
            service.start(&request, false);
        }
        if beginning {
            ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
        }
        if !service.message.is_empty() {
            ui.add_space(8.0);
            ui.label(service.message.as_str());
        }
    });
    section(ui, "高级自定义", |ui| {
        egui::CollapsingHeader::new("逐项选择全部 8 种操作").show(ui, |ui| {
            ui.weak("缓存刷新与内存页合并按需选择，不纳入推荐档位。");
            ui.add_enabled_ui(!service.busy(), |ui| expert(ui, config, confirm));
        });
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
                "允许自动执行影响应用或磁盘的清理项（可能停顿）",
            );
        });
        let automatic = config.cleanup.selected(true);
        ui.weak(format!(
            "自动模式实际执行：{}",
            if automatic.is_empty() {
                "无；当前档位需手动执行或明确允许上述操作".into()
            } else {
                operation_names(&automatic)
            }
        ));
        ui.weak("自动规则默认关闭；未明确允许时，仅可自动清低优先级待机缓存和合并相同页。程序运行期间生效。");
        ui.add_space(8.0);
        ui.checkbox(
            &mut config.cleanup.hotkey,
            "启用 Ctrl+Alt+C 打开快速清理面板",
        );
        ui.weak("快捷键和托盘只打开面板；手动清理均先展示本次操作清单，确认后才会执行。");
    });
    ui.weak("清理不是结束进程，也不能修复驱动内核池泄漏。可用内存的变化受其他正在运行的工作影响。");
}
fn expert(ui: &mut egui::Ui, config: &mut Config, confirm: &mut Review) {
    for op in Operation::ALL {
        ui.push_id(op, |ui| {
            let mut enabled = config.cleanup.operations.contains(&op);
            if ui.checkbox(&mut enabled, op.label()).changed() {
                confirm.cancel();
                if enabled {
                    config.cleanup.operations.push(op);
                } else {
                    config.cleanup.operations.retain(|o| *o != op);
                }
            }
            ui.label(
                RichText::new(op.description())
                    .size(12.0)
                    .color(theme::MUTED),
            );
            ui.add_space(9.0);
        });
    }
}
fn memory_tint(config: &Config, load: u32) -> Color32 {
    let item = panel::DisplayRow {
        key: "cleanup-memory".into(),
        label: "内存".into(),
        reading: Reading::percent(load as f64, "Windows"),
        row: config
            .rows
            .iter()
            .find(|r| r.metric == Metric::Memory)
            .cloned()
            .unwrap_or(Row {
                metric: Metric::Memory,
                ..Default::default()
            }),
    };
    panel::tint(panel::level(&item), config.status_colors, theme::ACCENT)
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_status_uses_configured_memory_thresholds_and_color_switch() {
        let mut config = Config::default();
        let row = config
            .rows
            .iter_mut()
            .find(|r| r.metric == Metric::Memory)
            .unwrap();
        row.warning_threshold = 65.0;
        row.threshold = 82.0;
        assert_eq!(
            memory_tint(&config, 64),
            panel::tint(panel::Level::Green, true, theme::ACCENT)
        );
        assert_eq!(
            memory_tint(&config, 65),
            panel::tint(panel::Level::Yellow, true, theme::ACCENT)
        );
        assert_eq!(
            memory_tint(&config, 82),
            panel::tint(panel::Level::Red, true, theme::ACCENT)
        );
        config.status_colors = false;
        assert_eq!(memory_tint(&config, 99), theme::ACCENT);
    }
}
