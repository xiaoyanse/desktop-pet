use eframe::egui;

/// Explicit developer experiment: keeps the production backend and viewport topology,
/// without starting application services or loading business artwork/fonts.
pub struct RenderBaseline {
    settings_open: bool,
    root_sized: bool,
}

impl RenderBaseline {
    pub fn new(ctx: &egui::Context, settings_open: bool) -> Self {
        crate::ui::configure_framework_atlas(ctx);
        Self {
            settings_open,
            root_sized: false,
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        if !self.root_sized {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(240.0, 142.0)));
            self.root_sized = true;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Developer renderer baseline");
            ui.label("No sampler, reports, CJK or artwork");
            if ui.button("Open test surface").clicked() {
                self.settings_open = true;
            }
        });
        if self.settings_open {
            let mut closed = false;
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("settings"),
                egui::ViewportBuilder::default()
                    .with_title("资源监视器 · 设置")
                    .with_taskbar(true)
                    .with_decorations(false)
                    .with_transparent(true)
                    .with_resizable(true)
                    .with_inner_size([1040.0, 820.0])
                    .with_min_inner_size([840.0, 600.0])
                    .with_always_on_top(),
                |ctx, _| {
                    closed = ctx.input(|i| i.viewport().close_requested());
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.heading("DEVELOPER BASELINE — NOT THE PRODUCT UI");
                        ui.label("Same Glow backend, alpha surface and window sizes.");
                        ui.horizontal(|ui| {
                            if ui.button("Maximize").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                            }
                            if ui.button("Restore").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                            }
                            closed |= ui.button("Close surface").clicked();
                        });
                    });
                },
            );
            self.settings_open &= !closed;
        }
        // This experiment is event-driven, with no sampling/animation repaint loop.
    }
}

impl eframe::App for RenderBaseline {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.show(ctx);
    }

    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        if let Some(maximum) = input.max_texture_side {
            input.max_texture_side = Some(maximum.min(2048));
        }
    }

    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_baseline_frames_do_not_send_resize_commands_that_keep_winit_awake() {
        let ctx = egui::Context::default();
        let mut app = RenderBaseline::new(&ctx, false);
        for frame in 0..5 {
            let output = ctx.run(egui::RawInput::default(), |ctx| app.show(ctx));
            let resizes = output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .iter()
                .filter(|cmd| matches!(cmd, egui::ViewportCommand::InnerSize(_)))
                .count();
            assert_eq!(resizes, usize::from(frame == 0));
        }
    }
}
