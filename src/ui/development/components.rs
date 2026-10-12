use crate::ui::common::{
    actions,
    components::{
        color, fields, selection,
        status::{self, Tone},
    },
    icons::{self, Icon},
    theme, widgets,
};
use eframe::egui::{self, RichText};

pub struct Gallery {
    title: String,
    body: String,
    invalid: String,
    enabled: bool,
    completed: bool,
    threshold: f64,
    device: String,
    selected: usize,
    color: [u8; 3],
}
impl Default for Gallery {
    fn default() -> Self {
        Self {
            title: "整理下一阶段的工作".into(),
            body: "保留想法、行动和工作线索。\n中文输入、选择和换行沿用原生编辑行为。".into(),
            invalid: "2026-13-12 25:00".into(),
            enabled: true,
            completed: true,
            threshold: 85.0,
            device: "自动选择已连接的设备".into(),
            selected: 0,
            color: [165, 213, 200],
        }
    }
}
impl Gallery {
    pub fn show(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BACKGROUND)
                    .corner_radius(16)
                    .inner_margin(28),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) =
                        ui.allocate_exact_size(egui::Vec2::splat(46.0), egui::Sense::hover());
                    icons::draw(ui.painter(), r, Icon::Palette, egui::Color32::WHITE);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("桌面仪表 · 基础组件").size(26.0).strong());
                        ui.label(
                            RichText::new("深蓝灰材质 / 玉绿交互 / 暖白信息")
                                .size(13.0)
                                .color(theme::MUTED),
                        );
                    });
                });
                ui.add_space(18.0);
                ui.columns(2, |columns| {
                    theme::glass_panel(&mut columns[0], |ui| {
                        heading(ui, "编辑与选择", "保留输入行为，统一边界与留白");
                        ui.label("标题");
                        fields::line(ui, &mut self.title, "记录一个标题", 64);
                        ui.label("内容");
                        fields::multiline(ui, &mut self.body, "写下工作线索", 8192, 3);
                        ui.add_space(8.0);
                        widgets::switch(ui, &mut self.enabled, "开启到期提醒");
                        selection::checkbox(ui, &mut self.completed, "标记这一项已完成");
                        ui.add_enabled_ui(false, |ui| {
                            let mut off = false;
                            widgets::switch(ui, &mut off, "当前不可用的操作");
                        });
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            for (index, label) in ["便签", "待办", "已完成"].iter().enumerate()
                            {
                                if selection::choice(ui, label, self.selected == index, 96.0)
                                    .clicked()
                                {
                                    self.selected = index;
                                }
                            }
                        });
                        ui.add_space(8.0);
                        selection::dropdown(ui, "gallery-device", &self.device.clone(), |ui| {
                            for name in [
                                "自动选择已连接的设备",
                                "显示器 1 · 主屏幕",
                                "显示器 2 · 扩展屏幕",
                            ] {
                                if selection::choice(
                                    ui,
                                    name,
                                    self.device == name,
                                    ui.available_width(),
                                )
                                .clicked()
                                {
                                    self.device = name.into();
                                }
                            }
                        });
                        ui.add_space(8.0);
                        widgets::number(ui, "内存提醒阈值", &mut self.threshold, 50.0..=99.0, "%");
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label("监测主题色");
                            color::picker(ui, &mut self.color);
                        });
                    });
                    theme::glass_panel(&mut columns[1], |ui| {
                        heading(ui, "操作与反馈", "主次清晰，状态不依赖颜色辨认");
                        actions::primary(ui, Icon::Save, "保存到本地", true);
                        ui.horizontal(|ui| {
                            actions::secondary(ui, Icon::Screenshot, "选择截图区域");
                            actions::secondary(ui, Icon::Restore, "取消编辑");
                        });
                        actions::primary(ui, Icon::Save, "正在保存 · 暂不可操作", false);
                        ui.horizontal(|ui| {
                            for (icon, label) in [
                                (Icon::Edit, "编辑"),
                                (Icon::Pin, "收藏"),
                                (Icon::Settings, "设置"),
                                (Icon::Cancel, "关闭"),
                            ] {
                                icons::sized_button(ui, icon, label, 36.0).on_hover_text(label);
                            }
                        });
                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            for (label, tone) in [
                                ("本地文件", Tone::Neutral),
                                ("已保存", Tone::Success),
                                ("即将到期", Tone::Warning),
                                ("保存失败", Tone::Error),
                            ] {
                                status::badge(ui, label, tone);
                            }
                        });
                        ui.add_space(8.0);
                        ui.label("需要修正的输入");
                        fields::entry(
                            ui,
                            egui::TextEdit::singleline(&mut self.invalid)
                                .desired_width(ui.available_width()),
                            true,
                        );
                        status::message(ui, "日期无效，请检查月份、日期和时间。", Tone::Error);
                        ui.add_space(8.0);
                        status::message(ui, "删除前确认条目，取消后保留原内容。", Tone::Warning);
                        actions::danger(ui, Icon::Cancel, "确认删除", true);
                    });
                });
                ui.add_space(16.0);
                ui.label(
                    RichText::new(
                        "真实 Rust 组件预览 · 与业务页面共用代码 · 示例数据不会写入工作文件",
                    )
                    .size(12.0)
                    .color(theme::MUTED),
                );
            });
    }
}
fn heading(ui: &mut egui::Ui, title: &str, detail: &str) {
    ui.label(RichText::new(title).size(18.0).strong());
    ui.label(RichText::new(detail).size(12.0).color(theme::MUTED));
    ui.add_space(12.0);
}
