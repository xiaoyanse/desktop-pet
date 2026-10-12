use crate::{
    application::office::OfficeService,
    domain::config::{Config, Metric},
    domain::office::Kind,
    interfaces::FontSource,
    ui::{
        common::{fonts, theme},
        work_tools::office::{Page, notice},
    },
};
use eframe::egui;
pub struct Preview {
    page: Page,
    gallery: super::components::Gallery,
    config: Config,
    editing: Option<Metric>,
    filter: Option<&'static str>,
    stage: usize,
}
impl Preview {
    pub fn new(ctx: &egui::Context, font: FontSource) -> Self {
        theme::setup(ctx);
        fonts::install(ctx, font);
        Self {
            page: Page::default(),
            gallery: super::components::Gallery::default(),
            config: Config::default(),
            editing: None,
            filter: None,
            stage: 0,
        }
    }
    pub fn stage(&mut self, stage: usize, service: &OfficeService) {
        self.stage = stage;
        self.page
            .open(if stage == 0 { Kind::Note } else { Kind::Task });
        if stage == 2 {
            self.page.edit_item(2, service);
        }
    }
    pub fn show(&mut self, ctx: &egui::Context, service: &mut OfficeService) {
        if self.stage >= 5 {
            crate::ui::settings::controls_preview(
                ctx,
                if self.stage == 5 { 0 } else { 2 },
                &mut self.config,
                &mut self.editing,
                &mut self.filter,
            );
        } else if self.stage == 4 {
            self.gallery.show(ctx);
        } else if self.stage == 3 {
            egui::CentralPanel::default()
                .frame(theme::glass_frame().fill(theme::SURFACE))
                .show(ctx, |ui| {
                    if let Some(item) = service.notice() {
                        notice::panel(ui, service, item);
                    }
                });
        } else {
            crate::ui::settings::office_preview(ctx, &mut self.page, service);
        }
    }
}
