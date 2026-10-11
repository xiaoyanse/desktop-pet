mod pages;
mod shell;
use crate::ui::{common::theme, companion::pet};
use crate::{
    application::Services,
    domain::{
        config::{Config, Metric},
        resources::Snapshot,
    },
};
use eframe::egui::{self, Color32, RichText};
use pages::cleanup_page;
use shell::{navigation, window_chrome};

#[derive(Default)]
pub struct Settings {
    tab: usize,
    pub open: bool,
    editing: Option<Metric>,
    metric_filter: Option<&'static str>,
    logo: Option<egui::TextureHandle>,
    confirm_clean: crate::domain::cleanup_profile::Review,
    pets: pet::Sprites,
}
impl Settings {
    pub fn open_cleanup(&mut self) {
        self.tab = 4;
        self.open = true;
    }
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        config: &mut Config,
        snapshot: &Snapshot,
        message: &mut String,
        services: &mut Services,
    ) -> bool {
        if !self.open {
            return false;
        }
        if self.logo.is_none() {
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [64, 64],
                include_bytes!(concat!(env!("OUT_DIR"), "/icon-64.rgba")),
            );
            self.logo = Some(ctx.load_texture("app-logo", image, egui::TextureOptions::LINEAR));
        }
        let before = serde_json::to_string(config).unwrap();
        let mut closed = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("settings"),
            egui::ViewportBuilder::default()
                .with_title("资源监视器 · 设置")
                .with_taskbar(true)
                .with_decorations(false)
                .with_resizable(true)
                .with_inner_size([920.0, 780.0])
                .with_min_inner_size([840.0, 600.0])
                .with_always_on_top(),
            |ctx, _| {
                if ctx.input(|i| i.viewport().close_requested()) {
                    closed = true;
                }
                closed |= window_chrome::show(ctx, self.logo.as_ref());
                if let Some(tab) = navigation::navigation(ctx, self.tab, config) {
                    self.confirm_clean.cancel();
                    self.tab = tab;
                }
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(theme::BACKGROUND).inner_margin(28))
                    .show(ctx, |ui| {
                        navigation::header(ui, self.tab, config, snapshot);
                        if self.tab == 1 {
                            pages::appearance::show(ui, config, &mut self.pets, snapshot);
                        } else {
                            egui::ScrollArea::vertical()
                                .id_salt(("settings-scroll", self.tab))
                                .auto_shrink([false, false])
                                .show(ui, |ui| match self.tab {
                                    0 => pages::metrics_page::show(
                                        ui,
                                        config,
                                        &mut self.editing,
                                        &mut self.metric_filter,
                                    ),
                                    2 => pages::devices::show(ui, config, snapshot),
                                    3 => pages::general::show(ui, config, message, services),
                                    4 => cleanup_page::show(
                                        ui,
                                        config,
                                        &mut services.cleaner,
                                        &mut self.confirm_clean,
                                    ),
                                    _ => cleanup_page::history(ui, &services.cleaner),
                                });
                        }
                        if !message.is_empty() {
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new(message.as_str())
                                    .size(12.0)
                                    .color(Color32::from_rgb(248, 200, 82)),
                            );
                        }
                    });
                window_chrome::resize(ctx);
            },
        );
        if closed {
            self.open = false;
            self.confirm_clean.cancel();
        }
        before != serde_json::to_string(config).unwrap()
    }
}
pub fn section(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(theme::SURFACE)
        .stroke(egui::Stroke::new(1.0_f32, theme::LINE))
        .corner_radius(10)
        .inner_margin(18)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(14.0).strong().color(theme::TEXT));
            ui.add_space(12.0);
            content(ui);
        });
    ui.add_space(14.0);
}
