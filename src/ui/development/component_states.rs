use crate::ui::common::{
    actions,
    components::{
        color, fields, numeric, selection,
        status::{self, Tone},
    },
    icons::Icon,
    theme,
};
use eframe::egui;

pub struct States {
    value: f64,
    rgb: [u8; 3],
    text: String,
    invalid: String,
    flag: bool,
}
impl Default for States {
    fn default() -> Self {
        Self {
            value: 65.0,
            rgb: [165, 213, 200],
            text: "支持中文输入、选择和快捷键".into(),
            invalid: "尚未输入完整时间".into(),
            flag: true,
        }
    }
}
impl States {
    pub fn show(&mut self, ctx: &egui::Context, popup: bool) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BACKGROUND).inner_margin(28))
            .show(ctx, |ui| {
                ui.heading("桌面仪表 · 组件状态与配色");
                ui.label(
                    egui::RichText::new("同一套组件覆盖输入、调节、选择、禁用与错误反馈")
                        .color(theme::MUTED),
                );
                ui.add_space(18.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    theme::glass_panel(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new("数值调节").size(18.0).strong());
                        numeric::number(ui, "常规阈值", &mut self.value, 0.0..=100.0, "%");
                        ui.add_enabled_ui(false, |ui| {
                            let mut v = 75.0;
                            numeric::number(ui, "禁用状态", &mut v, 0.0..=100.0, "%");
                        });
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label("监测主题色");
                            let r = color::picker(ui, &mut self.rgb);
                            if popup {
                                ui.memory_mut(|m| m.open_popup(r.id.with("palette")));
                            }
                        });
                    });
                    ui.add_space(14.0);
                    theme::glass_panel(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new("输入与选择").size(18.0).strong());
                        fields::line(ui, &mut self.text, "记录工作线索", 128);
                        fields::entry(
                            ui,
                            egui::TextEdit::singleline(&mut self.invalid)
                                .desired_width(ui.available_width()),
                            true,
                        );
                        ui.horizontal(|ui| {
                            selection::choice(ui, "已选中", true, 110.0);
                            selection::choice(ui, "可选择", false, 110.0);
                            ui.add_enabled_ui(false, |ui| {
                                selection::choice(ui, "暂不可用", false, 110.0)
                            });
                        });
                        selection::checkbox(ui, &mut self.flag, "可通过标签或键盘操作");
                        ui.add_enabled_ui(false, |ui| {
                            let mut selected = true;
                            selection::checkbox(ui, &mut selected, "已选择但暂不可修改");
                        });
                    });
                    ui.add_space(14.0);
                    theme::glass_panel(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new("操作与反馈").size(18.0).strong());
                        ui.horizontal_wrapped(|ui| {
                            actions::secondary(ui, Icon::Copy, "复制内容");
                            ui.add_enabled_ui(false, |ui| {
                                actions::secondary(ui, Icon::Save, "正在保存")
                            });
                            for (label, tone) in [
                                ("已保存", Tone::Success),
                                ("即将到期", Tone::Warning),
                                ("需要修正", Tone::Error),
                            ] {
                                status::badge(ui, label, tone);
                            }
                        });
                        status::message(
                            ui,
                            "无法保存时保留当前内容，修正后可以重试。",
                            Tone::Error,
                        );
                    });
                });
            });
    }
}
