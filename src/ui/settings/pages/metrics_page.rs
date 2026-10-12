use crate::domain::config::{Config, Metric};
use crate::ui::common::{
    icons::{self, Icon},
    theme, widgets,
};
use eframe::egui::{self, Color32, RichText};

pub fn show(
    ui: &mut egui::Ui,
    config: &mut Config,
    editing: &mut Option<Metric>,
    filter: &mut Option<&'static str>,
) {
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(18, 31, 44, 80))
        .corner_radius(12)
        .inner_margin(5)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let width = (ui.available_width() - 20.0) / 6.0;
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
        });
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        for (label, color) in [
            ("正常", Color32::from_rgb(91, 210, 153)),
            ("注意", Color32::from_rgb(248, 200, 82)),
            ("高压", Color32::from_rgb(255, 102, 112)),
        ] {
            let (r, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
            ui.painter().circle_filled(r.center(), 3.0, color);
            ui.label(RichText::new(label).size(12.0).color(theme::MUTED));
            ui.add_space(10.0);
        }
        ui.label(
            RichText::new("按各项阈值分级")
                .size(12.0)
                .color(theme::MUTED),
        );
    });
    ui.add_space(14.0);
    theme::glass_panel(ui, |ui| list(ui, config, editing, *filter));
    ui.add_space(12.0);
    ui.label(
        RichText::new("可用容量越低压力越高；速度与内核池大小保持主题色。")
            .size(12.0)
            .color(theme::MUTED),
    );
}

fn columns(width: f32) -> [f32; 4] {
    let name = (width * 0.31).clamp(154.0, 192.0);
    [name, 88.0, (width - name - 88.0 - 72.0).max(104.0), 72.0]
}
fn cell(ui: &mut egui::Ui, rect: egui::Rect, widths: &[f32; 4], index: usize) -> egui::Ui {
    ui.new_child(
        egui::UiBuilder::new()
            .id_salt(("metric-column", index))
            .max_rect(egui::Rect::from_min_size(
                rect.min + egui::vec2(widths[..index].iter().sum(), 8.0),
                egui::vec2(widths[index], 36.0),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    )
}
fn list(
    ui: &mut egui::Ui,
    config: &mut Config,
    editing: &mut Option<Metric>,
    filter: Option<&'static str>,
) {
    ui.set_width(ui.available_width());
    let widths = columns(ui.available_width());
    let (header, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 32.0), egui::Sense::hover());
    for (i, label) in ["监测项", "趋势", "颜色阈值", "排序"].iter().enumerate() {
        let x = header.left() + widths[..i].iter().sum::<f32>() + if i == 0 { 44.0 } else { 0.0 };
        ui.painter().text(
            egui::pos2(x, header.center().y),
            egui::Align2::LEFT_CENTER,
            *label,
            egui::FontId::proportional(12.0),
            theme::MUTED,
        );
    }
    let visible: Vec<usize> = config
        .rows
        .iter()
        .enumerate()
        .filter(|(_, r)| filter.is_none_or(|g| r.metric.group() == g))
        .map(|(i, _)| i)
        .collect();
    let mut movement = None;
    for (index, row) in config.rows.iter_mut().enumerate() {
        let Some(position) = visible.iter().position(|i| *i == index) else {
            continue;
        };
        ui.push_id(row.metric, |ui| {
            let (rect, _) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 52.0), egui::Sense::hover());
            ui.painter().line_segment(
                [rect.left_top(), rect.right_top()],
                egui::Stroke::new(1.0_f32, theme::LINE),
            );
            if ui.rect_contains_pointer(rect) {
                ui.painter()
                    .rect_filled(rect, 8, Color32::from_white_alpha(7));
            }
            widgets::switch(
                &mut cell(ui, rect, &widths, 0),
                &mut row.enabled,
                row.metric.label(),
            );
            widgets::switch(&mut cell(ui, rect, &widths, 1), &mut row.trend, "")
                .on_hover_text(format!("{}的最近一分钟趋势", row.metric.label()));
            if row.metric.color_supported() {
                let mut controls = cell(ui, rect, &widths, 2);
                controls.label(
                    RichText::new(format!(
                        "{} / {}{}",
                        row.warning_threshold.round(),
                        row.threshold.round(),
                        if row.metric.color_percent() {
                            "%"
                        } else {
                            " GB"
                        }
                    ))
                    .size(12.0)
                    .color(theme::MUTED),
                );
                if icons::button(&mut controls, Icon::General, "设置颜色阈值及持续告警").clicked()
                {
                    *editing = if *editing == Some(row.metric) {
                        None
                    } else {
                        Some(row.metric)
                    };
                }
            }
            let mut sort = cell(ui, rect, &widths, 3);
            for (icon, label, target) in [
                (Icon::Up, "上移", position.checked_sub(1)),
                (
                    Icon::Down,
                    "下移",
                    (position + 1 < visible.len()).then_some(position + 1),
                ),
            ] {
                if sort
                    .add_enabled_ui(target.is_some(), |ui| icons::button(ui, icon, label))
                    .inner
                    .clicked()
                {
                    movement = Some((index, visible[target.unwrap()]));
                }
            }
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
                ui.add_space(10.0);
            }
        });
    }
    if let Some((a, b)) = movement {
        config.rows.swap(a, b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn headings_and_actual_widgets_share_columns_in_a_narrow_settings_page() {
        let ctx = egui::Context::default();
        let mut config = Config::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(520.0, 1100.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .show(ctx, |ui| show(ui, &mut config, &mut None, &mut None));
            },
        );
        let position = |label: &str| {
            output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == label => Some(t.pos.x),
                    _ => None,
                })
                .unwrap()
        };
        assert!((position("监测项") - position("CPU 占用")).abs() < 0.1);
        assert!((position("颜色阈值") - position("70 / 90%")).abs() < 0.1);
        let up = super::super::super::super::common::artwork::texture(&ctx, Icon::Up.asset()).id();
        let glyph = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Mesh(m) if m.texture_id == up => Some((m.calc_bounds(), s.clip_rect)),
                _ => None,
            })
            .unwrap();
        assert!(glyph.1.contains_rect(glyph.0));
        assert!((glyph.0.center().x - position("排序") - 13.0).abs() < 0.1);
    }
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
