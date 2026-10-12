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
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
struct SyntheticMicrophone;
impl crate::interfaces::MicrophoneSource for SyntheticMicrophone {
    fn sample(&self) -> Result<crate::domain::meeting::MicrophoneStatus, String> {
        Err("合成预览禁止查询真实设备".into())
    }
}
pub struct Preview {
    page: Page,
    gallery: super::components::Gallery,
    states: super::component_states::States,
    screenshot: super::screenshot_preview::Preview,
    clipboard: super::clipboard_preview::Preview,
    pets: crate::ui::companion::pet::Sprites,
    config: Config,
    editing: Option<Metric>,
    filter: Option<&'static str>,
    stage: usize,
    meeting: crate::application::meeting::MeetingService,
}
impl Preview {
    pub fn new(ctx: &egui::Context, font: FontSource) -> Self {
        theme::setup(ctx);
        fonts::install(ctx, font);
        Self {
            page: Page::default(),
            gallery: super::components::Gallery::default(),
            states: super::component_states::States::default(),
            screenshot: super::screenshot_preview::Preview::default(),
            clipboard: Default::default(),
            pets: Default::default(),
            config: Config::default(),
            editing: None,
            filter: None,
            stage: 0,
            meeting: crate::application::meeting::MeetingService::new(
                Arc::new(SyntheticMicrophone),
                Arc::new(|| {}),
            ),
        }
    }
    pub fn stage(&mut self, stage: usize, service: &OfficeService) {
        self.stage = stage;
        if stage >= 16 {
            self.clipboard.stage(stage);
        }
        self.page
            .open(if stage == 0 { Kind::Note } else { Kind::Task });
        if stage == 2 {
            self.page.edit_item(2, service);
        }
        if stage >= 7 {
            self.meeting.microphone_at = Some(Instant::now());
            self.meeting.microphone = Some(Ok(crate::domain::meeting::MicrophoneStatus {
                name: "合成数据 · 会议麦克风".into(),
                muted: stage == 7,
                volume: Some(75),
            }));
            if stage >= 8 {
                self.meeting.session.start(&self.config.meeting);
                self.meeting.session.advance(Duration::from_secs(1860));
            }
            if stage == 9 {
                self.meeting.microphone = Some(Err(
                    "合成数据 · 没有可用的通信麦克风，请连接设备后刷新。".into(),
                ));
            }
            if stage == 10 {
                self.meeting
                    .notices
                    .push_back(crate::domain::meeting::Notice::Time);
            }
        }
    }
    pub fn show(&mut self, ctx: &egui::Context, service: &mut OfficeService) {
        if self.stage >= 16 {
            self.clipboard.show(ctx);
        } else if self.stage == 15 {
            self.screenshot.show(ctx);
        } else if self.stage == 14 {
            crate::ui::settings::appearance_components_preview(
                ctx,
                &mut self.config,
                &mut self.pets,
            );
        } else if self.stage >= 12 {
            self.states.show(ctx, self.stage == 13);
        } else if self.stage == 11 {
            super::control_icons::show(ctx);
        } else if self.stage == 10 {
            egui::CentralPanel::default()
                .frame(crate::ui::common::components::layout::dialog_frame())
                .show(ctx, |ui| {
                    crate::ui::work_tools::meeting::notice::panel(
                        ui,
                        &mut self.meeting,
                        crate::domain::meeting::Notice::Time,
                    );
                });
        } else if self.stage >= 7 {
            crate::ui::settings::meeting_preview(ctx, &mut self.config.meeting, &mut self.meeting);
        } else if self.stage >= 5 {
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
