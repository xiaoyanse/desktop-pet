pub mod notice;
#[cfg(test)]
mod tests;
use crate::{
    application::meeting::MeetingService,
    domain::{
        meeting::MeetingConfig,
        work_timer::{Reminder, State, clock},
    },
    ui::common::{
        actions,
        components::{
            selection,
            status::{self, Tone},
        },
        icons::Icon,
        theme, widgets,
    },
};
use eframe::egui::{self, RichText};
use std::time::Instant;

pub fn show(ui: &mut egui::Ui, config: &mut MeetingConfig, service: &mut MeetingService) {
    theme::glass_panel(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            status::badge(
                ui,
                match service.session.state {
                    State::Idle => "尚未开始",
                    State::Running => "会议进行中",
                    State::Paused => "已暂停",
                    State::Complete => "会议已结束",
                },
                if service.session.state == State::Running {
                    Tone::Success
                } else {
                    Tone::Neutral
                },
            );
        });
        ui.label(
            RichText::new(clock(service.session.elapsed.as_secs()))
                .size(46.0)
                .strong(),
        );
        if service.session.state != State::Idle {
            let overtime = service.session.overtime().as_secs();
            ui.label(
                RichText::new(if overtime > 0 {
                    format!(
                        "预设 {} 分钟 · 已超时 {}",
                        service.session.target.as_secs() / 60,
                        clock(overtime)
                    )
                } else {
                    format!(
                        "预设 {} 分钟 · 到时后继续计时",
                        service.session.target.as_secs() / 60
                    )
                })
                .size(13.0)
                .color(if overtime > 0 {
                    theme::WARNING
                } else {
                    theme::MUTED
                }),
            );
        } else {
            ui.weak("手动开始计时，暂停期间不计入会议时长。");
        }
        ui.add_space(12.0);
        let (icon, label) = match service.session.state {
            State::Running => (Icon::Pause, "暂停会议计时"),
            State::Paused => (Icon::Play, "继续会议计时"),
            _ => (Icon::Meeting, "开始会议计时"),
        };
        if actions::primary(ui, icon, label, true).clicked() {
            match service.session.state {
                State::Running => service.pause(Instant::now(), config),
                State::Paused => service.resume(Instant::now()),
                _ => service.start(config, Instant::now()),
            }
        }
        if service.session.active() && actions::secondary(ui, Icon::Check, "结束会议").clicked()
        {
            service.finish(Instant::now(), config);
        }
        if service.session.state == State::Complete
            && actions::secondary(ui, Icon::Restore, "清除本次计时").clicked()
        {
            service.reset();
        }
        if let Some(notice) = service.notices.front().copied() {
            ui.add_space(8.0);
            status::message(ui, notice.label(), Tone::Warning);
            if actions::secondary(ui, Icon::Check, "已查看提醒").clicked() {
                service.dismiss();
            }
        }
    });
    ui.add_space(14.0);
    microphone(ui, service);
    ui.add_space(14.0);
    theme::glass_panel(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("会议安排").size(16.0).strong());
        ui.add_enabled_ui(!service.session.active(), |ui| {
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 16.0) / 3.0;
                for minutes in [15, 30, 60] {
                    if selection::choice(
                        ui,
                        &format!("{minutes} 分钟"),
                        config.minutes == minutes,
                        width,
                    )
                    .clicked()
                    {
                        config.minutes = minutes;
                    }
                }
            });
            widgets::integer(ui, "预设时长", &mut config.minutes, 1..=1440, " min");
            widgets::switch(ui, &mut config.remind_mute, "定时提醒检查静音");
            if config.remind_mute {
                widgets::integer(ui, "检查间隔", &mut config.mute_minutes, 1..=120, " min");
            }
        });
        if service.session.active() {
            ui.weak("本次会议使用开始时的时长与检查间隔，结束后可修改。");
        }
        ui.add_space(8.0);
        ui.label("提醒方式");
        ui.horizontal(|ui| {
            let width = (ui.available_width() - 16.0) / 3.0;
            for (label, mode) in [
                ("静默", Reminder::Off),
                ("入口标记", Reminder::Badge),
                ("提醒卡片", Reminder::Card),
            ] {
                if selection::choice(ui, label, config.reminder == mode, width).clicked() {
                    config.reminder = mode;
                }
            }
        });
        ui.weak("提醒不播放声音、不抢焦点，也不会替你操作会议软件。关闭设置后会议计时继续；退出整个应用结束本次计时。");
    });
}
fn microphone(ui: &mut egui::Ui, service: &mut MeetingService) {
    theme::glass_panel(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("默认通信麦克风").size(16.0).strong());
        match &service.microphone {
            Some(Ok(mic)) => {
                ui.add(egui::Label::new(RichText::new(&mic.name).size(14.0)).truncate())
                    .on_hover_text(&mic.name);
                ui.horizontal_wrapped(|ui| {
                    status::badge(
                        ui,
                        if mic.muted {
                            "系统已静音"
                        } else {
                            "系统未静音"
                        },
                        if mic.muted {
                            Tone::Success
                        } else {
                            Tone::Warning
                        },
                    );
                    ui.label(
                        RichText::new(
                            mic.volume
                                .map_or("输入音量未知".into(), |v| format!("输入音量 {v}%")),
                        )
                        .size(13.0)
                        .color(theme::MUTED),
                    );
                    if let Some(at) = service.microphone_at {
                        ui.weak(format!(
                            "{} 秒前更新",
                            Instant::now().saturating_duration_since(at).as_secs()
                        ));
                    }
                });
            }
            Some(Err(error)) => status::message(ui, error, Tone::Warning),
            None => {
                status::badge(
                    ui,
                    if service.busy() {
                        "正在读取状态"
                    } else {
                        "尚未读取状态"
                    },
                    Tone::Neutral,
                );
            }
        }
        ui.add_space(8.0);
        if ui
            .add_enabled_ui(!service.busy(), |ui| {
                actions::secondary(
                    ui,
                    Icon::Restore,
                    if service.busy() {
                        "正在读取状态"
                    } else {
                        "刷新麦克风状态"
                    },
                )
            })
            .inner
            .clicked()
        {
            service.refresh();
        }
        ui.weak("这是系统端点的静音与输入音量，不代表会议软件内部静音或正在录音。会议软件选择其他设备时，请在软件中核对。");
        ui.weak("只查询状态，不读取音频内容；页面关闭且会议未运行时停止查询。");
    });
}
