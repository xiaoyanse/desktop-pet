use super::MonitorApp;
use crate::{
    application::screenshot::ScreenshotService, domain::screenshot::Mode, ui::settings::Settings,
};
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

#[derive(Default)]
pub(super) struct CaptureFlow {
    pub hidden: bool,
    pending: Option<(Mode, Instant)>,
    restore_workbench: bool,
    generation: u64,
}
impl CaptureFlow {
    pub fn begin(
        &mut self,
        ctx: &egui::Context,
        settings: &mut Settings,
        service: &ScreenshotService,
        mode: Mode,
        now: Instant,
    ) -> bool {
        if self.hidden || service.busy() {
            return false;
        }
        self.restore_workbench = settings.open;
        self.generation = service.generation;
        if service.pinned.is_some() {
            settings.screenshot_page.hide(ctx);
        }
        if settings.open {
            ctx.send_viewport_cmd_to(
                egui::ViewportId::from_hash_of("settings"),
                egui::ViewportCommand::Visible(false),
            );
        }
        settings.open = false;
        self.hidden = true;
        self.pending = Some((mode, now + Duration::from_millis(200)));
        suspend_root(ctx);
        ctx.request_repaint_after(Duration::from_millis(40));
        true
    }
    pub fn interrupt(&mut self) {
        self.pending = None;
        self.restore_workbench = false;
    }
    pub fn tick(
        &mut self,
        ctx: &egui::Context,
        settings: &mut Settings,
        service: &mut ScreenshotService,
        now: Instant,
        passthrough: bool,
    ) -> bool {
        service.tick();
        if let Some((mode, ready)) = self.pending
            && now >= ready
        {
            self.pending = None;
            service.capture(mode);
        }
        let busy = self.pending.is_some() || service.capturing();
        if self.hidden && !busy {
            self.hidden = false;
            restore_root(ctx, passthrough);
            if service.generation != self.generation {
                settings.open_screenshot();
            } else if self.restore_workbench {
                settings.open = true;
                ctx.send_viewport_cmd_to(
                    egui::ViewportId::from_hash_of("settings"),
                    egui::ViewportCommand::Visible(true),
                );
            }
            if service.pinned.is_some() {
                ctx.send_viewport_cmd_to(
                    egui::ViewportId::from_hash_of("pinned-screenshot"),
                    egui::ViewportCommand::Visible(true),
                );
            }
        }
        if busy {
            suspend_root(ctx);
            ctx.request_repaint_after(Duration::from_millis(40));
        }
        busy
    }
}

impl MonitorApp {
    pub(super) fn screenshot_tick(&mut self, ctx: &egui::Context) -> bool {
        let busy = self.capture_flow.tick(
            ctx,
            &mut self.settings,
            &mut self.services.screenshot,
            Instant::now(),
            self.passthrough,
        );
        if busy {
            self.flyout.dismiss();
            self.services.desktop.tooltip("", false);
        }
        busy
    }
    pub(super) fn begin_capture(&mut self, ctx: &egui::Context, mode: Mode) {
        if self.capture_flow.begin(
            ctx,
            &mut self.settings,
            &self.services.screenshot,
            mode,
            Instant::now(),
        ) {
            self.flyout.dismiss();
            self.pet_hover = false;
            self.hover_quick = false;
            self.services.desktop.tooltip("", false);
        }
    }
    pub(super) fn screenshot_ui(&mut self, ctx: &egui::Context) {
        if let Some(mode) = self.settings.screenshot_page.request.take() {
            self.begin_capture(ctx, mode);
        }
        self.settings.screenshot_page.pinned(
            ctx,
            &mut self.services.screenshot,
            self.capture_flow.hidden,
        );
        if self.services.screenshot.busy() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }
}
