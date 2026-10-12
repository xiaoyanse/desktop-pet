use super::Page;
use crate::{
    application::report::Document,
    domain::{
        config::Metric,
        report::{Process, Report},
    },
    ui::common::{
        actions,
        icons::{self, Icon},
        theme,
    },
};
use eframe::egui::{self, Color32, RichText};

pub fn empty(ui: &mut egui::Ui) {
    theme::glass_panel(ui, |ui| {
        ui.set_width(ui.available_width());
        let (r, _) = ui.allocate_exact_size(egui::Vec2::splat(72.0), egui::Sense::hover());
        icons::draw(ui.painter(), r, Icon::Report, Color32::WHITE);
        ui.label(
            RichText::new("把异常现场整理成一份报告")
                .size(20.0)
                .strong(),
        );
        ui.label(
            RichText::new(
                "资源与内核池、CPU / 内存排行，以及 Node / Python 的可执行路径和可核实父进程。",
            )
            .size(13.0)
            .color(theme::MUTED),
        );
        ui.add_space(10.0);
        ui.label(
            RichText::new(
                "采集约需 2 秒，按当前设备选择临时采集全部指标。不会改变监测项和刷新设置。",
            )
            .size(12.0)
            .color(theme::MUTED),
        );
    });
}
pub fn show(ui: &mut egui::Ui, page: &mut Page, document: &Document) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(format!("快照 · {}", document.report.captured_at))
                .size(12.0)
                .color(theme::MUTED),
        );
        let (icon, label) = if page.text_view {
            (Icon::Monitor, "查看资源与进程")
        } else {
            (Icon::Report, "查看完整原文")
        };
        if actions::secondary(ui, icon, label).clicked() {
            page.text_view = !page.text_view;
        }
    });
    ui.add_space(10.0);
    if page.text_view {
        theme::glass_panel(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("复制和保存的内容与此处一致，文本只读。")
                    .size(12.0)
                    .color(theme::MUTED),
            );
            let mut text = document.markdown.as_str();
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .frame(false),
            );
        });
        return;
    }
    overview(ui, &document.report);
    process_card(ui, "内存前五名", &document.report.memory_top);
    process_card(ui, "CPU 前五名", &document.report.cpu_top);
    runtime_card(ui, page, &document.report);
}
fn overview(ui: &mut egui::Ui, report: &Report) {
    theme::glass_panel(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("资源快照").size(16.0).strong());
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let width = (ui.available_width() - 24.0) / 4.0;
            for metric in [Metric::Cpu, Metric::Memory, Metric::Paged, Metric::Nonpaged] {
                ui.allocate_ui_with_layout(
                    egui::vec2(width, 55.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_min_width(width);
                        ui.label(RichText::new(metric.label()).size(12.0).color(theme::MUTED));
                        let value = report
                            .resources
                            .iter()
                            .find(|r| r.0 == metric)
                            .map_or("未知", |r| r.1.as_str());
                        ui.label(RichText::new(value).size(19.0).strong());
                    },
                );
            }
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new(format!(
                "{} 个进程 · Node {} · Python {} · {} 项 CPU/内存不可读取",
                report.enumerated,
                report.runtime_count("Node"),
                report.runtime_count("Python"),
                report.unreadable
            ))
            .size(12.0)
            .color(theme::ACCENT),
        );
        egui::CollapsingHeader::new("全部资源读数与数据口径").show(ui,|ui| {
            egui::Grid::new("report-resources").spacing(egui::vec2(30.0,8.0)).show(ui,|ui| {
                for (metric,value) in &report.resources {ui.label(metric.label());ui.label(value);ui.end_row();}
            });
            ui.label(RichText::new("工作集含共享页，不能相加；私有提交与物理驻留不同。内核池的单次快照不能定位驱动泄漏。").size(12.0).color(theme::MUTED));
        });
    });
    ui.add_space(12.0);
}
fn runtime_card(ui: &mut egui::Ui, page: &mut Page, report: &Report) {
    theme::glass_panel(ui, |ui| {
        ui.set_width(ui.available_width());
        let rows = &report.runtimes;
        ui.label(
            RichText::new(format!("Node / Python 来源 · {} 项", rows.len()))
                .size(16.0)
                .strong(),
        );
        ui.label(
            RichText::new(
                "路径代表运行时位置；不能据此认定每一个 MCP 服务。未核实的父进程明确显示未知。",
            )
            .size(12.0)
            .color(theme::MUTED),
        );
        ui.add_space(10.0);
        let pages = rows.len().div_ceil(20).max(1);
        page.runtime_page = page.runtime_page.min(pages - 1);
        if rows.is_empty() {
            ui.label("本次快照没有发现 Node / Python 进程。");
        }
        for p in rows.iter().skip(page.runtime_page * 20).take(20) {
            process_row(ui, p);
        }
        if pages > 1 {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled_ui(page.runtime_page > 0, |ui| {
                        actions::secondary(ui, Icon::Up, "上一页")
                    })
                    .inner
                    .clicked()
                {
                    page.runtime_page -= 1;
                }
                ui.label(
                    RichText::new(format!("{} / {pages}", page.runtime_page + 1))
                        .color(theme::MUTED),
                );
                if ui
                    .add_enabled_ui(page.runtime_page + 1 < pages, |ui| {
                        actions::secondary(ui, Icon::Down, "下一页")
                    })
                    .inner
                    .clicked()
                {
                    page.runtime_page += 1;
                }
            });
        }
        if report.omitted_runtimes > 0 {
            ui.label(format!(
                "还有 {} 项超出报告 512 项上限。",
                report.omitted_runtimes
            ));
        }
    });
}
fn process_card(ui: &mut egui::Ui, title: &str, rows: &[Process]) {
    theme::glass_panel(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(title).size(16.0).strong());
        ui.add_space(8.0);
        if rows.is_empty() {
            ui.label("无可读取数据，权限受限或采集期间进程发生变化。");
        }
        for p in rows {
            process_row(ui, p);
        }
    });
    ui.add_space(12.0);
}
fn process_row(ui: &mut egui::Ui, p: &Process) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(&p.name).size(14.0).strong());
        ui.label(
            RichText::new(format!("PID {}", p.pid))
                .size(11.0)
                .color(theme::MUTED),
        );
        let cpu = p.cpu.map_or("未知".into(), |v| format!("{v:.2}%"));
        let memory = p.working_set.map_or("未知".into(), |v| {
            format!("{:.1} MiB", v as f64 / 1048576.0)
        });
        ui.label(
            RichText::new(format!("CPU {cpu} · 工作集 {memory}"))
                .size(12.0)
                .color(theme::ACCENT),
        );
    });
    ui.label(
        RichText::new(p.path.as_str())
            .size(11.0)
            .color(theme::MUTED),
    );
    let parent = p.parent.as_ref().map_or(
        "父进程：未核实（权限 / 退出 / 时间不符）".into(),
        |(pid, name)| format!("父进程：{name} · PID {pid}"),
    );
    ui.label(RichText::new(parent).size(11.0).color(theme::MUTED));
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(6.0);
}
