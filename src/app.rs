use crate::{cleanup, config::Config, docking::Docking, metrics::Sampler, model::{Process, Snapshot}, native, panel::{self, Alerts, History}, settings::Settings, tooltip::Tooltip, tray::{Action, Tray}};
use eframe::egui::{self, Color32, RichText};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{collections::HashMap, sync::{Arc, Mutex, mpsc}, time::{Duration, Instant}};

enum Command { Configure(Config, bool), Stop }
pub struct MonitorApp {
    config: Config, snapshot: Snapshot, shared: Arc<Mutex<Option<Snapshot>>>,
    tx: mpsc::Sender<Command>, worker: Option<std::thread::JoinHandle<()>>,
    tray: Option<Tray>, settings: Settings, expanded: bool, locked: bool,
    history: History, alerts: Alerts,
    message: String, clean_rx: Option<mpsc::Receiver<String>>, confirm_clean: bool,
    saved_position: Option<[f32; 2]>, dirty_at: Option<Instant>, size: egui::Vec2,
    docking: Option<Docking>, tooltip: Option<Tooltip>, hwnd: isize,
}
impl MonitorApp {
    pub fn new(cc: &eframe::CreationContext<'_>, config: Config, warning: Option<String>) -> Self {
        let ctx = cc.egui_ctx.clone(); setup_visuals(&ctx);
        let mut message = warning.unwrap_or_default();
        if let Some((name, data)) = native::font_bytes() {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(name.to_string(), Arc::new(egui::FontData::from_owned(data)));
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts.families.entry(family).or_default().insert(0, name.to_string());
            }
            ctx.set_fonts(fonts);
        } else { message = "未找到中文字体，请安装微软雅黑或黑体后重新打开".into(); }
        let hwnd = cc.window_handle().ok().and_then(|window| match window.as_raw() {
            RawWindowHandle::Win32(handle) => Some(handle.hwnd.get()), _ => None,
        }).unwrap_or(0);
        if hwnd != 0 { native::safe_position(hwnd as _); }
        let docking = (hwnd != 0).then(|| Docking::attach(hwnd as _, config.snap_edges)).flatten();
        let tooltip = (hwnd != 0).then(|| Tooltip::new(hwnd as _)).flatten();
        if docking.is_none() { message = "窗口贴边初始化失败，本次无法自动贴边".into(); }
        let tray = Tray::start(ctx.clone()).map_err(|e| { message = e; }).ok();
        let shared = Arc::new(Mutex::new(None)); let worker_shared = shared.clone();
        let (tx, rx) = mpsc::channel::<Command>(); let initial = config.clone();
        let worker = std::thread::spawn(move || {
            let mut sampler = Sampler::new(); let mut config = initial; let mut expanded = false;
            loop {
                let sample = sampler.sample(&config, expanded);
                if let Ok(mut slot) = worker_shared.lock() { *slot = Some(sample); } else { break; }
                ctx.request_repaint();
                match rx.recv_timeout(Duration::from_secs(config.seconds)) {
                    Ok(Command::Configure(next, details)) => { config = next; expanded = details; }
                    Ok(Command::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
            }
        });
        Self { saved_position: config.position, config, snapshot: Snapshot::default(), shared, tx, worker: Some(worker),
            tray, settings: Settings::default(), expanded: false, locked: false, history: HashMap::new(), alerts: Alerts::default(), message,
            clean_rx: None, confirm_clean: false, dirty_at: None, size: egui::Vec2::ZERO, docking, tooltip, hwnd }
    }
    fn configure(&mut self) {
        let _ = self.tx.send(Command::Configure(self.config.clone(), self.expanded));
        self.dirty_at = Some(Instant::now());
    }
    fn unlock(&mut self, ctx: &egui::Context) {
        self.locked = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }
    fn receive(&mut self, ctx: &egui::Context) {
        if let Some(tray) = &self.tray {
            let actions: Vec<_> = tray.rx.try_iter().collect();
            for action in actions { match action {
                Action::Unlock => self.unlock(ctx),
                Action::Passthrough => { self.locked = true; ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(true)); }
                Action::Settings => { self.unlock(ctx); self.settings.open = true; }
                Action::ToggleMode => self.toggle_mode(ctx),
                Action::Exit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            } }
        }
        if let Ok(mut shared) = self.shared.lock()
            && let Some(snapshot) = shared.take() {
                let now = Instant::now();
                let rows = panel::rows(&self.config, &snapshot);
                self.history.retain(|key, _| rows.iter().any(|r| &r.key == key));
                self.alerts.sample(&rows, self.config.warnings, now);
                for item in &rows {
                    let history = self.history.entry(item.key.clone()).or_default();
                    if let Some(value) = item.reading.value { history.push_back((now, value)); }
                    while history.front().is_some_and(|(t, _)| now.duration_since(*t) > Duration::from_secs(60)) { history.pop_front(); }
                    while history.len() > 61 { history.pop_front(); }
                }
                self.snapshot = snapshot;
                if let Some(tooltip) = &mut self.tooltip {
                    let mut text = panel::indicators(&rows).iter().map(|i| i.tooltip.as_str()).collect::<Vec<_>>().join("\n");
                    text.push_str("\n绿：低于黄色阈值；黄：接近红色阈值；红：达到红色阈值\nC CPU · M 内存 · G 显卡 · D 磁盘 · N 网络\n双击恢复正常视图，右键打开菜单");
                    tooltip.update(&text);
                }
        }
        if let Some(rx) = &self.clean_rx {
            match rx.try_recv() {
                Ok(text) => { self.message = text; self.clean_rx = None; }
                Err(mpsc::TryRecvError::Disconnected) => { self.message = "清理辅助任务意外退出，未确认清理成功".into(); self.clean_rx = None; }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
    }
    fn card(&mut self, ctx: &egui::Context) {
        let color = Color32::from_rgb(self.config.color[0], self.config.color[1], self.config.color[2]);
        let rows = panel::rows(&self.config, &self.snapshot);
        let screen_height = ctx.input(|i| i.viewport().monitor_size.map(|v| v.y)).unwrap_or(900.0);
        let size = if self.config.compact { panel::compact_size(&rows) }
            else { egui::vec2(240.0, panel::height(&rows, self.expanded).min((screen_height - 80.0).max(180.0))) };
        if self.size != size { ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size)); self.size = size; }
        if let Some(tooltip) = &mut self.tooltip { tooltip.activate(self.config.compact); }
        let background = if self.config.background { Color32::from_rgba_unmultiplied(17, 23, 31, (self.config.opacity * 255.0) as u8) } else { Color32::TRANSPARENT };
        let border = if self.config.border { egui::Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(160, 180, 205, 70)) } else { egui::Stroke::NONE };
        egui::CentralPanel::default().frame(egui::Frame::new()
            .fill(background).corner_radius(9).inner_margin(if self.config.compact { 6 } else { 9 }).stroke(border))
            .show(ctx, |ui| {
                let drag = ui.interact(ui.max_rect(), ui.id().with("move-card"), egui::Sense::click_and_drag());
                if drag.drag_started() && !self.locked { ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag); }
                if self.config.compact && drag.double_clicked() { self.toggle_mode(ctx); }
                if drag.secondary_clicked() {
                    match native::card_menu(self.hwnd as _, self.config.compact) {
                        1 => self.toggle_mode(ctx), 2 => self.settings.open = true,
                        3 => { self.config.background = !self.config.background; self.dirty_at = Some(Instant::now()); }
                        4 => { self.config.border = !self.config.border; self.dirty_at = Some(Instant::now()); }
                        _ => {}
                    }
                }
                ui.spacing_mut().item_spacing.y = 2.0;
                if self.config.compact { panel::compact(ui, &rows, self.config.status_colors, color); return; }
                egui::ScrollArea::vertical().id_salt("card-scroll").max_height(size.y - 18.0).auto_shrink([false, false]).show(ui, |ui| {
                    for item in &rows {
                        panel::metric(ui, item, &self.history, self.alerts.active(&item.key), self.config.status_colors, color);
                    }
                    if rows.is_empty() { ui.label("悬停后点击设置添加指标"); }
                    if self.expanded { self.details(ui, ctx); }
                });
            });
        if !self.config.compact { self.hover_controls(ctx); }
    }
    fn toggle_mode(&mut self, ctx: &egui::Context) {
        self.config.compact = !self.config.compact;
        if self.config.compact && self.expanded { self.expanded = false; self.configure(); }
        self.confirm_clean = false; self.dirty_at = Some(Instant::now()); ctx.request_repaint();
    }
    fn hover_controls(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.pointer.hover_pos().is_some()) { return; }
        egui::Area::new(egui::Id::new("hover-controls")).order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(3.0, 3.0)).movable(false).show(ctx, |ui| {
                egui::Frame::new().fill(Color32::from_rgb(28, 37, 48)).corner_radius(6).inner_margin(4).show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(3.0, 0.0);
                    ui.horizontal(|ui| {
                        if ui.small_button("设置").clicked() { self.settings.open = true; }
                        if ui.small_button(if self.locked { "解锁" } else { "固定" }).clicked() {
                            if self.locked { self.unlock(ctx); } else { self.locked = true; }
                        }
                        if ui.small_button(if self.expanded { "收起" } else { "详情" }).clicked() { self.expanded = !self.expanded; self.configure(); }
                        if ui.small_button("精简").clicked() { self.toggle_mode(ctx); }
                    });
                });
            });
    }
    fn details(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.separator(); ui.add_space(8.0);
        process_list(ui, "CPU 前五", &self.snapshot.cpu_top, true);
        ui.add_space(12.0); process_list(ui, "内存前五 · 物理工作集", &self.snapshot.memory_top, false);
        ui.label(RichText::new(&self.snapshot.process_status).size(11.0).color(Color32::GRAY));
        ui.add_space(14.0); ui.separator(); ui.add_space(10.0);
        ui.label(RichText::new("手动清理低优先级缓存").strong());
        ui.label(RichText::new("待机缓存本来就可复用；清理不会修复非分页池泄漏。").size(12.0).color(Color32::GRAY));
        if ui.add_enabled(self.clean_rx.is_none(), egui::Button::new(if self.clean_rx.is_some() { "正在处理…" } else { "清理低优先级待机缓存" })).clicked() { self.confirm_clean = true; }
        if !self.message.is_empty() { ui.label(RichText::new(&self.message).size(12.0)); }
        if self.confirm_clean {
            egui::Window::new("执行手动清理").collapsible(false).resizable(false).default_width(285.0).show(ctx, |ui| {
                ui.label("只清理优先级 0 的待机缓存。不会压缩应用工作集。Windows 可能要求管理员授权。");
                ui.horizontal(|ui| {
                    if ui.button("清理一次").clicked() { self.clean_rx = Some(cleanup::request(ctx.clone())); self.message = "等待 Windows 授权及清理结果…".into(); self.confirm_clean = false; }
                    if ui.button("取消").clicked() { self.confirm_clean = false; }
                });
            });
        }
    }
}
impl eframe::App for MonitorApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] { [0.0; 4] }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive(ctx); self.card(ctx);
        let previous = self.config.clone();
        if self.settings.show(ctx, &mut self.config, &self.snapshot, &mut self.message) {
            self.config.normalize();
            self.alerts.clear();
            let devices_changed = (&previous.gpu, &previous.disk, &previous.network) != (&self.config.gpu, &self.config.disk, &self.config.network);
            if devices_changed { self.history.clear(); }
            if self.config.compact && self.expanded { self.expanded = false; self.configure(); }
            let categories = ["CPU", "内存", "GPU", "磁盘", "网络"];
            if previous.seconds != self.config.seconds || categories.iter().any(|g| previous.enabled(g) != self.config.enabled(g)) || devices_changed { self.configure(); }
            if let Some(docking) = &self.docking { docking.set_enabled(self.config.snap_edges); }
            self.dirty_at = Some(Instant::now());
            ctx.request_repaint();
        }
        if let Some(position) = ctx.input(|i| i.viewport().outer_rect.map(|r| [r.left(), r.top()]))
            && self.saved_position != Some(position) {
            self.saved_position = Some(position); self.config.position = Some(position); self.dirty_at = Some(Instant::now());
        }
        if self.dirty_at.is_some_and(|t| t.elapsed() >= Duration::from_millis(500)) {
            if let Err(e) = self.config.save() { self.message = format!("设置保存失败：{e}"); }
            self.dirty_at = None;
        }
        if self.dirty_at.is_some() { ctx.request_repaint_after(Duration::from_millis(500)); }
    }
}
impl Drop for MonitorApp {
    fn drop(&mut self) {
        let _ = self.config.save(); let _ = self.tx.send(Command::Stop);
        if let Some(worker) = self.worker.take() { let _ = worker.join(); }
    }
}
fn setup_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(Color32::from_rgb(225, 232, 239));
    visuals.selection.bg_fill = Color32::from_rgb(39, 93, 91);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(37, 46, 58);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(53, 68, 83);
    visuals.window_fill = Color32::from_rgb(23, 29, 37);
    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 7.0);
        style.spacing.button_padding = egui::vec2(9.0, 5.0);
        style.animation_time = 0.08;
    });
}
fn process_list(ui: &mut egui::Ui, title: &str, items: &[Process], cpu: bool) {
    ui.label(RichText::new(title).size(13.0).strong());
    if items.is_empty() { ui.label("等待排行样本…"); }
    for item in items {
        ui.horizontal(|ui| {
            ui.add(egui::Label::new(RichText::new(&item.name).size(11.0)).truncate()).on_hover_text(format!("{} · PID {}", item.name, item.pid));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(if cpu { format!("{:.1}%", item.cpu) } else { format!("{:.2} GB", item.memory) }).size(11.0));
                ui.label(RichText::new(item.pid.to_string()).size(10.0).color(Color32::GRAY));
            });
        });
    }
}
