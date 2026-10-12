mod canvas;
mod pinned;
mod preview;
#[cfg(test)]
mod tests;
mod toolbar;
use crate::{
    application::screenshot::ScreenshotService,
    domain::screenshot::{Mode, Tool},
    ui::common::{actions, icons::Icon, theme},
};
use eframe::egui::{self, RichText};

pub struct Page {
    pub request: Option<Mode>,
    mode: Mode,
    tool: Tool,
    color: [u8; 3],
    width: u32,
    generation: u64,
    texture: Option<egui::TextureHandle>,
    stroke: Vec<[u32; 2]>,
    pin: pinned::Pinned,
}
impl Default for Page {
    fn default() -> Self {
        Self {
            request: None,
            mode: Mode::Rectangle,
            tool: Tool::Arrow,
            color: [239, 92, 83],
            width: 3,
            generation: u64::MAX,
            texture: None,
            stroke: vec![],
            pin: Default::default(),
        }
    }
}
impl Page {
    pub fn show(&mut self, ui: &mut egui::Ui, service: &mut ScreenshotService) {
        if self.generation != service.generation {
            self.texture = None;
            self.stroke.clear();
            self.generation = service.generation;
        }
        ui.add_enabled_ui(!service.busy(), |ui| {
            let width = ((ui.available_width() - 24.0) / 4.0).max(90.0);
            ui.horizontal(|ui| {
                for mode in Mode::ALL {
                    ui.allocate_ui_with_layout(
                        egui::vec2(width, 62.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            let text = RichText::new(mode.label()).size(14.0);
                            if ui
                                .add_sized(
                                    [width, 40.0],
                                    egui::Button::new(text).selected(self.mode == mode),
                                )
                                .on_hover_text(mode.description())
                                .clicked()
                            {
                                self.mode = mode;
                            }
                            ui.label(
                                RichText::new(match mode {
                                    Mode::Rectangle => "选区 · 精确调整",
                                    Mode::Freehand => "轮廓 · 外部透明",
                                    Mode::Window => "悬停 · 点击确认",
                                    Mode::Fullscreen => "桌面 · 包含副屏",
                                })
                                .size(11.0)
                                .color(theme::MUTED),
                            );
                        },
                    );
                }
            });
        });
        if actions::primary(
            ui,
            Icon::Screenshot,
            &format!("开始{}", self.mode.label()),
            !service.busy(),
        )
        .clicked()
        {
            self.request = Some(self.mode);
        }
        if service.capturing() && actions::secondary(ui, Icon::Cancel, "取消选区").clicked() {
            service.cancel();
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("保存到").color(theme::MUTED));
            ui.add_enabled(
                !service.busy(),
                egui::TextEdit::singleline(&mut service.folder)
                    .desired_width(ui.available_width())
                    .hint_text("绝对路径目录，如 E:\\Screenshots"),
            );
        });
        if !service.message.is_empty() {
            ui.label(
                RichText::new(&service.message)
                    .size(12.0)
                    .color(theme::ACCENT),
            );
        }
        ui.add_space(10.0);
        if service.document.is_some() {
            toolbar::show(ui, self, service);
            ui.add_space(8.0);
            canvas::show(ui, self, service);
        } else {
            theme::glass_panel(ui, |ui| {
                ui.set_min_height(130.0);
                ui.label(
                    RichText::new("把画面留下，把重点说清楚")
                        .size(22.0)
                        .strong(),
                );
                ui.add_space(10.0);
                ui.label("截图后可画箭头、框重点、遮挡敏感信息，再复制、保存或置顶参考。");
                ui.weak("按需截图，不会自动记录桌面或写入剪贴板。");
            });
        }
    }
    pub fn pinned(&mut self, ctx: &egui::Context, service: &mut ScreenshotService, hidden: bool) {
        self.pin.show(ctx, service, hidden);
    }
    pub fn hide(&mut self, ctx: &egui::Context) {
        self.pin.hide(ctx);
    }
    pub fn release(&mut self) {
        self.texture = None;
        self.stroke.clear();
    }
}
