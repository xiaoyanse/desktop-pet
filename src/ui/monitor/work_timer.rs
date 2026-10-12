use super::MonitorApp;
use crate::domain::work_timer::Reminder;
use eframe::egui;

impl MonitorApp {
    pub(super) fn timer_feedback(&mut self, ctx: &egui::Context) {
        if let Some(delay) = self.services.timer.next_wakeup() {
            ctx.request_repaint_after(delay);
        }
        let card = self.services.timer.notice && self.config.timer.reminder == Reminder::Card;
        let geometry = if card {
            self.services.desktop.geometry()
        } else {
            None
        };
        if crate::ui::work_tools::work_timer::feedback(
            ctx,
            &mut self.services.timer,
            &self.config.timer,
            geometry,
        ) {
            self.settings.open_timer();
        }
        if card {
            self.services.desktop.configure_auxiliary();
        }
    }
}
