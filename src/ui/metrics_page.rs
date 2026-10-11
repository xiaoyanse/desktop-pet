use super::{
    icons::{self, Icon},
    theme, widgets,
};
use crate::domain::config::{Config, Metric};
use eframe::egui::{self, Color32, RichText};
pub fn show(
    ui: &mut egui::Ui,
    config: &mut Config,
    editing: &mut Option<Metric>,
    filter: &mut Option<&'static str>,
) {
    ui.horizontal(|ui| {
        let width = (ui.available_width() - ui.spacing().item_spacing.x * 5.0) / 6.0;
        for (name, group) in [
            ("全部", None),
            ("CPU", Some("CPU")),
            ("内存", Some("内存")),
            ("GPU", Some("GPU")),
            ("磁盘", Some("磁盘")),
            ("网络", Some("网络")),
        ] {
            if widgets::tab(ui, name, *filter == group, width).clicked() {
                *filter = group;
            }
        }
    });
    ui.add_space(12.0);
    ui.label(
        RichText::new("绿色 < 黄色阈值 · 黄色 < 红色阈值 · 红色 ≥ 红色阈值")
            .size(12.0)
            .color(Color32::from_rgb(141, 188, 181)),
    );
    ui.add_space(14.0);
    let mut movement = None;
    let visible: Vec<usize> = config
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| filter.is_none_or(|group| row.metric.group() == group))
        .map(|(index, _)| index)
        .collect();
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.add_sized(
            [154.0, 20.0],
            egui::Label::new(RichText::new("监测项 / 启用").small().color(theme::MUTED)),
        );
        ui.add_sized(
            [78.0, 20.0],
            egui::Label::new(RichText::new("趋势").small().color(theme::MUTED)),
        );
        ui.label(RichText::new("颜色阈值").small().color(theme::MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new("排序").small().color(theme::MUTED));
        });
    });
    for (index, row) in config.rows.iter_mut().enumerate() {
        let Some(position) = visible.iter().position(|i| *i == index) else {
            continue;
        };
        ui.push_id(row.metric, |ui| {
            egui::Frame::new()
                .fill(theme::SURFACE)
                .corner_radius(8)
                .inner_margin(10)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.allocate_ui(egui::vec2(154.0, 30.0), |ui| {
                            widgets::switch(ui, &mut row.enabled, row.metric.label());
                        });
                        widgets::switch(ui, &mut row.trend, "趋势");
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
                                .add_enabled_ui(position + 1 < visible.len(), |ui| {
                                    icons::button(ui, Icon::Down, "下移")
                                })
                                .inner
                                .clicked()
                            {
                                movement = Some((index, visible[position + 1]));
                            }
                            if ui
                                .add_enabled_ui(position > 0, |ui| {
                                    icons::button(ui, Icon::Up, "上移")
                                })
                                .inner
                                .clicked()
                            {
                                movement = Some((index, visible[position - 1]));
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
                        ui.colored_label(Color32::from_rgb(248, 200, 82), "黄色 · 开始提醒");
                        widgets::number(
                            ui,
                            "提醒阈值",
                            &mut row.warning_threshold,
                            0.0..=max,
                            suffix,
                        );
                        ui.add_space(8.0);
                        ui.colored_label(Color32::from_rgb(255, 102, 112), "红色 · 高压力");
                        widgets::number(ui, "高压阈值", &mut row.threshold, 0.0..=max, suffix);
                        if row.metric.low_warning() {
                            ui.weak("剩余容量越低压力越高：低于阈值变色。");
                        }
                        if row.metric.alert_supported() {
                            widgets::switch(ui, &mut row.alert, "连续超限 10 秒额外显示压力标记");
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn group_tab_filters_visible_rows_without_changing_monitoring_configuration() {
        let ctx = egui::Context::default();
        let mut config = Config::default();
        let before = serde_json::to_string(&config).unwrap();
        let mut editing = None;
        let mut filter = None;
        let render = |events,
                      config: &mut Config,
                      filter: &mut Option<&'static str>,
                      editing: &mut Option<Metric>| {
            ctx.run(
                egui::RawInput {
                    events,
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 1100.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| show(ui, config, editing, filter));
                },
            )
        };
        let output = render(Vec::new(), &mut config, &mut filter, &mut editing);
        let point = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == "内存" => {
                    Some(t.pos + t.galley.size() * 0.5)
                }
                _ => None,
            })
            .unwrap();
        for pressed in [true, false] {
            render(
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                &mut config,
                &mut filter,
                &mut editing,
            );
        }
        assert_eq!(filter, Some("内存"));
        assert_eq!(serde_json::to_string(&config).unwrap(), before);
        let output = render(Vec::new(), &mut config, &mut filter, &mut editing);
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some(t.galley.text()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"内存占用"));
        assert!(!texts.contains(&"CPU 占用"));
        assert!(!texts.contains(&"磁盘读取"));
    }
}
