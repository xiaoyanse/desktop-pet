use crate::{config::Config, model::{Device, Snapshot}, native};
use eframe::egui::{self, Color32, RichText};

#[derive(Default)]
pub struct Settings { tab: usize, pub open: bool }
impl Settings {
    pub fn show(&mut self, ctx: &egui::Context, config: &mut Config, snapshot: &Snapshot, message: &mut String) -> bool {
        if !self.open { return false; }
        let before = serde_json::to_string(config).unwrap();
        let mut closed = false;
        ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("settings"),
            egui::ViewportBuilder::default().with_title("资源监视器 · 设置").with_inner_size([660.0, 690.0])
                .with_min_inner_size([600.0, 450.0]).with_always_on_top(), |ctx, _class| {
                if ctx.input(|i| i.viewport().close_requested()) { closed = true; }
                egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::from_rgb(19, 23, 29)).inner_margin(24)).show(ctx, |ui| {
                    ui.label(RichText::new("让桌面只显示你关心的数据").size(23.0).strong());
                    ui.add_space(6.0);
                    ui.label(RichText::new("设置即时生效并自动保存").color(Color32::from_rgb(150, 161, 174)));
                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        for (index, name) in ["监测项", "外观", "设备", "常规"].into_iter().enumerate() {
                            ui.selectable_value(&mut self.tab, index, name);
                        }
                    });
                    ui.add_space(14.0); ui.separator(); ui.add_space(12.0);
                    egui::ScrollArea::vertical().id_salt("settings-scroll").show(ui, |ui| {
                        match self.tab {
                            0 => metrics(ui, config),
                            1 => appearance(ui, config),
                            2 => devices(ui, config, snapshot),
                            _ => general(ui, config, message),
                        }
                    });
                    if !message.is_empty() { ui.add_space(12.0); ui.label(message.as_str()); }
                });
            });
        if closed { self.open = false; }
        before != serde_json::to_string(config).unwrap()
    }
}
fn metrics(ui: &mut egui::Ui, config: &mut Config) {
    ui.label("绿 / 黄 / 红即时区分负载。连续达到红色阈值 10 秒，可额外显示压力标记。");
    ui.add_space(12.0);
    let mut movement = None;
    let len = config.rows.len();
    egui::Grid::new("metric-settings").striped(true).spacing([10.0, 10.0]).min_col_width(70.0).show(ui, |ui| {
        ui.strong("显示项"); ui.strong("趋势");
        ui.colored_label(Color32::from_rgb(248, 200, 82), "黄色阈值");
        ui.colored_label(Color32::from_rgb(255, 102, 112), "红色阈值");
        ui.strong("持续标记"); ui.strong("顺序"); ui.end_row();
        for (index, row) in config.rows.iter_mut().enumerate() {
            ui.checkbox(&mut row.enabled, row.metric.label());
            ui.checkbox(&mut row.trend, "趋势");
            if row.metric.color_supported() {
                let suffix = if row.metric.color_percent() { "%" } else { " GB" };
                let max = if row.metric.color_percent() { 100.0 } else { 100000.0 };
                let prefix = if row.metric.low_warning() { "≤ " } else { "≥ " };
                ui.add(egui::DragValue::new(&mut row.warning_threshold).range(0.0..=max).speed(1.0).prefix(prefix).suffix(suffix));
                ui.add(egui::DragValue::new(&mut row.threshold).range(0.0..=max).speed(1.0).prefix(prefix).suffix(suffix));
            } else {
                ui.weak("—").on_hover_text("没有固定容量的数据使用主题色，不将速度或容量高直接判定为异常。");
                ui.weak("—");
            }
            if row.metric.alert_supported() { ui.checkbox(&mut row.alert, "10 秒"); }
            else { ui.weak("—"); }
            ui.horizontal(|ui| {
                if ui.add_enabled(index > 0, egui::Button::new("↑")).clicked() { movement = Some((index, index - 1)); }
                if ui.add_enabled(index + 1 < len, egui::Button::new("↓")).clicked() { movement = Some((index, index + 1)); }
            });
            ui.end_row();
        }
    });
    if let Some((a, b)) = movement { config.rows.swap(a, b); }
    ui.add_space(16.0);
    ui.label("默认百分比：低于 70% 绿色，70% 起黄色，90% 起红色。可用内存和剩余磁盘反向判断。GPU / 磁盘活动颜色表示负载；读写速度与内核池大小保持主题色。");
}
fn appearance(ui: &mut egui::Ui, config: &mut Config) {
    section(ui, "显示状态");
    ui.horizontal(|ui| {
        ui.selectable_value(&mut config.compact, false, "正常卡片");
        ui.selectable_value(&mut config.compact, true, "精简指示条");
    });
    ui.label("精简状态只显示资源标志。悬停查看数值，双击恢复正常；右键或托盘也可切换。");
    ui.add_space(18.0); section(ui, "背景与边框");
    ui.horizontal(|ui| { ui.checkbox(&mut config.background, "显示背景"); ui.checkbox(&mut config.border, "显示边框"); });
    ui.label("两项都关闭时，只保留数字 / 指示标志，不绘制底色和外框。");
    ui.add_space(18.0);
    section(ui, "整体主题色");
    ui.horizontal(|ui| { ui.color_edit_button_srgb(&mut config.color); ui.label("应用于无分级指标及关闭分级后的数值"); });
    ui.checkbox(&mut config.status_colors, "使用绿 / 黄 / 红分级颜色");
    ui.add_space(20.0); section(ui, "背景透明度");
    ui.add_enabled(config.background, egui::Slider::new(&mut config.opacity, 0.15..=1.0).text("不透明度").fixed_decimals(2));
    ui.label("只调整悬浮卡片背景，文字保持不透明。数值越小，越容易看到下面的窗口。");
    ui.add_space(20.0); section(ui, "快捷主题");
    ui.horizontal(|ui| {
        for (name, color) in [("青绿", [100, 210, 195]), ("冰蓝", [113, 177, 250]), ("暖橙", [245, 178, 106]), ("浅紫", [185, 162, 245])] {
            if ui.button(RichText::new(name).color(Color32::from_rgb(color[0], color[1], color[2]))).clicked() { config.color = color; }
        }
    });
}
fn devices(ui: &mut egui::Ui, config: &mut Config, snapshot: &Snapshot) {
    section(ui, "GPU");
    choose(ui, "gpu", &mut config.gpu, &snapshot.gpus, "自动：逐块显示所有硬件显卡");
    ui.add_space(20.0); section(ui, "磁盘卷");
    choose(ui, "disk", &mut config.disk, &snapshot.disks, "自动：第一块本地卷");
    ui.label("以卷（C:、E: 等）统计空间、活动率与读写速度。");
    ui.add_space(20.0); section(ui, "网络接口");
    choose(ui, "network", &mut config.network, &snapshot.networks, "自动：已连接物理网卡合计");
    ui.label("自动模式排除虚拟网卡，避免重复计算；可手动选择 VPN 或虚拟网卡。");
    ui.add_space(20.0);
    ui.label("开启该类监测后才枚举设备。更换设备会重新积累差分和趋势数据。");
}
fn choose(ui: &mut egui::Ui, id: &str, current: &mut String, devices: &[Device], automatic: &str) {
    let label = if current.is_empty() { automatic.to_string() }
        else { devices.iter().find(|d| &d.id == current).map(|d| d.name.clone()).unwrap_or_else(|| format!("{current}（当前未枚举）")) };
    egui::ComboBox::from_id_salt(id).width(490.0).selected_text(label).show_ui(ui, |ui| {
        ui.selectable_value(current, String::new(), automatic);
        for device in devices { ui.selectable_value(current, device.id.clone(), &device.name); }
    });
}
fn general(ui: &mut egui::Ui, config: &mut Config, message: &mut String) {
    section(ui, "刷新间隔");
    ui.horizontal(|ui| { for seconds in [1, 2, 5] { ui.selectable_value(&mut config.seconds, seconds, format!("{seconds} 秒")); } });
    ui.label("建议 2 秒；5 秒进一步减少常驻开销。");
    ui.add_space(20.0); section(ui, "告警");
    ui.checkbox(&mut config.warnings, "连续达到红色阈值 10 秒显示压力标记（各项可独立关闭）");
    ui.add_space(20.0); section(ui, "自动贴边");
    ui.checkbox(&mut config.snap_edges, "靠近屏幕四边 16 像素时自动贴边");
    ui.label("距离随 Windows 缩放调整；支持多显示器，避开任务栏。贴边后切换大小保持位置。");
    ui.add_space(20.0); section(ui, "开机启动");
    let mut autostart = native::autostart();
    if ui.checkbox(&mut autostart, "登录 Windows 时启动").changed() {
        *message = match native::set_autostart(autostart) { Ok(()) => "开机启动设置已更新".into(), Err(e) => e };
    }
    ui.label("首次默认关闭。开启后，请保持可执行文件位置不变。");
    ui.add_space(20.0); section(ui, "鼠标操作");
    ui.label("悬停操作栏浮在数据上方，不占布局空间。拖动指标区域移动。固定只固定位置；鼠标穿透在托盘开启，托盘单击、Ctrl+Alt+M 或重新打开程序可恢复。");
    ui.add_space(20.0); section(ui, "内存清理");
    ui.label("详情面板提供手动清理低优先级待机缓存。不会定时清理，也不会压缩其他程序工作集。此操作不能修复内核池泄漏。");
}
fn section(ui: &mut egui::Ui, title: &str) { ui.label(RichText::new(title).size(17.0).strong()); ui.add_space(8.0); }
