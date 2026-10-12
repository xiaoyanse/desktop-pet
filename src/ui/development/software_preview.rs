use crate::ui::{
    common::{
        artwork, fonts,
        icons::{self, Icon},
        theme,
    },
    companion::pet,
};
use crate::{domain::desktop::PetKind, interfaces::FontSource};
use eframe::egui::{self, Color32, RichText};

#[derive(Default)]
pub(crate) struct Actions {
    pub maximize: Option<bool>,
    pub close: bool,
    pub open_settings: bool,
    pub drag: bool,
}

pub(crate) struct Preview {
    root: bool,
    pets: pet::Sprites,
    enabled: [bool; 4],
}

impl Preview {
    pub fn new(ctx: &egui::Context, root: bool, font: FontSource) -> Self {
        theme::setup(ctx);
        fonts::install(ctx, font);
        Self {
            root,
            pets: pet::Sprites::default(),
            enabled: [true; 4],
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, maximized: bool) -> Actions {
        let mut action = Actions::default();
        if self.root {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ctx, |ui| {
                    let texture = self.pets.texture(ctx, PetKind::Cat, false);
                    ui.painter().image(
                        texture.id(),
                        egui::Rect::from_min_size(egui::pos2(0.0, 12.0), egui::vec2(96.0, 96.0)),
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    egui::Area::new(egui::Id::new("software-root-controls"))
                        .fixed_pos(egui::pos2(98.0, 36.0))
                        .show(ctx, |ui| {
                            ui.label(RichText::new("透明绘制实验").color(theme::ACCENT));
                            ui.horizontal(|ui| {
                                action.open_settings =
                                    icons::button(ui, Icon::Settings, "打开实验设置").clicked();
                                action.close =
                                    icons::button(ui, Icon::Cancel, "退出绘制实验").clicked();
                            });
                        });
                });
            return action;
        }
        let rect = ctx.screen_rect();
        artwork::draw(
            &ctx.layer_painter(egui::LayerId::background()),
            rect,
            "workspace",
            Color32::WHITE,
        );
        // Keep real Chinese text legible over the artwork's bright upper highlights.
        ctx.layer_painter(egui::LayerId::background()).rect_filled(
            egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), 152.0)),
            0,
            Color32::from_rgba_unmultiplied(17, 25, 35, 228),
        );
        egui::TopBottomPanel::top("software-title")
            .frame(egui::Frame::NONE)
            .exact_height(52.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    icons::draw(
                        ui.painter(),
                        egui::Rect::from_min_size(ui.cursor().min, egui::vec2(24.0, 24.0)),
                        Icon::Monitor,
                        Color32::WHITE,
                    );
                    ui.add_space(32.0);
                    ui.label(RichText::new("软件绘制实验 · 开发入口").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(12.0);
                        action.close = icons::button(ui, Icon::Cancel, "关闭实验设置").clicked();
                        if icons::button(
                            ui,
                            if maximized {
                                Icon::Restore
                            } else {
                                Icon::Maximize
                            },
                            "最大化或还原",
                        )
                        .clicked()
                        {
                            action.maximize = Some(!maximized);
                        }
                    });
                });
                let drag_rect = egui::Rect::from_min_max(
                    egui::pos2(260.0, 0.0),
                    egui::pos2(rect.right() - 90.0, 48.0),
                );
                action.drag = ui.input(|i| {
                    i.pointer.button_pressed(egui::PointerButton::Primary)
                        && i.pointer
                            .press_origin()
                            .is_some_and(|p| drag_rect.contains(p))
                });
            });
        egui::SidePanel::left("software-nav")
            .exact_width(186.0)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(17, 29, 41, 180))
                    .inner_margin(16),
            )
            .show(ctx, |ui| {
                ui.label(RichText::new("材质与文字对照").small().color(theme::MUTED));
                ui.add_space(20.0);
                for (icon, label) in [
                    (Icon::Monitor, "监测项目"),
                    (Icon::Pet, "外观与宠物"),
                    (Icon::Devices, "监测设备"),
                    (Icon::General, "常规设置"),
                    (Icon::Clean, "内存清理"),
                    (Icon::History, "清理记录"),
                    (Icon::Timer, "工作计时"),
                    (Icon::Report, "诊断报告"),
                ] {
                    ui.horizontal(|ui| {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                        icons::draw(ui.painter(), icon_rect, icon, Color32::WHITE);
                        ui.label(label);
                    });
                    ui.add_space(16.0);
                }
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.inner_margin(26))
            .show(ctx, |ui| {
                ui.heading(RichText::new("渲染兼容性").size(28.0));
                ui.label(
                    RichText::new("复用现有中文字体、AI 图标与玻璃控件；数据为合成测试值。")
                        .color(theme::MUTED),
                );
                ui.add_space(24.0);
                theme::glass_panel(ui, |ui| {
                    ui.label(
                        RichText::new("开发实验，不是正常应用入口")
                            .color(theme::CHAMPAGNE)
                            .strong(),
                    );
                    ui.label("没有采样、清理、录音、报告任务；这里只验证绘制与鼠标输入。");
                    ui.label("最大化、还原、关闭和再次打开用于检查原生缓冲的生命周期。");
                });
                ui.add_space(18.0);
                for (index, (name, value, fraction, color)) in [
                    ("CPU 占用", "42%", 0.42, Color32::from_rgb(129, 203, 157)),
                    ("内存占用", "78%", 0.78, Color32::from_rgb(227, 187, 102)),
                    (
                        "非分页池",
                        "10.4 GB",
                        0.91,
                        Color32::from_rgb(233, 123, 134),
                    ),
                    ("磁盘读取", "126 MB/s", 0.24, theme::ACCENT),
                ]
                .into_iter()
                .enumerate()
                {
                    theme::glass_panel(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.enabled[index], name);
                            ui.label(RichText::new(value).color(color).strong());
                        });
                        ui.add(
                            egui::ProgressBar::new(fraction)
                                .fill(color)
                                .desired_width(ui.available_width()),
                        );
                    });
                    ui.add_space(12.0);
                }
            });
        action
    }
}
