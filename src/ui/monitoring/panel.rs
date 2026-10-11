use crate::domain::{
    config::{Config, Metric, Row},
    resources::{Reading, Snapshot},
};
use eframe::egui::{self, Color32, RichText};
use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};

pub type History = HashMap<String, VecDeque<(Instant, f64)>>;
pub struct DisplayRow {
    pub key: String,
    pub label: String,
    pub reading: Reading,
    pub row: Row,
}
pub fn rows(config: &Config, snapshot: &Snapshot) -> Vec<DisplayRow> {
    let mut result = Vec::new();
    for row in config.rows.iter().filter(|r| r.enabled) {
        if row.metric.group() == "GPU" && !snapshot.gpu_values.is_empty() {
            for gpu in &snapshot.gpu_values {
                let index = snapshot
                    .gpus
                    .iter()
                    .position(|d| d.id == gpu.device.id)
                    .unwrap_or(0);
                let label = match row.metric {
                    Metric::Gpu => format!("GPU {index}"),
                    Metric::Dedicated => format!("GPU {index} 专用"),
                    _ => format!("GPU {index} 共享"),
                };
                result.push(DisplayRow {
                    key: format!("{:?}:{}", row.metric, gpu.device.id),
                    label,
                    reading: gpu
                        .values
                        .get(&row.metric)
                        .cloned()
                        .unwrap_or_else(|| Reading::missing("显卡采样不可用")),
                    row: row.clone(),
                });
            }
        } else {
            result.push(DisplayRow {
                key: format!("{:?}", row.metric),
                label: row.metric.label().into(),
                reading: snapshot
                    .values
                    .get(&row.metric)
                    .cloned()
                    .unwrap_or_else(|| Reading::missing("等待首个采样")),
                row: row.clone(),
            });
        }
    }
    result
}
#[derive(Default)]
pub struct Alerts {
    since: HashMap<String, Instant>,
    active: HashMap<String, bool>,
}
impl Alerts {
    pub fn clear(&mut self) {
        self.since.clear();
        self.active.clear();
    }
    pub fn sample(&mut self, rows: &[DisplayRow], enabled: bool, now: Instant) {
        self.since
            .retain(|key, _| rows.iter().any(|r| &r.key == key));
        self.active.clear();
        for item in rows {
            let exceeded = enabled
                && item.row.alert
                && item
                    .row
                    .metric
                    .alert_value(&item.reading)
                    .is_some_and(|v| item.row.metric.exceeds(v, item.row.threshold));
            if exceeded {
                let start = self.since.entry(item.key.clone()).or_insert(now);
                self.active.insert(
                    item.key.clone(),
                    now.duration_since(*start) >= Duration::from_secs(10),
                );
            } else {
                self.since.remove(&item.key);
            }
        }
    }
    pub fn active(&self, key: &str) -> bool {
        self.active.get(key).copied().unwrap_or(false)
    }
}
pub fn height(rows: &[DisplayRow], expanded: bool) -> f32 {
    (18.0
        + rows
            .iter()
            .map(|r| 31.0 + if r.row.trend { 27.0 } else { 0.0 })
            .sum::<f32>()
        + if expanded { 480.0 } else { 0.0 })
    .max(48.0)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Missing,
    Neutral,
    Green,
    Yellow,
    Red,
}
pub fn level(item: &DisplayRow) -> Level {
    if item.reading.value.is_none() {
        return Level::Missing;
    }
    let Some(value) = item
        .row
        .metric
        .color_value(&item.reading)
        .filter(|v| v.is_finite())
    else {
        return Level::Neutral;
    };
    let (red, yellow) = if item.row.metric.low_warning() {
        (
            value <= item.row.threshold,
            value <= item.row.warning_threshold,
        )
    } else {
        (
            value >= item.row.threshold,
            value >= item.row.warning_threshold,
        )
    };
    if red {
        Level::Red
    } else if yellow {
        Level::Yellow
    } else {
        Level::Green
    }
}
pub fn tint(level: Level, enabled: bool, theme: Color32) -> Color32 {
    if level == Level::Missing {
        return Color32::from_rgb(122, 132, 147);
    }
    if !enabled {
        return theme;
    }
    match level {
        Level::Green => Color32::from_rgb(90, 214, 145),
        Level::Yellow => Color32::from_rgb(248, 200, 82),
        Level::Red => Color32::from_rgb(255, 102, 112),
        _ => theme,
    }
}
pub struct Indicator {
    pub tooltip: String,
}
pub fn indicators(rows: &[DisplayRow]) -> Vec<Indicator> {
    let mut groups = Vec::<(String, Indicator)>::new();
    for item in rows {
        let group = if item.row.metric.group() == "GPU" {
            item.key
                .split_once(':')
                .map(|(_, id)| format!("GPU:{id}"))
                .unwrap_or_else(|| "GPU".into())
        } else {
            item.row.metric.group().into()
        };
        let text = format!("{}：{}", item.label, item.reading.text);
        if let Some((_, indicator)) = groups.iter_mut().find(|(key, _)| key == &group) {
            indicator.tooltip.push_str(&format!("\n{text}"));
        } else {
            groups.push((group, Indicator { tooltip: text }));
        }
    }
    groups.into_iter().map(|(_, indicator)| indicator).collect()
}
pub fn compact_size(rows: &[DisplayRow]) -> egui::Vec2 {
    egui::vec2(
        240.0,
        (rows.len().max(1).div_ceil(2) as f32 * 25.0 + 12.0).max(42.0),
    )
}
pub fn compact(ui: &mut egui::Ui, rows: &[DisplayRow], colors: bool, theme: Color32) {
    ui.spacing_mut().item_spacing = egui::vec2(4.0, 1.0);
    if rows.is_empty() {
        ui.label("在设置中选择监测项");
    }
    for pair in rows.chunks(2) {
        ui.horizontal(|ui| {
            for item in pair {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(111.0, 24.0), egui::Sense::hover());
                let color = tint(level(item), colors, theme);
                let label = compact_label(item);
                ui.painter().text(
                    rect.left_center(),
                    egui::Align2::LEFT_CENTER,
                    label,
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(175, 193, 204),
                );
                ui.painter().text(
                    rect.right_center(),
                    egui::Align2::RIGHT_CENTER,
                    compact_value(item),
                    egui::FontId::monospace(11.0),
                    color,
                );
            }
        });
    }
}
fn compact_label(item: &DisplayRow) -> String {
    match item.row.metric {
        Metric::Cpu => "CPU".into(),
        Metric::Memory => "内存".into(),
        Metric::Gpu => item.label.clone(),
        Metric::DiskRead => "盘读".into(),
        Metric::DiskWrite => "盘写".into(),
        Metric::Upload => "上传".into(),
        Metric::Download => "下载".into(),
        Metric::Dedicated => item.label.replace(" 专用", "显存"),
        Metric::Shared => item.label.replace(" 共享", "共享"),
        _ => item.label.clone(),
    }
}
fn compact_value(item: &DisplayRow) -> String {
    let Some(value) = item.reading.value else {
        return "—".into();
    };
    if item.row.metric.percent() {
        format!("{value:.0}%")
    } else {
        item.reading
            .text
            .replace(" GB", "G")
            .replace(" GHz", "Ghz")
            .replace(" KB/s", "K/s")
            .replace(" MB/s", "M/s")
    }
}
pub fn metric(
    ui: &mut egui::Ui,
    item: &DisplayRow,
    history: &History,
    warning: bool,
    colors: bool,
    color: Color32,
) {
    let reading = &item.reading;
    let tint = tint(level(item), colors, color);
    let history = history.get(&item.key);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 22.0), egui::Sense::hover());
    let galley = ui.painter().layout_no_wrap(
        reading.text.clone(),
        egui::FontId::proportional(14.0),
        if reading.value.is_some() {
            tint
        } else {
            Color32::GRAY
        },
    );
    let available = (rect.width() - galley.size().x - 10.0).max(0.0);
    let mut name_ui = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
        rect.min + egui::vec2(if warning { 8.0 } else { 0.0 }, 0.0),
        egui::vec2(
            (available - if warning { 8.0 } else { 0.0 }).max(0.0),
            rect.height(),
        ),
    )));
    name_ui.add(
        egui::Label::new(
            RichText::new(&item.label)
                .size(12.0)
                .color(Color32::from_rgb(202, 214, 224)),
        )
        .truncate(),
    );
    ui.painter().galley(
        egui::pos2(
            rect.right() - galley.size().x,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        tint,
    );
    if warning {
        ui.painter().circle_filled(
            egui::pos2(rect.left() + 2.0, rect.center().y),
            2.0,
            Color32::from_rgb(255, 102, 112),
        );
    }
    let peak = history
        .and_then(|h| h.iter().map(|(_, v)| *v).reduce(f64::max))
        .unwrap_or(1.0)
        .max(0.001);
    let fraction = reading
        .fraction
        .unwrap_or_else(|| (reading.value.unwrap_or(0.0) / peak) as f32)
        .clamp(0.0, 1.0);
    let (bar, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 2.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(bar, 1, Color32::from_rgb(46, 57, 68));
    if reading.value.is_some() {
        let mut fill = bar;
        fill.max.x = fill.min.x + fill.width() * fraction;
        ui.painter().rect_filled(fill, 1, tint);
    }
    if item.row.trend {
        super::trend::show(ui, history, item.row.metric.percent(), tint);
    }
    ui.add_space(3.0);
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::resources::{Device, GpuReading};
    #[test]
    fn sustained_pressure_paints_one_progress_bar_and_a_dot() {
        let ctx = egui::Context::default();
        let item = DisplayRow {
            key: "Memory".into(),
            label: "RAM".into(),
            reading: Reading::percent(95.0, "test"),
            row: Row {
                metric: Metric::Memory,
                ..Default::default()
            },
        };
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(240.0, 142.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    metric(ui, &item, &History::new(), true, true, Color32::WHITE)
                });
            },
        );
        let red = tint(Level::Red, true, Color32::WHITE);
        assert_eq!(
            output
                .shapes
                .iter()
                .filter(|s| matches!(&s.shape,egui::Shape::Rect(r) if r.fill==red))
                .count(),
            1
        );
        assert_eq!(
            output
                .shapes
                .iter()
                .filter(|s| matches!(&s.shape,egui::Shape::Circle(c) if c.fill==red))
                .count(),
            1
        );
        assert!(
            !output.shapes.iter().any(
                |s| matches!(&s.shape,egui::Shape::LineSegment {stroke,..} if stroke.color==red)
            )
        );
    }
    #[test]
    fn hover_controls_reserve_no_header_and_compact_retains_names_and_numbers() {
        let mut config = Config::default();
        config.rows.retain(|r| r.metric == Metric::Cpu);
        let rows = rows(&config, &Snapshot::default());
        assert_eq!(height(&rows, false), 49.0);
        assert_eq!(compact_size(&rows).y, 42.0);
        assert_eq!(compact_label(&rows[0]), "CPU");
        let mut item = DisplayRow {
            key: "CPU".into(),
            label: "CPU 占用".into(),
            reading: Reading::percent(42.0, "test"),
            row: Row::default(),
        };
        assert_eq!(compact_value(&item), "42%");
        item.reading = Reading::missing("test");
        assert_eq!(compact_value(&item), "—");
    }
    #[test]
    fn color_boundaries_capacity_direction_and_unavailable_are_explicit() {
        let mut item = DisplayRow {
            key: "CPU".into(),
            label: "CPU".into(),
            reading: Reading::percent(69.9, "test"),
            row: Row::default(),
        };
        assert_eq!(level(&item), Level::Green);
        item.reading = Reading::percent(70.0, "test");
        assert_eq!(level(&item), Level::Yellow);
        item.reading = Reading::percent(90.0, "test");
        assert_eq!(level(&item), Level::Red);
        item.row.metric = Metric::Available;
        item.row.threshold = 2.0;
        item.row.warning_threshold = 4.0;
        item.reading = Reading::gib(3 << 30, None, "test");
        assert_eq!(level(&item), Level::Yellow);
        item.reading = Reading::gib(1 << 30, None, "test");
        assert_eq!(level(&item), Level::Red);
        item.reading = Reading::missing("test");
        assert_eq!(level(&item), Level::Missing);
        item.row.metric = Metric::Upload;
        item.reading = Reading::rate(100000.0, "test");
        assert_eq!(level(&item), Level::Neutral);
    }
    #[test]
    fn two_gpus_have_independent_rows_and_pressure_state() {
        let mut snapshot = Snapshot::default();
        for (id, value) in [("integrated", 4.0), ("discrete", 98.0)] {
            let device = Device {
                id: id.into(),
                name: id.into(),
            };
            snapshot.gpus.push(device.clone());
            snapshot.gpu_values.push(GpuReading {
                device,
                values: HashMap::from([(Metric::Gpu, Reading::percent(value, id))]),
            });
        }
        let config = Config::default();
        let rows = rows(&config, &snapshot);
        let gpus: Vec<_> = rows
            .iter()
            .filter(|r| r.row.metric == Metric::Gpu)
            .collect();
        assert_eq!(gpus.len(), 2);
        assert_ne!(gpus[0].key, gpus[1].key);
        assert_eq!(gpus[0].reading.value, Some(4.0));
        assert_eq!(gpus[1].reading.value, Some(98.0));
        assert!(height(&rows, false) < 300.0);
        let mut alerts = Alerts::default();
        let start = Instant::now();
        alerts.sample(&rows, true, start);
        alerts.sample(&rows, true, start + Duration::from_secs(20));
        assert!(!alerts.active(&gpus[1].key));
    }
    #[test]
    fn brief_spikes_and_missing_samples_reset_alerts() {
        let mut snapshot = Snapshot::default();
        snapshot
            .values
            .insert(Metric::Memory, Reading::percent(95.0, "test"));
        let mut config = Config::default();
        config.rows.retain(|r| r.metric == Metric::Memory);
        let mut items = rows(&config, &snapshot);
        let key = items[0].key.clone();
        let start = Instant::now();
        let mut alerts = Alerts::default();
        alerts.sample(&items, true, start);
        assert!(!alerts.active(&key));
        alerts.sample(&items, true, start + Duration::from_secs(10));
        assert!(alerts.active(&key));
        items[0].reading = Reading::missing("disconnected");
        alerts.sample(&items, true, start + Duration::from_secs(11));
        assert!(!alerts.active(&key));
        items[0].reading = Reading::percent(99.0, "test");
        alerts.sample(&items, true, start + Duration::from_secs(12));
        assert!(!alerts.active(&key));
        alerts.sample(&items, false, start + Duration::from_secs(50));
        assert!(!alerts.active(&key));
    }
}
