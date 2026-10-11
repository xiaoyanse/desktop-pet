use super::{
    cleanup_page,
    icons::{self, Icon},
    settings_shell, theme,
};
use crate::{
    application::Services,
    domain::{
        config::{Config, Metric},
        resources::{Device, Snapshot},
    },
};
use eframe::egui::{self, Color32, RichText};

#[derive(Default)]
pub struct Settings {
    tab: usize,
    pub open: bool,
    editing: Option<Metric>,
    logo: Option<egui::TextureHandle>,
    confirm_clean: crate::domain::cleanup_profile::Review,
    pets: super::pet::Sprites,
}
impl Settings {
    pub fn open_cleanup(&mut self) {
        self.tab = 4;
        self.open = true;
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
            return false;
        }
        if self.logo.is_none() {
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [64, 64],
                include_bytes!(concat!(env!("OUT_DIR"), "/icon-64.rgba")),
            );
            self.logo = Some(ctx.load_texture("app-logo", image, egui::TextureOptions::LINEAR));
        }
        let before = serde_json::to_string(config).unwrap();
        let mut closed = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("settings"),
            egui::ViewportBuilder::default()
                .with_title("资源监视器 · 设置")
                .with_taskbar(true)
                .with_inner_size([920.0, 780.0])
                .with_min_inner_size([840.0, 600.0])
                .with_always_on_top(),
            |ctx, _| {
                if ctx.input(|i| i.viewport().close_requested()) {
                    closed = true;
                }
                if let Some(tab) =
                    settings_shell::navigation(ctx, self.tab, self.logo.as_ref(), config)
                {
                    self.confirm_clean.cancel();
                    self.tab = tab;
                }
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(theme::BACKGROUND).inner_margin(28))
                    .show(ctx, |ui| {
                        closed |= settings_shell::header(ui, self.tab, config, snapshot);
                        egui::ScrollArea::vertical()
                            .id_salt(("settings-scroll", self.tab))
                            .auto_shrink([false, false])
                            .show(ui, |ui| match self.tab {
                                0 => metrics(ui, config, &mut self.editing),
                                1 => super::appearance::show(ui, config, &mut self.pets),
                                2 => devices(ui, config, snapshot),
                                3 => general(ui, config, message, services),
                                4 => cleanup_page::show(
                                    ui,
                                    config,
                                    &mut services.cleaner,
                                    &mut self.confirm_clean,
                                ),
                                _ => cleanup_page::history(ui, &services.cleaner),
                            });
                        if !message.is_empty() {
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new(message.as_str())
                                    .size(12.0)
                                    .color(Color32::from_rgb(248, 200, 82)),
                            );
                        }
                    });
            },
        );
        if closed {
            self.open = false;
            self.confirm_clean.cancel();
        }
        before != serde_json::to_string(config).unwrap()
    }
}
pub fn section(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(theme::SURFACE)
        .stroke(egui::Stroke::new(1.0_f32, theme::LINE))
        .corner_radius(10)
        .inner_margin(18)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(14.0).strong().color(theme::TEXT));
            ui.add_space(12.0);
            content(ui);
        });
    ui.add_space(14.0);
}
fn metrics(ui: &mut egui::Ui, config: &mut Config, editing: &mut Option<Metric>) {
    ui.label(
        RichText::new("绿色 < 黄色阈值 · 黄色 < 红色阈值 · 红色 ≥ 红色阈值")
            .size(12.0)
            .color(Color32::from_rgb(141, 188, 181)),
    );
    ui.add_space(14.0);
    let mut movement = None;
    let len = config.rows.len();
    for (index, row) in config.rows.iter_mut().enumerate() {
        ui.push_id(row.metric, |ui| {
            egui::Frame::new()
                .fill(theme::SURFACE)
                .corner_radius(8)
                .inner_margin(10)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.allocate_ui(egui::vec2(135.0, 26.0), |ui| {
                            ui.checkbox(&mut row.enabled, row.metric.label());
                        });
                        ui.checkbox(&mut row.trend, "趋势");
                        if row.metric.color_supported() {
                            let summary = format!(
                                "{} / {}{}",
                                row.warning_threshold.round(),
                                row.threshold.round(),
                                if row.metric.color_percent() {
                                    "%"
                                } else {
                                    " GB"
                                }
                            );
                            ui.label(
                                RichText::new(summary)
                                    .size(11.0)
                                    .color(Color32::from_rgb(150, 171, 188)),
                            );
                            if icons::button(ui, Icon::General, "设置颜色阈值及持续告警").clicked()
                            {
                                *editing = if *editing == Some(row.metric) {
                                    None
                                } else {
                                    Some(row.metric)
                                };
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_enabled_ui(index + 1 < len, |ui| {
                                    icons::button(ui, Icon::Down, "下移")
                                })
                                .inner
                                .clicked()
                            {
                                movement = Some((index, index + 1));
                            }
                            if ui
                                .add_enabled_ui(index > 0, |ui| icons::button(ui, Icon::Up, "上移"))
                                .inner
                                .clicked()
                            {
                                movement = Some((index, index - 1));
                            }
                        });
                    });
                    if *editing == Some(row.metric) && row.metric.color_supported() {
                        ui.add_space(10.0);
                        let max = if row.metric.color_percent() {
                            100.0
                        } else {
                            (row.warning_threshold.max(row.threshold) * 2.0).max(32.0)
                        };
                        let suffix = if row.metric.color_percent() {
                            "%"
                        } else {
                            " GB"
                        };
                        ui.horizontal(|ui| {
                            ui.colored_label(Color32::from_rgb(248, 200, 82), "黄色");
                            ui.add(
                                egui::Slider::new(&mut row.warning_threshold, 0.0..=max)
                                    .suffix(suffix)
                                    .fixed_decimals(0),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.colored_label(Color32::from_rgb(255, 102, 112), "红色");
                            ui.add(
                                egui::Slider::new(&mut row.threshold, 0.0..=max)
                                    .suffix(suffix)
                                    .fixed_decimals(0),
                            );
                        });
                        if row.metric.low_warning() {
                            ui.weak("剩余容量越低压力越高：低于阈值变色。");
                        }
                        if row.metric.alert_supported() {
                            ui.checkbox(&mut row.alert, "连续超限 10 秒额外显示压力标记");
                        }
                    }
                });
            ui.add_space(5.0);
        });
    }
    if let Some((a, b)) = movement {
        config.rows.swap(a, b);
    }
    ui.add_space(12.0);
    ui.weak("GPU / 磁盘活动颜色表示负载；读写速度与内核池大小保持主题色。");
}
fn devices(ui: &mut egui::Ui, config: &mut Config, snapshot: &Snapshot) {
    section(ui, "显卡", |ui| {
        choose(
            ui,
            "gpu",
            &mut config.gpu,
            &snapshot.gpus,
            "自动：逐块显示所有硬件显卡",
        );
    });
    section(ui, "磁盘卷", |ui| {
        choose(
            ui,
            "disk",
            &mut config.disk,
            &snapshot.disks,
            "自动：第一块本地卷",
        );
        ui.weak("按卷（C:、E: 等）统计空间、活动率与读写速度。");
    });
    section(ui, "网络接口", |ui| {
        choose(
            ui,
            "network",
            &mut config.network,
            &snapshot.networks,
            "自动：已连接物理网卡合计",
        );
        ui.weak("自动模式排除虚拟网卡，避免重复计算。可手动指定 VPN 或虚拟网卡。");
    });
}
fn choose(ui: &mut egui::Ui, id: &str, current: &mut String, devices: &[Device], automatic: &str) {
    let label = if current.is_empty() {
        automatic.to_string()
    } else {
        devices
            .iter()
            .find(|d| &d.id == current)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| format!("{current}（当前未枚举）"))
    };
    egui::ComboBox::from_id_salt(id)
        .width(ui.available_width() - 24.0)
        .selected_text(label)
        .show_ui(ui, |ui| {
            ui.selectable_value(current, String::new(), automatic);
            for device in devices {
                ui.selectable_value(current, device.id.clone(), &device.name);
            }
        });
}
fn general(ui: &mut egui::Ui, config: &mut Config, message: &mut String, services: &Services) {
    section(ui, "刷新", |ui| {
        ui.horizontal(|ui| {
            for seconds in [1, 2, 5] {
                ui.radio_value(&mut config.seconds, seconds, format!("{seconds} 秒"));
            }
        });
        ui.weak("建议 2 秒；5 秒进一步减少常驻开销。");
    });
    section(ui, "压力提示与贴边", |ui| {
        ui.checkbox(&mut config.warnings, "连续超限 10 秒显示压力标记");
        ui.checkbox(&mut config.snap_edges, "靠近屏幕四边自动贴边");
        ui.weak("无论是否贴边，整个窗口均限制在屏幕工作区内；切换视图后仍保持可见。");
    });
    section(ui, "启动与恢复", |ui| {
        let mut autostart = services.desktop.autostart();
        if ui.checkbox(&mut autostart, "登录 Windows 时启动").changed() {
            *message = match services.desktop.set_autostart(autostart) {
                Ok(()) => "开机启动设置已更新".into(),
                Err(e) => e,
            };
        }
        ui.weak("首次默认关闭。保持可执行文件位置不变。");
        ui.add_space(10.0);
        ui.label("悬停显示图标操作栏；拖动数据区域移动。固定仅限制位置。鼠标穿透在托盘开启，单击托盘或 Ctrl+Alt+M 恢复完整视图。");
    });
}
