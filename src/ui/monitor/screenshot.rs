use super::MonitorApp;
use eframe::egui;
use std::time::{Duration, Instant};

pub(crate) fn suspend_root(ctx: &egui::Context) {
    // A hidden Win32 root stops WM_PAINT; the same update loop must advance capture.
    ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(true));
}
pub(crate) fn restore_root(ctx: &egui::Context, passthrough: bool) {
    ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(passthrough));
}

#[cfg(test)]
mod tests;

impl MonitorApp {
    pub(super) fn screenshot_tick(&mut self, ctx: &egui::Context) -> bool {
        self.services.screenshot.tick();
        if let Some((mode, ready)) = self.screenshot_pending
            && Instant::now() >= ready
        {
            self.screenshot_pending = None;
            self.services.screenshot.capture(mode);
        }
        let busy = self.screenshot_pending.is_some() || self.services.screenshot.capturing();
        if self.screenshot_hidden && !busy {
            self.screenshot_hidden = false;
            restore_root(ctx, self.passthrough);
            ctx.send_viewport_cmd_to(
                egui::ViewportId::from_hash_of("settings"),
                egui::ViewportCommand::Visible(true),
            );
            self.settings.open_screenshot();
            if self.services.screenshot.pinned.is_some() {
                ctx.send_viewport_cmd_to(
                    egui::ViewportId::from_hash_of("pinned-screenshot"),
                    egui::ViewportCommand::Visible(true),
                );
            }
        }
        if busy {
            suspend_root(ctx);
            self.flyout.dismiss();
            self.services.desktop.tooltip("", false);
            ctx.request_repaint_after(Duration::from_millis(40));
            return true;
        }
        false
    }
    pub(super) fn screenshot_ui(&mut self, ctx: &egui::Context) {
        if let Some(mode) = self.settings.screenshot_page.request.take()
            && !self.services.screenshot.busy()
        {
            self.flyout.dismiss();
            self.pet_hover = false;
            self.hover_quick = false;
            self.services.desktop.tooltip("", false);
            self.settings.screenshot_page.hide(ctx);
            self.settings.open = false;
            self.screenshot_hidden = true;
            suspend_root(ctx);
            ctx.send_viewport_cmd_to(
                egui::ViewportId::from_hash_of("settings"),
                egui::ViewportCommand::Visible(false),
            );
            self.screenshot_pending = Some((mode, Instant::now() + Duration::from_millis(200)));
            ctx.request_repaint_after(Duration::from_millis(40));
        }
        self.settings.screenshot_page.pinned(
            ctx,
            &mut self.services.screenshot,
            self.screenshot_hidden,
        );
        if self.services.screenshot.busy() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }
}
