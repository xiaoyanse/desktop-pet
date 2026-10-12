use super::MonitorApp;
use crate::interfaces::DesktopAction as Action;
use eframe::egui;

impl MonitorApp {
    pub(super) fn unlock(&mut self, ctx: &egui::Context) {
        self.locked = false;
        self.passthrough = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }
    pub(super) fn lock(&mut self, ctx: &egui::Context, passthrough: bool) {
        self.locked = true;
        self.passthrough = passthrough;
        self.pet_hover = false;
        self.hover_quick = false;
        self.flyout.dismiss();
        self.services.desktop.tooltip("", false);
        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(passthrough));
        ctx.request_repaint();
    }
    pub(super) fn receive_actions(&mut self, ctx: &egui::Context) {
        for action in self.services.desktop.actions() {
            if self.screenshot_hidden && interrupts_capture(action) {
                self.screenshot_pending = None;
                self.services.screenshot.cancel();
            }
            match action {
                Action::Unlock => {
                    self.unlock(ctx);
                    if self.config.compact || self.config.pet_mode {
                        self.toggle_mode(ctx);
                    }
                }
                Action::Passthrough => self.lock(ctx, true),
                Action::Settings => {
                    self.unlock(ctx);
                    self.settings.open = true;
                }
                Action::ToggleMode => self.toggle_mode(ctx),
                Action::Pet => self.toggle_pet(ctx),
                Action::PetReact => self.pets.react(false),
                Action::PetDrop => self.pets.react(true),
                Action::Cleanup => self.flyout.open(),
                Action::Exit => self.request_exit(ctx),
            }
        }
    }
}

fn interrupts_capture(action: Action) -> bool {
    matches!(
        action,
        Action::Unlock | Action::Settings | Action::ToggleMode | Action::Pet | Action::Exit
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restoring_or_exiting_interrupts_capture_but_pass_through_keeps_it_running() {
        for action in [
            Action::Unlock,
            Action::Settings,
            Action::ToggleMode,
            Action::Pet,
            Action::Exit,
        ] {
            assert!(interrupts_capture(action));
        }
        for action in [
            Action::Passthrough,
            Action::PetReact,
            Action::PetDrop,
            Action::Cleanup,
        ] {
            assert!(!interrupts_capture(action));
        }
    }
}
