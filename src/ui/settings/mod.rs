mod pages;
mod shell;
#[cfg(test)]
mod tests;
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
    reveal_requested: bool,
    editing: Option<Metric>,
    metric_filter: Option<&'static str>,
    confirm_clean: crate::domain::cleanup_profile::Review,
    pets: pet::Sprites,
    report_page: crate::ui::work_tools::diagnostic_report::Page,
    pub screenshot_page: crate::ui::work_tools::screenshot::Page,
    office_page: crate::ui::work_tools::office::Page,
    clipboard_page: crate::ui::work_tools::clipboard::Page,
}
impl Settings {
    pub fn reveal(&mut self) {
        self.open = true;
        self.reveal_requested = true;
    }
    pub fn open_from_pet(&mut self, action: crate::ui::companion::flyout::Action) -> bool {
        use crate::ui::companion::flyout::Action;
        match action {
            Action::Settings => self.reveal(),
            Action::AdvancedClean => self.open_cleanup(),
            Action::Timer => self.open_timer(),
            Action::Meeting => self.open_meeting(),
            Action::Clipboard => self.open_clipboard(),
            Action::Report => self.open_report(),
            Action::Screenshot => self.open_screenshot(),
            Action::Notes => self.open_office(crate::domain::office::Kind::Note),
            Action::Tasks => self.open_office(crate::domain::office::Kind::Task),
            Action::Restore | Action::Compact => return false,
        }
        true
    }
    pub fn open_clipboard(&mut self) {
        self.tab = 11;
        self.reveal();
    }
    pub fn meeting_visible(&self) -> bool {
        self.open && self.tab == 10
    }
    pub fn open_meeting(&mut self) {
        self.tab = 10;
        self.reveal();
    }
    pub fn open_cleanup(&mut self) {
        self.tab = 4;
        self.reveal();
    }
    pub fn open_timer(&mut self) {
        self.tab = 6;
        self.reveal();
    }
    pub fn open_report(&mut self) {
        self.tab = 7;
        self.reveal();
    }
    pub fn open_screenshot(&mut self) {
        self.tab = 8;
        self.reveal();
    }
    pub fn open_office(&mut self, kind: crate::domain::office::Kind) {
        self.tab = 9;
        self.reveal();
        self.office_page.open(kind);
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
            self.clipboard_page.release();
            return false;
        }
        let before = serde_json::to_string(config).unwrap();
        let mut closed = false;
        let reveal = std::mem::take(&mut self.reveal_requested);
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
                if reveal {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                if ctx.input(|i| i.viewport().close_requested()) {
                    closed = true;
                }
                backdrop::show(ctx);
                closed |= window_chrome::show(ctx);
                if let Some(tab) = navigation::navigation(ctx, self.tab, config) {
                    self.confirm_clean.cancel();
                    if self.tab == 11 && tab != 11 {
                        self.clipboard_page.release();
                    }
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
                        } else if self.tab == 7 {
                            self.report_page.show(ui, config, &mut services.report);
                        } else if self.tab == 8 {
                            let previous_folder = services.screenshot.folder.clone();
                            self.screenshot_page.show(ui, &mut services.screenshot);
                            if previous_folder != services.screenshot.folder {
                                config.screenshot_folder = services.screenshot.folder.clone();
                            }
                        } else if self.tab == 9 {
                            self.office_page.show(ui, &mut services.office);
                        } else if self.tab == 11 {
                            self.clipboard_page.show(
                                ui,
                                &mut config.clipboard,
                                &mut services.clipboard,
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
                                    10 => crate::ui::work_tools::meeting::show(
                                        ui,
                                        &mut config.meeting,
                                        &mut services.meeting,
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
            self.screenshot_page.release();
            self.clipboard_page.release();
            services.screenshot.discard();
        }
        before != serde_json::to_string(config).unwrap()
    }
}
pub(crate) fn clipboard_preview(
    ctx: &egui::Context,
    page: &mut crate::ui::work_tools::clipboard::Page,
    service: &mut crate::application::clipboard::ClipboardService,
) {
    let mut config = service.config.clone();
    preview_shell(ctx, 11, |ui| page.show(ui, &mut config, service));
}
pub fn section(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
    theme::glass_panel(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(title).size(14.0).strong().color(theme::TEXT));
        ui.add_space(12.0);
        content(ui);
    });
    ui.add_space(14.0);
}

pub(crate) fn office_preview(
    ctx: &egui::Context,
    page: &mut crate::ui::work_tools::office::Page,
    service: &mut crate::application::office::OfficeService,
) {
    preview_shell(ctx, 9, |ui| page.show(ui, service));
}

pub(crate) fn controls_preview(
    ctx: &egui::Context,
    tab: usize,
    config: &mut Config,
    editing: &mut Option<Metric>,
    filter: &mut Option<&'static str>,
) {
    preview_shell(ctx, tab, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            if tab == 0 {
                pages::metrics_page::show(ui, config, editing, filter);
            } else {
                pages::devices::show(ui, config, &Snapshot::default());
            }
        });
    });
}

pub(crate) fn meeting_preview(
    ctx: &egui::Context,
    config: &mut crate::domain::meeting::MeetingConfig,
    service: &mut crate::application::meeting::MeetingService,
) {
    preview_shell(ctx, 10, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            crate::ui::work_tools::meeting::show(ui, config, service)
        });
    });
}

fn preview_shell(ctx: &egui::Context, tab: usize, contents: impl FnOnce(&mut egui::Ui)) {
    let config = Config::default();
    backdrop::show(ctx);
    window_chrome::show(ctx);
    navigation::navigation(ctx, tab, &config);
    egui::CentralPanel::default()
        .frame(egui::Frame::new().inner_margin(28))
        .show(ctx, |ui| {
            navigation::header(ui, tab, &config, &Snapshot::default());
            contents(ui);
        });
}

pub(crate) fn appearance_components_preview(
    ctx: &egui::Context,
    config: &mut Config,
    pets: &mut pet::Sprites,
) {
    preview_shell(ctx, 1, |ui| {
        pages::appearance::show(ui, config, pets, &Snapshot::default());
    });
}

pub(crate) fn screenshot_preview(
    ctx: &egui::Context,
    page: &mut crate::ui::work_tools::screenshot::Page,
    service: &mut crate::application::screenshot::ScreenshotService,
) {
    preview_shell(ctx, 8, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| page.show(ui, service));
    });
}
