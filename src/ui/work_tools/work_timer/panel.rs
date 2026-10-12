use super::display;
use crate::{
    application::work_timer::WorkTimer,
    domain::work_timer::{Mode, Reminder, State, TimerConfig},
    ui::{
        common::{actions, icons::Icon, theme, widgets},
        settings::section,
    },
};
use eframe::egui::{self, RichText};
use std::time::Instant;

pub fn show(ui: &mut egui::Ui, config: &mut TimerConfig, timer: &mut WorkTimer) {
    let active = matches!(timer.session.state, State::Running | State::Paused);
    section(ui, "工作节奏", |ui| {
        ui.add_enabled_ui(!active, |ui| {
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 8.0) / 2.0;
                for (label, stopwatch) in [("专注与休息", false), ("正计时", true)] {
                    if widgets::tab(ui, label, config.stopwatch == stopwatch, width).clicked() {
                        config.stopwatch = stopwatch;
                        timer.reset();
                    }
                }
            });
        });
        display::ring(ui, timer, config);
        ui.label(
            RichText::new(display::status(timer))
                .size(13.0)
                .color(theme::ACCENT),
        );
        ui.add_space(8.0);
        controls(ui, config, timer);
        if timer.session.state == State::Complete && timer.session.mode == Some(Mode::Focus) {
            ui.label(
                RichText::new(format!(
                    "本次休息 {} 分钟；方案修改用于下一次专注。",
                    timer.session.break_length.as_secs() / 60
                ))
                .size(12.0)
                .color(theme::MUTED),
            );
        }
    });
    if !config.stopwatch {
        section(ui, "时长方案", |ui| {
            ui.add_enabled_ui(!active, |ui| durations(ui, config));
            if active {
                ui.label(
                    RichText::new(format!(
                        "当前方案：专注 {} 分钟 · 休息 {} 分钟。结束或重置后可修改。",
                        config.focus_minutes, config.break_minutes
                    ))
                    .size(12.0)
                    .color(theme::MUTED),
                );
            }
        });
    }
    section(ui, "到时提醒", |ui| {
        ui.horizontal(|ui| {
            let width = (ui.available_width() - 16.0) / 3.0;
            for (label, reminder) in [
                ("静默", Reminder::Off),
                ("宠物标记", Reminder::Badge),
                ("提醒卡片", Reminder::Card),
            ] {
                if widgets::tab(ui, label, config.reminder == reminder, width).clicked() {
                    config.reminder = reminder;
                }
            }
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new("默认只显示标记。提醒卡片不抢键盘焦点，也不播放声音。")
                .size(12.0)
                .color(theme::MUTED),
        );
    });
    ui.label(
        RichText::new("关闭设置继续计时；退出整个应用结束本次计时。时长方案和提醒设置自动保存。")
            .size(12.0)
            .color(theme::MUTED),
    );
}

fn controls(ui: &mut egui::Ui, config: &TimerConfig, timer: &mut WorkTimer) {
    let (icon, label) = match timer.session.state {
        State::Running => (Icon::Pause, "暂停计时"),
        State::Paused => (Icon::Play, "继续计时"),
        State::Complete if timer.session.mode == Some(Mode::Focus) => (Icon::Play, "开始休息"),
        _ if config.stopwatch => (Icon::Play, "开始正计时"),
        _ => (Icon::Play, "开始专注"),
    };
    if actions::primary(ui, icon, label, true).clicked() {
        let now = Instant::now();
        match timer.session.state {
            State::Running => timer.pause(now),
            State::Paused => timer.resume(now),
            State::Complete if timer.session.mode == Some(Mode::Focus) => timer.start_break(now),
            _ => timer.start(
                if config.stopwatch {
                    Mode::Stopwatch
                } else {
                    Mode::Focus
                },
                config,
                now,
            ),
        }
    }
    if timer.session.state != State::Idle {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if timer.session.mode == Some(Mode::Stopwatch)
                && matches!(timer.session.state, State::Running | State::Paused)
                && actions::secondary(ui, Icon::Check, "结束计时").clicked()
            {
                timer.finish_stopwatch(Instant::now());
            }
            if actions::secondary(ui, Icon::Cancel, "重置本次").clicked() {
                timer.reset();
            }
            if timer.notice && actions::secondary(ui, Icon::Check, "知道了").clicked() {
                timer.notice = false;
            }
        });
    }
}

fn durations(ui: &mut egui::Ui, config: &mut TimerConfig) {
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 16.0) / 3.0;
        for (label, preset) in [
            ("25 / 5 分钟", Some((25, 5))),
            ("50 / 10 分钟", Some((50, 10))),
            ("自定义", None),
        ] {
            let selected = preset.map_or(config.custom, |(f, b)| {
                !config.custom && (config.focus_minutes, config.break_minutes) == (f, b)
            });
            if widgets::tab(ui, label, selected, width).clicked() {
                config.custom = preset.is_none();
                if let Some((f, b)) = preset {
                    config.focus_minutes = f;
                    config.break_minutes = b;
                }
            }
        }
    });
    ui.add_space(8.0);
    if config.custom {
        widgets::integer(ui, "专注时长", &mut config.focus_minutes, 1..=1440, " 分钟");
        widgets::integer(ui, "休息时长", &mut config.break_minutes, 1..=1440, " 分钟");
    } else {
        ui.label(
            RichText::new(format!(
                "专注 {} 分钟 · 休息 {} 分钟",
                config.focus_minutes, config.break_minutes
            ))
            .size(12.0)
            .color(theme::MUTED),
        );
    }
}
