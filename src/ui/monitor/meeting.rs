use super::MonitorApp;
use crate::domain::work_timer::Reminder;
use eframe::egui;
use std::time::Instant;
impl MonitorApp {
    pub(super) fn meeting_tick(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        let open = self.settings.meeting_visible();
        self.services.meeting.tick(now, open, &self.config.meeting);
        if let Some(delay) = self.services.meeting.next_wakeup(now, open) {
            ctx.request_repaint_after(delay);
        }
    }
    pub(super) fn meeting_feedback(&mut self, ctx: &egui::Context) -> bool {
        let card = self.config.meeting.reminder == Reminder::Card
            && !self.services.meeting.notices.is_empty();
        let geometry = if card {
            self.services.desktop.geometry()
        } else {
            None
        };
        if crate::ui::work_tools::meeting::notice::feedback(
            ctx,
            &mut self.services.meeting,
            self.config.meeting.reminder,
            geometry,
        ) {
            self.settings.open_meeting();
        }
        card
    }
}
