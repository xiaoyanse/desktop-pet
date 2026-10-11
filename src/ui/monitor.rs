use super::{
    common::theme,
    companion::{flyout, pet},
    monitoring::{controls, details, surface},
};
use crate::{
    application::{Dependencies, Services, monitoring::MonitorWorker},
    domain::{config::Config, resources::Snapshot},
    interfaces::DesktopAction as Action,
    ui::{
        monitoring::panel::{self, Alerts, History},
        settings::Settings,
    },
};
use eframe::egui::{self, Color32};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

pub struct MonitorApp {
    config: Config,
    snapshot: Snapshot,
    worker: MonitorWorker,
    services: Services,
    settings: Settings,
    expanded: bool,
    locked: bool,
    history: History,
    alerts: Alerts,
    message: String,
    tooltip_text: String,
    saved_position: Option<[f32; 2]>,
    dirty_at: Option<Instant>,
    size: egui::Vec2,
    pets: pet::Sprites,
    flyout: flyout::Flyout,
    hover_quick: bool,
    pet_hover: bool,
}
impl MonitorApp {
    pub fn open_settings(&mut self) {
        self.settings.open = true;
    }
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        config: Config,
        warning: Option<String>,
        dependencies: Dependencies,
    ) -> Self {
        let ctx = cc.egui_ctx.clone();
        theme::setup(&ctx);
        let mut message = warning.unwrap_or_default();
        if let Some((name, data)) = dependencies.font {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert(name.to_string(), Arc::new(egui::FontData::from_owned(data)));
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                if family == egui::FontFamily::Monospace {
                    fonts
                        .families
                        .entry(family)
                        .or_default()
                        .push(name.to_string());
                } else {
                    fonts
                        .families
                        .entry(family)
                        .or_default()
                        .insert(0, name.to_string());
                }
            }
            ctx.set_fonts(fonts);
        } else {
            message = "未找到中文字体，请安装微软雅黑或黑体后重新打开".into();
        }
        let hwnd = cc
            .window_handle()
            .ok()
            .and_then(|window| match window.as_raw() {
                RawWindowHandle::Win32(handle) => Some(handle.hwnd.get()),
                _ => None,
            })
            .unwrap_or(0);
        let wake: crate::interfaces::Wake = Arc::new(move || ctx.request_repaint());
        let services = Services {
            settings: dependencies.settings,
            cleaner: crate::application::cleanup::CleanupService::new(
                dependencies.cleaner,
                dependencies.cleanup_history,
                wake.clone(),
            ),
            desktop: (dependencies.desktop)(hwnd, wake.clone()),
        };
        if let Some(warning) = services.desktop.warning() {
            message = warning;
        }
        if config.cleanup.hotkey
            && let Err(e) = services.desktop.set_clean_hotkey(true)
        {
            message = e;
        }
        let worker = MonitorWorker::start(dependencies.sampler, config.clone(), wake);
        Self {
            saved_position: config.position,
            config,
            snapshot: Snapshot::default(),
            worker,
            services,
            settings: Settings::default(),
            expanded: false,
            locked: false,
            history: HashMap::new(),
            alerts: Alerts::default(),
            message,
            tooltip_text: String::new(),
            dirty_at: None,
            size: egui::Vec2::ZERO,
            pets: pet::Sprites::default(),
            flyout: flyout::Flyout::default(),
            hover_quick: false,
            pet_hover: false,
        }
    }
    fn configure(&mut self) {
        self.worker.configure(&self.config, self.expanded);
        self.dirty_at = Some(Instant::now());
    }
    fn unlock(&mut self, ctx: &egui::Context) {
        self.locked = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }
    fn receive(&mut self, ctx: &egui::Context) {
        {
            let actions = self.services.desktop.actions();
            for action in actions {
                match action {
                    Action::Unlock => {
                        self.unlock(ctx);
                        if self.config.compact || self.config.pet_mode {
                            self.toggle_mode(ctx);
                        }
                    }
                    Action::Passthrough => {
                        self.locked = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(true));
                    }
                    Action::Settings => {
                        self.unlock(ctx);
                        self.settings.open = true;
                    }
                    Action::ToggleMode => self.toggle_mode(ctx),
                    Action::Pet => self.toggle_pet(ctx),
                    Action::PetReact => self.pets.react(false),
                    Action::PetDrop => self.pets.react(true),
                    Action::Cleanup => self.flyout.open(),
                    Action::Exit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                }
            }
        }
        if let Some(snapshot) = self.worker.take_sample() {
            let now = Instant::now();
            let rows = panel::rows(&self.config, &snapshot);
            self.history
                .retain(|key, _| rows.iter().any(|r| &r.key == key));
            self.alerts.sample(&rows, self.config.warnings, now);
            for item in &rows {
                let history = self.history.entry(item.key.clone()).or_default();
                if let Some(value) = item.reading.value {
                    history.push_back((now, value));
                }
                while history
                    .front()
                    .is_some_and(|(t, _)| now.duration_since(*t) > Duration::from_secs(60))
                {
                    history.pop_front();
                }
                while history.len() > 61 {
                    history.pop_front();
                }
            }
            self.snapshot = snapshot;
            {
                let mut text = panel::indicators(&rows)
                    .iter()
                    .map(|i| i.tooltip.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                text.push_str(
                    "\n悬停图标栏可切换视图、设置和清理；托盘单击 / Ctrl+Alt+M 恢复完整视图",
                );
                self.tooltip_text = text;
            }
        }
        self.services.cleaner.tick(
            &self.config.cleanup,
            self.settings.open || self.flyout.active(),
        );
    }
    fn card(&mut self, ctx: &egui::Context) {
        self.pet_hover = false;
        self.hover_quick = false;
        let color = Color32::from_rgb(
            self.config.color[0],
            self.config.color[1],
            self.config.color[2],
        );
        let rows = panel::rows(&self.config, &self.snapshot);
        let screen_height = ctx
            .input(|i| i.viewport().monitor_size.map(|v| v.y))
            .unwrap_or(900.0);
        let size = surface::size(&self.config, &rows, self.expanded, screen_height);
        if self.size != size {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
            self.size = size;
        }
        self.services.desktop.configure_card(
            self.config.compact || self.config.pet_mode,
            !self.locked,
            if self.config.compact || self.config.pet_mode {
                size.y
            } else {
                panel::height(&rows, false).min(size.y)
            },
            if self.config.pet_mode {
                0.0
            } else if self.config.compact {
                156.0
            } else {
                185.0
            },
        );
        if self.config.pet_mode {
            let cursor = if self.config.pet_animation {
                self.services.desktop.cursor()
            } else {
                None
            };
            let (hover, menu) = self.pets.show(ctx, &self.config, &self.snapshot, cursor);
            self.pet_hover = hover;
            self.services.desktop.tooltip("", false);
            if menu {
                self.context_menu(ctx);
            }
            return;
        }
        egui::CentralPanel::default()
            .frame(surface::frame(&self.config))
            .show(ctx, |ui| {
                let drag = ui.interact(
                    ui.max_rect(),
                    ui.id().with("move-card"),
                    egui::Sense::click_and_drag(),
                );
                if self.config.compact && drag.double_clicked() {
                    self.toggle_mode(ctx);
                }
                if drag.secondary_clicked() {
                    self.context_menu(ctx);
                }
                ui.spacing_mut().item_spacing.y = 2.0;
                if self.config.compact {
                    panel::compact(ui, &rows, self.config.status_colors, color);
                    return;
                }
                egui::ScrollArea::vertical()
                    .id_salt("card-scroll")
                    .drag_to_scroll(false)
                    .max_height(size.y - 18.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for item in &rows {
                            panel::metric(
                                ui,
                                item,
                                &self.history,
                                self.alerts.active(&item.key),
                                self.config.status_colors,
                                color,
                            );
                        }
                        if rows.is_empty() {
                            ui.label("悬停后点击设置添加指标");
                        }
                        if self.expanded && details::show(ui, &self.snapshot, &self.message) {
                            self.settings.open_cleanup();
                        }
                    });
            });
        self.hover_controls(ctx);
    }
    fn toggle_mode(&mut self, ctx: &egui::Context) {
        self.config.compact = !self.config.pet_mode && !self.config.compact;
        self.config.pet_mode = false;
        if self.config.compact && self.expanded {
            self.expanded = false;
            self.configure();
        }
        self.dirty_at = Some(Instant::now());
        ctx.request_repaint();
    }
    fn toggle_pet(&mut self, ctx: &egui::Context) {
        self.config.pet_mode = !self.config.pet_mode;
        self.config.compact = false;
        if self.config.pet_mode && self.expanded {
            self.expanded = false;
            self.configure();
        }
        self.dirty_at = Some(Instant::now());
        ctx.request_repaint();
    }
    fn context_menu(&mut self, ctx: &egui::Context) {
        match self
            .services
            .desktop
            .menu(self.config.compact || self.config.pet_mode)
        {
            1 => self.toggle_mode(ctx),
            2 => self.settings.open = true,
            3 => self.config.background = !self.config.background,
            4 => self.config.border = !self.config.border,
            5 => self.toggle_pet(ctx),
            6 => self.flyout.open(),
            _ => return,
        }
        self.dirty_at = Some(Instant::now());
    }
    fn hover_controls(&mut self, ctx: &egui::Context) {
        use controls::Action;
        let (action, tip) = controls::toolbar(ctx, self.config.compact, self.locked, self.expanded);
        self.hover_quick = tip == Some("快速清理低优先级待机缓存");
        self.services.desktop.tooltip(
            tip.unwrap_or(&self.tooltip_text),
            !self.hover_quick && !self.flyout.active(),
        );
        match action {
            Some(Action::Settings) => self.settings.open = true,
            Some(Action::Lock) => {
                if self.locked {
                    self.unlock(ctx);
                } else {
                    self.locked = true;
                }
            }
            Some(Action::Details) => {
                self.expanded = !self.expanded;
                self.configure();
            }
            Some(Action::ToggleMode) => self.toggle_mode(ctx),
            Some(Action::Cleanup) => self.flyout.open(),
            Some(Action::Pet) => self.toggle_pet(ctx),
            None => {}
        }
    }
}
impl eframe::App for MonitorApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive(ctx);
        self.card(ctx);
        let trigger = self.pet_hover || self.hover_quick;
        let geometry = if trigger || self.flyout.active() || self.flyout.pinned {
            self.services.desktop.geometry()
        } else {
            None
        };
        match self.flyout.show(
            ctx,
            geometry,
            &self.config,
            &self.snapshot,
            &mut self.services.cleaner,
            trigger,
        ) {
            Some(flyout::Action::Restore) => {
                self.config.pet_mode = false;
                self.config.compact = false;
                self.configure();
            }
            Some(flyout::Action::Compact) => {
                self.config.pet_mode = false;
                self.config.compact = true;
                self.expanded = false;
                self.configure();
            }
            Some(flyout::Action::Settings) => self.settings.open = true,
            Some(flyout::Action::AdvancedClean) => self.settings.open_cleanup(),
            None => {}
        }
        let previous = self.config.clone();
        if self.flyout.active() {
            self.services.desktop.configure_auxiliary();
        }
        if self.settings.show(
            ctx,
            &mut self.config,
            &self.snapshot,
            &mut self.message,
            &mut self.services,
        ) {
            self.config.normalize();
            self.alerts.clear();
            let devices_changed = (&previous.gpu, &previous.disk, &previous.network)
                != (&self.config.gpu, &self.config.disk, &self.config.network);
            if devices_changed {
                self.history.clear();
            }
            if (self.config.compact || self.config.pet_mode) && self.expanded {
                self.expanded = false;
                self.configure();
            }
            let categories = ["CPU", "内存", "GPU", "磁盘", "网络"];
            if previous.seconds != self.config.seconds
                || categories
                    .iter()
                    .any(|g| previous.enabled(g) != self.config.enabled(g))
                || devices_changed
            {
                self.configure();
            }
            self.services.desktop.set_snapping(self.config.snap_edges);
            if previous.cleanup.hotkey != self.config.cleanup.hotkey
                && let Err(e) = self
                    .services
                    .desktop
                    .set_clean_hotkey(self.config.cleanup.hotkey)
            {
                self.config.cleanup.hotkey = false;
                self.message = e;
            }
            self.dirty_at = Some(Instant::now());
            ctx.request_repaint();
        }
        if let Some(position) = ctx.input(|i| i.viewport().outer_rect.map(|r| [r.left(), r.top()]))
            && self.saved_position != Some(position)
        {
            self.saved_position = Some(position);
            self.config.position = Some(position);
            self.dirty_at = Some(Instant::now());
        }
        if self
            .dirty_at
            .is_some_and(|t| t.elapsed() >= Duration::from_millis(500))
        {
            if let Err(e) = self.services.settings.save(&self.config) {
                self.message = format!("设置保存失败：{e}");
            }
            self.dirty_at = None;
        }
        if self.dirty_at.is_some() {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
        if self.config.cleanup.auto_load || self.config.cleanup.auto_interval {
            ctx.request_repaint_after(Duration::from_secs(2));
        }
    }
}
impl Drop for MonitorApp {
    fn drop(&mut self) {
        let _ = self.services.settings.save(&self.config);
    }
}
