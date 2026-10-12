use super::MonitorApp;
use crate::{application::office, domain::office::Kind};
use eframe::egui;
impl MonitorApp {
    pub(super) fn request_exit(&mut self, ctx: &egui::Context) {
        if self.services.office.busy() {
            self.exit_after_office = true;
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
    pub(super) fn office_tick(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && self.services.office.busy() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.request_exit(ctx);
        }
        let now = office::now();
        self.services.office.tick(now);
        if self.exit_after_office && !self.services.office.busy() {
            self.exit_after_office = false;
            if self.services.office.failed {
                self.message = "本地保存或读取失败，已取消退出，请核对便签提示".into();
                self.settings.open_office(Kind::Note);
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if let Some(delay) = self.services.office.next_wakeup(now) {
            ctx.request_repaint_after(delay);
        }
    }
    pub(super) fn office_feedback(&mut self, ctx: &egui::Context) -> bool {
        let visible = self.services.office.notice().is_some();
        let geometry = if visible {
            self.services.desktop.geometry()
        } else {
            None
        };
        if crate::ui::work_tools::office::notice::show(ctx, &mut self.services.office, geometry) {
            self.settings.open_office(Kind::Task);
        }
        visible
    }
}
