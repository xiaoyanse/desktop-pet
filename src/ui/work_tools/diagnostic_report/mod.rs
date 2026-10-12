mod preview;
mod toolbar;
use crate::{application::report::ReportService, domain::config::Config};
use eframe::egui;

#[derive(Default)]
pub struct Page {
    text_view: bool,
    runtime_page: usize,
    captured: String,
}
impl Page {
    pub fn show(&mut self, ui: &mut egui::Ui, config: &Config, service: &mut ReportService) {
        toolbar::show(ui, config, service);
        ui.add_space(12.0);
        ui.spacing_mut().scroll = egui::style::ScrollStyle {
            foreground_color: true,
            ..egui::style::ScrollStyle::solid()
        };
        let refreshed = service
            .document
            .as_ref()
            .is_some_and(|d| self.captured != d.report.captured_at);
        if let Some(document) = &service.document
            && refreshed
        {
            self.captured = document.report.captured_at.clone();
            self.runtime_page = 0;
        }
        let scroll = egui::ScrollArea::vertical()
            .id_salt("diagnostic-report-content")
            .max_height(ui.available_height().max(80.0))
            .auto_shrink([false, false]);
        let scroll = if refreshed {
            scroll.vertical_scroll_offset(0.0)
        } else {
            scroll
        };
        scroll.show(ui, |ui| {
            if let Some(document) = &service.document {
                preview::show(ui, self, document);
            } else {
                preview::empty(ui);
            }
        });
    }
}
