use crate::ui::{
    common::{
        actions, cleanup_choice,
        icons::{self, Icon},
        theme,
    },
    monitoring::panel,
};
use crate::{
    application::Services,
    domain::{
        cleanup::CleanupConfig,
        cleanup_profile::{Profile, Review},
        config::Config,
        desktop::{DesktopGeometry, flyout_position},
        resources::Snapshot,
    },
};
use eframe::egui::{self, Color32, RichText};
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub enum Action {
    Restore,
    Compact,
    Settings,
    AdvancedClean,
    Timer,
}
pub struct Flyout {
    pub pinned: bool,
    until: Instant,
    active: bool,
    review: Review,
}
impl Default for Flyout {
    fn default() -> Self {
        Self {
            pinned: false,
            until: Instant::now(),
            active: false,
            review: Review::default(),
        }
    }
}
impl Flyout {
    pub fn open(&mut self) {
        self.pinned = true;
    }
    pub fn active(&self) -> bool {
        self.active
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
        let (service, timer) = (&mut services.cleaner, &services.timer);
        if trigger {
            self.until = Instant::now() + Duration::from_millis(500);
        }
        self.active = self.pinned || Instant::now() < self.until;
        if !self.active {
            return None;
        }
        let geometry = geometry?;
        let size = [
            300.0_f32.min(geometry.work_area[2] - geometry.work_area[0]),
            360.0_f32.min(geometry.work_area[3] - geometry.work_area[1]),
        ];
        let position = flyout_position(geometry, size);
        let mut action = None;
        let mut close = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("resource-flyout"),
            egui::ViewportBuilder::default()
                .with_title("资源助手 · 快捷操作")
                .with_decorations(false)
                .with_taskbar(false)
                .with_active(false)
                .with_transparent(true)
                .with_always_on_top()
                .with_resizable(false)
                .with_inner_size(size)
                .with_position(position),
            |ctx, _| {
                if ctx.input(|i| i.pointer.hover_pos().is_some()) {
                    self.until = Instant::now() + Duration::from_millis(500);
                }
                if ctx.input(|i| i.viewport().close_requested()) {
                    close = true;
                }
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::new()
                            .fill(theme::BACKGROUND)
                            .corner_radius(12)
                            .inner_margin(16)
                            .stroke(egui::Stroke::new(1.0_f32, theme::LINE)),
                    )
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("资源助手").size(18.0).strong());
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if icons::bare_button(ui, Icon::Cancel, "收起快捷面板")
                                        .clicked()
                                    {
                                        close = true;
                                    }
                                },
                            );
                        });
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                let rows = panel::rows(config, snapshot);
                                for row in rows.iter().take(5) {
                                    ui.horizontal(|ui| {
                                        ui.weak(&row.label);
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.colored_label(
                                                    panel::tint(
                                                        panel::level(row),
                                                        config.status_colors,
                                                        Color32::from_rgb(100, 210, 195),
                                                    ),
                                                    &row.reading.text,
                                                );
                                            },
                                        );
                                    });
                                }
                                if rows.is_empty() {
                                    ui.weak("在设置中选择需要显示的监测项。");
                                }
                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    let state = &timer.session;
                                    let label =
                                        if state.state == crate::domain::work_timer::State::Idle {
                                            "工作计时 · 开始你的节奏".to_owned()
                                        } else {
                                            format!(
                                                "{} · {}",
                                                state.mode.map_or("计时", |m| m.label()),
                                                crate::domain::work_timer::clock(
                                                    state.displayed_seconds()
                                                )
                                            )
                                        };
                                    if actions::secondary(ui, Icon::Timer, &label).clicked() {
                                        action = Some(Action::Timer);
                                    }
                                });
                                if timer.notice {
                                    ui.colored_label(theme::ACCENT, "计时已到，请休息或查看计时");
                                }
                                ui.add_space(10.0);
                                ui.separator();
                                ui.add_space(6.0);
                                let mut help = "完整视图 · 精简视图 · 设置 · 清理中心";
                                ui.horizontal(|ui| {
                                    for (icon, label, target) in [
                                        (Icon::Restore, "恢复完整视图", Action::Restore),
                                        (Icon::Compact, "切换精简视图", Action::Compact),
                                        (Icon::Settings, "打开设置", Action::Settings),
                                        (Icon::General, "打开完整清理中心", Action::AdvancedClean),
                                    ] {
                                        let response = icons::bare_button(ui, icon, label);
                                        if response.hovered() {
                                            help = label;
                                        }
                                        if response.clicked() {
                                            action = Some(target);
                                        }
                                    }
                                });
                                ui.label(RichText::new(help).size(11.0));
                                ui.add_space(10.0);
                                egui::Frame::new()
                                    .fill(theme::SURFACE)
                                    .corner_radius(9)
                                    .inner_margin(10)
                                    .show(ui, |ui| {
                                        ui.label(RichText::new("轻度清理 · 日常推荐").strong());
                                        ui.weak("只清低优先级待机缓存，不收缩应用工作集。");
                                        ui.add_space(8.0);
                                        if self.review.operations().is_none()
                                            && actions::primary(
                                                ui,
                                                Icon::Clean,
                                                if service.busy() {
                                                    "正在处理…"
                                                } else {
                                                    "轻度清理一次"
                                                },
                                                !service.busy(),
                                            )
                                            .clicked()
                                        {
                                            self.review.begin(&CleanupConfig {
                                                operations: Profile::Light.operations().to_vec(),
                                                ..Default::default()
                                            });
                                            self.pinned = true;
                                        }
                                        if let Some(request) = cleanup_choice::review(
                                            ui,
                                            &mut self.review,
                                            !service.busy(),
                                        ) {
                                            service.start(&request, false);
                                        }
                                        ui.add_space(6.0);
                                        if actions::secondary(ui, Icon::General, "选择中度 / 重度")
                                            .clicked()
                                        {
                                            action = Some(Action::AdvancedClean);
                                        }
                                        if !service.message.is_empty() {
                                            ui.label(RichText::new(&service.message).size(11.0));
                                        }
                                    });
                                ui.add_space(6.0);
                                ui.weak("待机缓存已属于可用内存；清理无法修复内核池泄漏。");
                            });
                    });
            },
        );
        if close || action.is_some() {
            self.review.cancel();
            self.pinned = false;
            self.until = Instant::now();
            self.active = false;
        } else {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
        action
    }
}
