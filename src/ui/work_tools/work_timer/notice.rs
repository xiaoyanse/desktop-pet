use super::display;
use crate::{
    application::work_timer::WorkTimer,
    domain::{
        desktop::{DesktopGeometry, flyout_position},
        work_timer::{Mode, Reminder, TimerConfig},
    },
    ui::common::{
        actions,
        icons::{self, Icon},
        theme,
    },
};
use eframe::egui::{self, RichText};
use std::time::Instant;

pub fn feedback(
    ctx: &egui::Context,
    timer: &mut WorkTimer,
    config: &TimerConfig,
    geometry: Option<DesktopGeometry>,
) -> bool {
    if !timer.notice || config.reminder == Reminder::Off {
        return false;
    }
    if config.reminder == Reminder::Badge {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("timer-complete-badge"),
        ));
        let center = egui::pos2(ctx.screen_rect().right() - 12.0, 12.0);
        painter.circle_filled(center, 10.0, theme::SURFACE);
        painter.circle_stroke(center, 10.0, egui::Stroke::new(1.0_f32, theme::CHAMPAGNE));
        icons::draw(
            &painter,
            egui::Rect::from_center_size(center, egui::Vec2::splat(17.0)),
            Icon::Timer,
            egui::Color32::WHITE,
        );
        return false;
    }
    let Some(geometry) = geometry else {
        return false;
    };
    let size = [340.0, 210.0];
    let mut open = false;
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("timer-reminder"),
        egui::ViewportBuilder::default()
            .with_title("资源助手 · 计时提醒")
            .with_decorations(false)
            .with_taskbar(false)
            .with_transparent(true)
            .with_active(false)
            .with_always_on_top()
            .with_resizable(false)
            .with_inner_size(size)
            .with_position(flyout_position(geometry, size)),
        |ctx, _| {
            if ctx.input(|i| i.viewport().close_requested()) {
                timer.notice = false;
            }
            egui::CentralPanel::default()
                .frame(theme::glass_frame().fill(theme::SURFACE))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let (_, rect) = ui.allocate_space(egui::Vec2::splat(32.0));
                        icons::draw(ui.painter(), rect, Icon::Timer, egui::Color32::WHITE);
                        ui.label(RichText::new("到时间了").size(22.0));
                    });
                    ui.label(
                        RichText::new(display::status(timer))
                            .size(13.0)
                            .color(theme::ACCENT),
                    );
                    ui.add_space(10.0);
                    if timer.session.mode == Some(Mode::Focus)
                        && actions::primary(ui, Icon::Play, "开始休息", true).clicked()
                    {
                        timer.start_break(Instant::now());
                    }
                    ui.horizontal(|ui| {
                        if actions::secondary(ui, Icon::Timer, "查看计时").clicked() {
                            open = true;
                            timer.notice = false;
                        }
                        if actions::secondary(ui, Icon::Check, "知道了").clicked() {
                            timer.notice = false;
                        }
                    });
                });
        },
    );
    open
}
