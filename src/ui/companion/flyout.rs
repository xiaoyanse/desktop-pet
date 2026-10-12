mod components;
#[cfg(test)]
mod lifecycle_tests;
mod render;
#[cfg(test)]
mod tests;
use crate::{
    application::Services,
    domain::{
        cleanup_profile::Review,
        companion::spiral::{Group, Layout, MenuState},
        config::Config,
        desktop::DesktopGeometry,
        resources::Snapshot,
    },
};
use eframe::egui;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Restore,
    Compact,
    Settings,
    AdvancedClean,
    Timer,
    Report,
    Screenshot,
    Notes,
    Tasks,
    Meeting,
    Clipboard,
}
pub struct Flyout {
    pub pinned: bool,
    pub warning: Option<String>,
    state: MenuState,
    review: Review,
    started: Instant,
    preview: bool,
    last_regions: Vec<[f32; 4]>,
}
impl Default for Flyout {
    fn default() -> Self {
        Self {
            pinned: false,
            warning: None,
            state: MenuState::default(),
            review: Review::default(),
            started: Instant::now(),
            preview: false,
            last_regions: Vec::new(),
        }
    }
}
impl Flyout {
    pub fn open(&mut self) {
        self.pinned = true;
        self.state.open(self.started.elapsed());
        self.state
            .hover(self.started.elapsed(), Some(Group::Resources), true);
    }
    pub fn open_preview(&mut self) {
        self.preview = true;
        self.pinned = true;
        self.state.open(self.started.elapsed());
    }
    pub fn active(&self) -> bool {
        self.state.active
    }
    pub fn dismiss(&mut self) {
        self.state.close();
        self.pinned = false;
        self.preview = false;
        self.last_regions.clear();
        self.review.cancel();
    }
    pub fn settings_closed(&mut self, open: bool) {
        if self.preview && !open {
            self.state.close();
            self.pinned = false;
            self.preview = false;
            self.last_regions.clear();
            self.review.cancel();
        }
    }
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        geometry: Option<DesktopGeometry>,
        config: &Config,
        snapshot: &Snapshot,
        services: &mut Services,
        trigger: bool,
    ) -> Option<Action> {
        let now = self.started.elapsed();
        self.state.observe_trigger(trigger && !self.preview);
        let geometry = geometry?;
        let layout = Layout::new(geometry);
        let screen_cursor = services
            .desktop
            .cursor()
            .map(|p| [p[0] + geometry.frame[0], p[1] + geometry.frame[1]]);
        let inside =
            screen_cursor.is_some_and(|p| self.last_regions.iter().any(|r| layout.contains(p, *r)));
        self.state.update(
            now,
            trigger && !self.preview,
            inside,
            self.pinned || self.review.operations().is_some(),
        );
        if !self.state.active {
            self.review.cancel();
            self.last_regions.clear();
            return None;
        }
        if self.review.operations().is_none()
            && (!config.pet_animation
                || now.saturating_sub(self.state.opened) >= Duration::from_millis(250))
        {
            let hovered = screen_cursor.and_then(|p| {
                Group::ALL.into_iter().find(|g| {
                    layout.contains(
                        p,
                        layout.group(
                            *g,
                            now.saturating_sub(self.state.opened),
                            config.pet_animation,
                        ),
                    )
                })
            });
            self.state.hover(now, hovered, false);
        }
        let preview = self.preview;
        let animating = self.state.animating(now, config.pet_animation);
        let animation_region = [[0.0, 0.0, layout.size[0], layout.size[1]]];
        if animating && let Err(error) = services.desktop.quick_regions(&animation_region, preview)
        {
            self.warning = Some(error);
            self.dismiss();
            return None;
        }
        let mut output = render::Output::default();
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of(if preview {
                "spiral-preview"
            } else {
                "resource-flyout"
            }),
            egui::ViewportBuilder::default()
                .with_title(if preview {
                    "资源监视器 · 快捷入口试用"
                } else {
                    "资源助手 · 快捷操作"
                })
                .with_decorations(false)
                .with_taskbar(preview)
                .with_active(false)
                .with_transparent(true)
                .with_always_on_top()
                .with_resizable(false)
                .with_inner_size(layout.size)
                .with_position(layout.position),
            |ctx, _| {
                if ctx.input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape))
                {
                    output.close = true;
                }
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
                        let mut content = render::Content {
                            config,
                            snapshot,
                            services,
                            review: &mut self.review,
                        };
                        let rendered =
                            render::show(ui, &layout, &self.state, now, &mut content, self.pinned);
                        output.close |= rendered.close;
                        output.action = rendered.action;
                        output.toggle_pin = rendered.toggle_pin;
                        output.group = rendered.group;
                        output.regions = rendered.regions;
                    });
            },
        );
        if let Some(group) = output.group {
            self.state.hover(now, Some(group), true);
        }
        if output.toggle_pin {
            self.pinned = !self.pinned;
        }
        let native_regions = if animating {
            &animation_region[..]
        } else {
            &output.regions
        };
        if let Err(error) = services.desktop.quick_regions(native_regions, preview) {
            self.warning = Some(error);
            output.close = true;
        }
        self.last_regions = output.regions;
        if output.close || output.action.is_some() {
            self.dismiss();
            ctx.request_repaint();
        } else if animating {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(
                self.state
                    .next_input_check(now)
                    .unwrap_or(Duration::from_millis(50))
                    .min(Duration::from_millis(50)),
            );
        }
        output.action
    }
}
