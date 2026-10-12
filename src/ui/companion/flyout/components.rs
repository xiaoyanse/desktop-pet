use super::{Action, render::Content};
use crate::{
    domain::{
        cleanup::{CleanupConfig, Operation},
        cleanup_profile::Profile,
        companion::spiral::{Group, Rect},
        work_timer::{Mode, State, clock},
    },
    ui::{
        common::{
            actions,
            icons::{self, Icon},
            theme,
        },
        monitoring::panel,
    },
};
use eframe::egui::{self, Color32, RichText};
use std::time::Instant;

pub fn rects(group: Group) -> &'static [Rect] {
    match group {
        Group::Resources => &[
            [250.0, 16.0, 510.0, 136.0],
            [270.0, 140.0, 510.0, 344.0],
            [265.0, 350.0, 510.0, 458.0],
        ],
        Group::Office => &[[250.0, 133.0, 510.0, 340.0]],
        Group::Tools => &[[270.0, 75.0, 510.0, 250.0], [270.0, 262.0, 510.0, 446.0]],
    }
}
pub fn show(
    ui: &mut egui::Ui,
    group: Group,
    index: usize,
    content: &mut Content<'_>,
    action: &mut Option<Action>,
) {
    ui.set_width(ui.available_width());
    match (group, index) {
        (Group::Resources, 0) => summary(ui, content),
        (Group::Resources, 1) => cleanup(ui, content, action),
        (Group::Resources, _) => {
            ui.label(RichText::new("桌面视图").size(14.0).strong());
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 8.0) / 2.0;
                for (icon, label, next) in [
                    (Icon::Restore, "完整", Action::Restore),
                    (Icon::Compact, "精简", Action::Compact),
                ] {
                    ui.allocate_ui(egui::vec2(width, 36.0), |ui| {
                        if actions::secondary(ui, icon, label).clicked() {
                            *action = Some(next);
                        }
                    });
                }
            });
        }
        (Group::Office, _) => timer(ui, content, action),
        (Group::Tools, 0) => {
            let (_, r) = ui.allocate_space(egui::vec2(40.0, 40.0));
            icons::draw(ui.painter(), r, Icon::Report, Color32::WHITE);
            ui.label(RichText::new("资源诊断报告").size(16.0).strong());
            ui.label(
                RichText::new("资源 · 进程来源 · 默认脱敏")
                    .size(12.0)
                    .color(theme::MUTED),
            );
            if actions::secondary(ui, Icon::Report, "打开诊断报告").clicked() {
                *action = Some(Action::Report);
            }
        }
        (Group::Tools, _) => {
            let (_, r) = ui.allocate_space(egui::vec2(48.0, 48.0));
            icons::draw(ui.painter(), r, Icon::Settings, Color32::WHITE);
            ui.label(RichText::new("设置工作台").size(18.0).strong());
            ui.label(
                RichText::new("监测 · 外观 · 自动规则")
                    .size(12.0)
                    .color(theme::MUTED),
            );
            if actions::secondary(ui, Icon::Settings, "打开设置").clicked() {
                *action = Some(Action::Settings);
            }
        }
    }
}
fn summary(ui: &mut egui::Ui, content: &Content<'_>) {
    ui.label(RichText::new("资源概况").size(14.0).strong());
    let rows = panel::rows(content.config, content.snapshot);
    for row in rows.iter().take(2) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(&row.label).size(12.0).color(theme::MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(&row.reading.text)
                        .size(14.0)
                        .color(panel::tint(
                            panel::level(row),
                            content.config.status_colors,
                            theme::ACCENT,
                        )),
                );
            });
        });
    }
    if rows.is_empty() {
        ui.label(RichText::new("未启用监测项").size(12.0).color(theme::MUTED));
    }
}
fn cleanup(ui: &mut egui::Ui, content: &mut Content<'_>, action: &mut Option<Action>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("内存清理").size(15.0).strong());
        if icons::bare_button(ui, Icon::General, "打开完整清理中心").clicked() {
            *action = Some(Action::AdvancedClean);
        }
    });
    let feedback = if content.services.cleaner.message.is_empty() {
        Profile::detect(&content.config.cleanup.operations).map_or("自定义清单", |p| p.label())
    } else {
        &content.services.cleaner.message
    };
    ui.add(egui::Label::new(RichText::new(feedback).size(12.0).color(theme::MUTED)).truncate())
        .on_hover_text(feedback);
    ui.add_space(4.0);
    if actions::primary(
        ui,
        Icon::Clean,
        &format!("检查清单 · {} 项", content.config.cleanup.operations.len()),
        !content.services.cleaner.busy() && !content.config.cleanup.operations.is_empty(),
    )
    .clicked()
    {
        content.review.begin(&content.config.cleanup);
    }
    if ui
        .add_enabled_ui(!content.services.cleaner.busy(), |ui| {
            actions::secondary(ui, Icon::Clean, "日常轻清 · 1 项")
        })
        .inner
        .clicked()
    {
        content.review.begin(&CleanupConfig {
            operations: vec![Operation::LowStandby],
            ..Default::default()
        });
    }
}
fn timer(ui: &mut egui::Ui, content: &mut Content<'_>, action: &mut Option<Action>) {
    let timer = &mut content.services.timer;
    let idle = timer.session.state == State::Idle;
    ui.label(
        RichText::new(timer.session.mode.map_or("工作计时", |m| m.label()))
            .size(14.0)
            .color(theme::MUTED),
    );
    let seconds = if idle {
        if content.config.timer.stopwatch {
            0
        } else {
            content.config.timer.focus_minutes * 60
        }
    } else {
        timer.session.displayed_seconds()
    };
    ui.label(RichText::new(clock(seconds)).size(32.0).strong());
    ui.label(
        RichText::new(crate::ui::work_tools::work_timer::status(timer))
            .size(12.0)
            .color(theme::ACCENT),
    );
    let (icon, label) = match timer.session.state {
        State::Running => (Icon::Pause, "暂停计时"),
        State::Paused => (Icon::Play, "继续计时"),
        State::Complete if timer.session.mode == Some(Mode::Focus) => (Icon::Play, "开始休息"),
        _ => (Icon::Play, "开始计时"),
    };
    if actions::primary(ui, icon, label, true).clicked() {
        let now = Instant::now();
        match timer.session.state {
            State::Running => timer.pause(now),
            State::Paused => timer.resume(now),
            State::Complete if timer.session.mode == Some(Mode::Focus) => timer.start_break(now),
            _ => timer.start(
                if content.config.timer.stopwatch {
                    Mode::Stopwatch
                } else {
                    Mode::Focus
                },
                &content.config.timer,
                now,
            ),
        }
    }
    if actions::secondary(ui, Icon::Timer, "详细计时").clicked() {
        *action = Some(Action::Timer);
    }
}
