mod pages;
mod shell;
use crate::ui::{
    common::{artwork, theme},
    companion::pet,
};
use crate::{
    application::Services,
    domain::{
        config::{Config, Metric},
        resources::Snapshot,
    },
};
use eframe::egui::{self, Color32, RichText};
use pages::cleanup_page;
use shell::{backdrop, navigation, window_chrome};

#[derive(Default)]
pub struct Settings {
    tab: usize,
    pub open: bool,
    pub preview_quick: bool,
    editing: Option<Metric>,
    metric_filter: Option<&'static str>,
    confirm_clean: crate::domain::cleanup_profile::Review,
    pets: pet::Sprites,
}
impl Settings {
    pub fn open_cleanup(&mut self) {
        self.tab = 4;
        self.open = true;
    }
    pub fn open_timer(&mut self) {
        self.tab = 6;
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
        let before = serde_json::to_string(config).unwrap();
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
                if ctx.input(|i| i.viewport().close_requested()) {
                    closed = true;
                }
                backdrop::show(ctx);
                closed |= window_chrome::show(ctx);
                if let Some(tab) = navigation::navigation(ctx, self.tab, config) {
                    self.confirm_clean.cancel();
                    self.tab = tab;
                }
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().inner_margin(28))
                    .show(ctx, |ui| {
                        navigation::header(ui, self.tab, config, snapshot);
                        if !message.is_empty() {
                            ui.label(
                                RichText::new(message.as_str())
                                    .size(12.0)
                                    .color(Color32::from_rgb(248, 200, 82)),
                            );
                            ui.add_space(12.0);
                        }
                        if self.tab == 1 {
                            self.preview_quick |=
                                pages::appearance::show(ui, config, &mut self.pets, snapshot);
                        } else if self.tab == 4 {
                            cleanup_page::show(
                                ui,
                                config,
                                &mut services.cleaner,
                                &mut self.confirm_clean,
                            );
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
                                    6 => crate::ui::work_tools::work_timer::show(
                                        ui,
                                        &mut config.timer,
                                        &mut services.timer,
                                    ),
                                    _ => cleanup_page::history(ui, &services.cleaner),
                                });
                        }
                    });
                window_chrome::resize(ctx);
            },
        );
        if closed {
            self.open = false;
            self.confirm_clean.cancel();
            artwork::release(ctx, "workspace");
        }
        before != serde_json::to_string(config).unwrap()
    }
}
pub fn section(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
    theme::glass_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(title).size(14.0).strong().color(theme::TEXT));
        ui.add_space(12.0);
        content(ui);
    });
    ui.add_space(14.0);
}
