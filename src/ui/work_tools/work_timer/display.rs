use crate::{
    application::work_timer::WorkTimer,
    domain::work_timer::{Mode, State, TimerConfig, clock},
    ui::common::theme,
};
use eframe::egui::{self, Color32};

pub fn status(timer: &WorkTimer) -> &'static str {
    match timer.session.state {
        State::Idle => "准备开始",
        State::Running => "进行中",
        State::Paused => "已暂停",
        State::Complete if timer.session.mode == Some(Mode::Focus) => "专注结束 · 休息一下",
        State::Complete if timer.session.mode == Some(Mode::Break) => "休息结束 · 按自己的节奏继续",
        State::Complete => "本次计时已结束",
    }
}

pub fn ring(ui: &mut egui::Ui, timer: &WorkTimer, config: &TimerConfig) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 138.0),
        egui::Sense::hover(),
    );
    let center = rect.center();
    let radius = 56.0;
    ui.painter().circle_stroke(
        center,
        radius,
        egui::Stroke::new(4.0_f32, theme::PROGRESS_TRACK),
    );
    let fraction = if timer.session.state == State::Idle {
        0.0
    } else {
        timer.session.target.map_or(0.0, |target| {
            (timer.session.elapsed.as_secs_f64() / target.as_secs_f64()).clamp(0.0, 1.0)
        })
    };
    if fraction > 0.0 {
        let points = (0..=96)
            .map(|i| {
                let angle = -std::f32::consts::FRAC_PI_2
                    + std::f32::consts::TAU * fraction as f32 * i as f32 / 96.0;
                center + egui::vec2(angle.cos(), angle.sin()) * radius
            })
            .collect();
        ui.painter().add(egui::Shape::line(
            points,
            egui::Stroke::new(4.0_f32, theme::ACCENT),
        ));
    }
    let mode = timer.session.mode.unwrap_or(if config.stopwatch {
        Mode::Stopwatch
    } else {
        Mode::Focus
    });
    let seconds = if timer.session.state == State::Idle {
        if config.stopwatch {
            0
        } else {
            config.focus_minutes * 60
        }
    } else {
        timer.session.displayed_seconds()
    };
    ui.painter().text(
        center + egui::vec2(0.0, -27.0),
        egui::Align2::CENTER_CENTER,
        mode.label(),
        egui::FontId::proportional(12.0),
        theme::MUTED,
    );
    let text = clock(seconds);
    ui.painter().text(
        center,
        egui::Align2::CENTER_CENTER,
        &text,
        egui::FontId::proportional(if text.len() > 5 { 27.0 } else { 38.0 }),
        theme::TEXT,
    );
    ui.painter().text(
        center + egui::vec2(0.0, 29.0),
        egui::Align2::CENTER_CENTER,
        if timer.session.state == State::Paused {
            "暂停不计时"
        } else if mode == Mode::Stopwatch {
            "已用时间"
        } else {
            "剩余时间"
        },
        egui::FontId::proportional(11.0),
        Color32::from_rgb(181, 194, 204),
    );
}
