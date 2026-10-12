use crate::ui::{
    common::{actions, cleanup_choice, icons::Icon, theme, widgets},
    monitoring::panel,
    settings::section,
};
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
    let body_height = (ui.available_height() - 100.0).max(80.0);
    egui::ScrollArea::vertical()
        .id_salt("cleanup-options")
        .max_height(body_height)
        .auto_shrink([false, false])
        .show(ui, |ui| options(ui, config, service, confirm));
    ui.add_space(12.0);
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(20, 36, 47, 160))
        .corner_radius(14)
        .inner_margin(10)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(format!(
                    "本次清单 · {} 项操作",
                    config.cleanup.operations.len()
                ))
                .size(12.0)
                .color(theme::MUTED),
            );
            let pending = confirm.operations().is_some();
            if actions::primary(
                ui,
                Icon::Clean,
                if service.busy() {
                    "正在处理…"
                } else if pending {
                    "等待确认"
                } else {
                    "清理一次"
                },
                !service.busy() && !pending && !config.cleanup.operations.is_empty(),
            )
            .clicked()
            {
                confirm.begin(&config.cleanup);
            }
        });
    if let Some(request) = cleanup_choice::modal(ui.ctx(), confirm, !service.busy()) {
        service.start(&request, false);
    }
}

fn options(ui: &mut egui::Ui, config: &mut Config, service: &CleanupService, confirm: &mut Review) {
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
            widgets::switch(
                ui,
                &mut config.cleanup.auto_load,
                "物理内存占用达到阈值时清理",
            );
            ui.add_enabled_ui(config.cleanup.auto_load, |ui| {
                let mut threshold = config.cleanup.load_percent as u64;
                if widgets::integer(ui, "占用阈值", &mut threshold, 1..=100, "%") {
                    config.cleanup.load_percent = threshold as u32;
                }
            });
            ui.add_space(8.0);
            widgets::switch(ui, &mut config.cleanup.auto_interval, "按时间间隔清理");
            ui.add_enabled_ui(config.cleanup.auto_interval, |ui| {
                widgets::integer(
                    ui,
                    "清理间隔",
                    &mut config.cleanup.interval_minutes,
                    1..=1440,
                    " 分钟",
                );
            });
            ui.add_space(8.0);
            widgets::integer(
                ui,
                "最短冷却",
                &mut config.cleanup.cooldown_minutes,
                1..=1440,
                " 分钟",
            );
            ui.add_space(8.0);
            widgets::switch(
                ui,
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
        widgets::switch(
            ui,
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
            if widgets::switch(ui, &mut enabled, op.label()).changed() {
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
    for (index, report) in service.reports.iter().enumerate().rev() {
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
                ui.add_space(10.0);
                egui::Grid::new(("cleanup-result-table", index, report.at))
                    .num_columns(3)
                    .striped(true)
                    .spacing(egui::vec2(14.0, 10.0))
                    .show(ui, |ui| {
                        for name in ["操作", "结果", "说明"] {
                            ui.label(RichText::new(name).small().color(theme::MUTED));
                        }
                        ui.end_row();
                        for result in &report.results {
                            ui.add(egui::Label::new(result.operation.label()).wrap());
                            ui.colored_label(
                                if result.success {
                                    Color32::from_rgb(90, 214, 145)
                                } else {
                                    Color32::from_rgb(255, 102, 112)
                                },
                                if result.success { "成功" } else { "失败" },
                            );
                            ui.add(
                                egui::Label::new(RichText::new(&result.detail).color(theme::MUTED))
                                    .wrap(),
                            );
                            ui.end_row();
                        }
                    });
            },
        );
    }
    ui.weak("保留最近 50 次操作。按本机时间显示，内存变化不等同于各操作的独立收益。");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            cleanup::{CleanupReport, MemoryStatus},
            resources::Snapshot,
        },
        interfaces::{CleanupHistory, MemoryCleaner, Wake},
    };

    struct NoSideEffects;
    impl MemoryCleaner for NoSideEffects {
        fn request(
            &self,
            _: Vec<Operation>,
            _: bool,
            _: Wake,
        ) -> std::sync::mpsc::Receiver<Result<CleanupReport, String>> {
            panic!("opening the review must not execute cleanup");
        }
        fn elevated(&self) -> bool {
            false
        }
        fn inspect(&self) -> Result<MemoryStatus, String> {
            panic!("UI rendering must not inspect the backend");
        }
    }
    impl CleanupHistory for NoSideEffects {
        fn load(&self) -> Result<Vec<CleanupReport>, String> {
            Ok(Vec::new())
        }
        fn save(&self, _: &[CleanupReport]) -> Result<(), String> {
            panic!("UI rendering must not write cleanup history");
        }
    }

    #[test]
    fn primary_remains_visible_while_options_scroll_and_only_opens_review() {
        for size in [egui::vec2(840.0, 600.0), egui::vec2(1040.0, 820.0)] {
            let ctx = egui::Context::default();
            theme::setup(&ctx);
            let mut config = Config::default();
            let before = serde_json::to_string(&config).unwrap();
            let mut review = Review::default();
            let mut service = CleanupService::new(
                Box::new(NoSideEffects),
                Box::new(NoSideEffects),
                std::sync::Arc::new(|| {}),
            );
            service.status = Some(MemoryStatus {
                total: 32 * 1073741824,
                available: 3 * 1073741824,
                load: 90,
                ..Default::default()
            });
            let mut render = |ctx: &egui::Context| {
                super::super::super::shell::window_chrome::show(ctx);
                super::super::super::shell::navigation::navigation(ctx, 4, &config);
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().inner_margin(28))
                    .show(ctx, |ui| {
                        super::super::super::shell::navigation::header(
                            ui,
                            4,
                            &config,
                            &Snapshot::default(),
                        );
                        show(ui, &mut config, &mut service, &mut review);
                    });
            };
            let input = |events| egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                events,
                ..Default::default()
            };
            let output = ctx.run(input(vec![]), &mut render);
            let button_position = |output: &egui::FullOutput| {
                output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == "清理一次" => {
                            let bounds = egui::Rect::from_min_size(text.pos, text.galley.size());
                            assert!(shape.clip_rect.contains_rect(bounds));
                            assert!(
                                egui::Rect::from_min_size(egui::Pos2::ZERO, size)
                                    .contains_rect(bounds)
                            );
                            Some(bounds.center())
                        }
                        _ => None,
                    })
                    .expect("primary action must be visible")
            };
            let pos = button_position(&output);
            let output = ctx.run(
                input(vec![
                    egui::Event::PointerMoved(egui::pos2(500.0, 280.0)),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(0.0, -400.0),
                        modifiers: egui::Modifiers::NONE,
                    },
                ]),
                &mut render,
            );
            assert_eq!(
                button_position(&output),
                pos,
                "scrolling options must not move the primary"
            );
            for pressed in [true, false] {
                let _ = ctx.run(
                    input(vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ]),
                    &mut render,
                );
            }
            assert_eq!(
                review.operations(),
                Some(config.cleanup.operations.as_slice())
            );
            assert!(!service.busy());
            assert_eq!(serde_json::to_string(&config).unwrap(), before);
        }
    }

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
