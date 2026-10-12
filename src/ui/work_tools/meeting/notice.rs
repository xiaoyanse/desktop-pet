use crate::{
    application::meeting::MeetingService,
    domain::{
        desktop::{DesktopGeometry, flyout_position},
        meeting::Notice,
        work_timer::Reminder,
    },
    ui::common::{
        actions,
        components::layout,
        icons::{self, Icon},
        theme,
    },
};
use eframe::egui::{self, RichText};
pub fn feedback(
    ctx: &egui::Context,
    service: &mut MeetingService,
    reminder: Reminder,
    geometry: Option<DesktopGeometry>,
) -> bool {
    let Some(notice) = service.notices.front().copied() else {
        return false;
    };
    if reminder == Reminder::Off {
        return false;
    }
    if reminder == Reminder::Badge {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("meeting-badge"),
        ));
        let rect = egui::Rect::from_center_size(
            egui::pos2(ctx.screen_rect().right() - 12.0, 37.0),
            egui::Vec2::splat(22.0),
        );
        painter.rect_filled(rect, 7, theme::SURFACE);
        icons::draw(
            &painter,
            rect.shrink(2.0),
            Icon::Meeting,
            egui::Color32::WHITE,
        );
        return false;
    }
    let Some(geometry) = geometry else {
        return false;
    };
    let size = [360.0, 250.0];
    let mut open = false;
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("meeting-reminder"),
        egui::ViewportBuilder::default()
            .with_title("资源助手 · 会议提醒")
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
                service.dismiss();
                return;
            }
            egui::CentralPanel::default()
                .frame(layout::dialog_frame())
                .show(ctx, |ui| {
                    open = panel(ui, service, notice);
                });
        },
    );
    open
}

pub(crate) fn panel(ui: &mut egui::Ui, service: &mut MeetingService, notice: Notice) -> bool {
    ui.label(
        RichText::new(notice.label())
            .size(21.0)
            .color(theme::WARNING),
    );
    ui.label(
        RichText::new(notice.detail())
            .size(13.0)
            .color(theme::MUTED),
    );
    ui.add_space(12.0);
    let open = actions::primary(ui, Icon::Meeting, "查看会议", true).clicked();
    let dismiss = actions::secondary(ui, Icon::Check, "知道了").clicked();
    if open || dismiss {
        service.dismiss();
    }
    open
}
