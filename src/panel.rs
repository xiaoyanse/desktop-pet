use crate::{config::{Config, Metric, Row}, model::{Reading, Snapshot}};
use eframe::egui::{self, Color32, RichText};
use std::{collections::{HashMap, VecDeque}, time::{Duration, Instant}};

pub type History = HashMap<String, VecDeque<(Instant, f64)>>;
pub struct DisplayRow { pub key: String, pub label: String, pub reading: Reading, pub row: Row }
pub fn rows(config: &Config, snapshot: &Snapshot) -> Vec<DisplayRow> {
    let mut result = Vec::new();
    for row in config.rows.iter().filter(|r| r.enabled) {
        if row.metric.group() == "GPU" && !snapshot.gpu_values.is_empty() {
            for gpu in &snapshot.gpu_values {
                let index = snapshot.gpus.iter().position(|d| d.id == gpu.device.id).unwrap_or(0);
                let label = match row.metric {
                    Metric::Gpu => format!("GPU {index}"),
                    Metric::Dedicated => format!("GPU {index} 专用"),
                    _ => format!("GPU {index} 共享"),
                };
                result.push(DisplayRow { key: format!("{:?}:{}", row.metric, gpu.device.id), label,
                    reading: gpu.values.get(&row.metric).cloned().unwrap_or_else(|| Reading::missing("显卡采样不可用")), row: row.clone() });
            }
        } else {
            result.push(DisplayRow { key: format!("{:?}", row.metric), label: row.metric.label().into(),
                reading: snapshot.values.get(&row.metric).cloned().unwrap_or_else(|| Reading::missing("等待首个采样")), row: row.clone() });
        }
    }
    result
}
#[derive(Default)]
pub struct Alerts { since: HashMap<String, Instant>, active: HashMap<String, bool> }
impl Alerts {
    pub fn clear(&mut self) { self.since.clear(); self.active.clear(); }
    pub fn sample(&mut self, rows: &[DisplayRow], enabled: bool, now: Instant) {
        self.since.retain(|key, _| rows.iter().any(|r| &r.key == key));
        self.active.clear();
        for item in rows {
            let exceeded = enabled && item.row.alert && item.row.metric.alert_value(&item.reading)
                .is_some_and(|v| item.row.metric.exceeds(v, item.row.threshold));
            if exceeded {
                let start = self.since.entry(item.key.clone()).or_insert(now);
                self.active.insert(item.key.clone(), now.duration_since(*start) >= Duration::from_secs(10));
            } else { self.since.remove(&item.key); }
        }
    }
    pub fn active(&self, key: &str) -> bool { self.active.get(key).copied().unwrap_or(false) }
}
pub fn height(rows: &[DisplayRow], expanded: bool) -> f32 {
    (18.0 + rows.iter().map(|r| 31.0 + if r.row.trend { 27.0 } else { 0.0 }).sum::<f32>()
        + if expanded { 480.0 } else { 0.0 }).max(48.0)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level { Missing, Neutral, Green, Yellow, Red }
pub fn level(item: &DisplayRow) -> Level {
    if item.reading.value.is_none() { return Level::Missing; }
    let Some(value) = item.row.metric.color_value(&item.reading).filter(|v| v.is_finite()) else { return Level::Neutral; };
    let (red, yellow) = if item.row.metric.low_warning() {
        (value <= item.row.threshold, value <= item.row.warning_threshold)
    } else { (value >= item.row.threshold, value >= item.row.warning_threshold) };
    if red { Level::Red } else if yellow { Level::Yellow } else { Level::Green }
}
pub fn tint(level: Level, enabled: bool, theme: Color32) -> Color32 {
    if level == Level::Missing { return Color32::from_rgb(122, 132, 147); }
    if !enabled { return theme; }
    match level {
        Level::Green => Color32::from_rgb(90, 214, 145),
        Level::Yellow => Color32::from_rgb(248, 200, 82),
        Level::Red => Color32::from_rgb(255, 102, 112),
        _ => theme,
    }
}
pub struct Indicator { pub label: String, pub level: Level, pub tooltip: String }
pub fn indicators(rows: &[DisplayRow]) -> Vec<Indicator> {
    let mut groups = Vec::<(String, Indicator)>::new();
    for item in rows {
        let group = if item.row.metric.group() == "GPU" {
            item.key.split_once(':').map(|(_, id)| format!("GPU:{id}")).unwrap_or_else(|| "GPU".into())
        } else { item.row.metric.group().into() };
        let label = if item.row.metric.group() == "GPU" {
            item.label.strip_prefix("GPU ").and_then(|s| s.split_whitespace().next())
                .filter(|s| s.chars().all(|c| c.is_ascii_digit())).map(|n| format!("G{n}")).unwrap_or_else(|| "G".into())
        } else { item.row.metric.short_label().into() };
        let text = format!("{}：{}", item.label, item.reading.text);
        if let Some((_, indicator)) = groups.iter_mut().find(|(key, _)| key == &group) {
            indicator.level = indicator.level.max(level(item));
            indicator.tooltip.push_str(&format!("\n{text}"));
        } else { groups.push((group, Indicator { label, level: level(item), tooltip: text })); }
    }
    groups.into_iter().map(|(_, indicator)| indicator).collect()
}
pub fn compact_size(rows: &[DisplayRow]) -> egui::Vec2 {
    egui::vec2((indicators(rows).len().max(1) as f32 * 26.0 + 12.0).max(42.0), 32.0)
}
pub fn compact(ui: &mut egui::Ui, rows: &[DisplayRow], colors: bool, theme: Color32) {
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    ui.horizontal(|ui| {
        let indicators = indicators(rows);
        if indicators.is_empty() { ui.label("·"); }
        for indicator in indicators {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(26.0, 20.0), egui::Sense::hover());
            let color = tint(indicator.level, colors, theme);
            ui.painter().circle_filled(rect.center(), 9.0, color.gamma_multiply(0.18));
            ui.painter().circle_stroke(rect.center(), 9.0, egui::Stroke::new(1.3_f32, color));
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, indicator.label, egui::FontId::proportional(9.0), color);
        }
    });
}
pub fn metric(ui: &mut egui::Ui, item: &DisplayRow, history: &History, warning: bool, colors: bool, color: Color32) {
    let reading = &item.reading;
    let tint = tint(level(item), colors, color);
    let history = history.get(&item.key);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 22.0), egui::Sense::hover());
    response.on_hover_text(format!("{}\n{}{}", item.row.metric.label(), reading.source,
        if warning { "\n资源压力持续超限" } else { "" }));
    let galley = ui.painter().layout_no_wrap(reading.text.clone(), egui::FontId::proportional(14.0), if reading.value.is_some() { tint } else { Color32::GRAY });
    let available = (rect.width() - galley.size().x - 10.0).max(0.0);
    let mut name_ui = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(rect.min, egui::vec2(available, rect.height()))));
    name_ui.add(egui::Label::new(RichText::new(&item.label).size(12.0).color(Color32::from_rgb(202, 214, 224))).truncate());
    ui.painter().galley(egui::pos2(rect.right() - galley.size().x, rect.center().y - galley.size().y * 0.5), galley, tint);
    if warning { ui.painter().line_segment([rect.left_bottom(), rect.right_bottom()], egui::Stroke::new(1.0_f32, Color32::from_rgb(255, 102, 112))); }
    let peak = history.and_then(|h| h.iter().map(|(_, v)| *v).reduce(f64::max)).unwrap_or(1.0).max(0.001);
    let fraction = reading.fraction.unwrap_or_else(|| (reading.value.unwrap_or(0.0) / peak) as f32).clamp(0.0, 1.0);
    let (bar, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 2.0), egui::Sense::hover());
    response.on_hover_text(if reading.fraction.is_some() { "容量或占用比例" } else { "按最近一分钟峰值缩放，仅表示活动变化" });
    ui.painter().rect_filled(bar, 1, Color32::from_rgb(46, 57, 68));
    if reading.value.is_some() { let mut fill = bar; fill.max.x = fill.min.x + fill.width() * fraction; ui.painter().rect_filled(fill, 1, tint); }
    if item.row.trend { trend(ui, history, item.row.metric.percent(), tint); }
    ui.add_space(3.0);
}
fn trend(ui: &mut egui::Ui, history: Option<&VecDeque<(Instant, f64)>>, percent: bool, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::hover());
    let Some(history) = history.filter(|h| h.len() > 1) else { return; };
    let max = if percent { 100.0 } else { history.iter().map(|(_, v)| *v).reduce(f64::max).unwrap_or(1.0).max(0.001) };
    let now = Instant::now();
    let points = history.iter().map(|(time, value)| egui::pos2(rect.right() - (now.duration_since(*time).as_secs_f32() / 60.0).min(1.0) * rect.width(), rect.bottom() - (*value / max).clamp(0.0, 1.0) as f32 * rect.height())).collect();
    ui.painter().add(egui::Shape::line(points, egui::Stroke::new(1.25_f32, color)));
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Device, GpuReading};
    #[test]
    fn hover_controls_reserve_no_header_and_compact_is_one_line() {
        let mut config = Config::default();
        config.rows.retain(|r| r.metric == Metric::Cpu);
        let rows = rows(&config, &Snapshot::default());
        assert_eq!(height(&rows, false), 49.0);
        assert_eq!(compact_size(&rows).y, 32.0);
    }
    #[test]
    fn color_boundaries_capacity_direction_and_unavailable_are_explicit() {
        let mut item = DisplayRow { key: "CPU".into(), label: "CPU".into(), reading: Reading::percent(69.9, "test"), row: Row::default() };
        assert_eq!(level(&item), Level::Green);
        item.reading = Reading::percent(70.0, "test"); assert_eq!(level(&item), Level::Yellow);
        item.reading = Reading::percent(90.0, "test"); assert_eq!(level(&item), Level::Red);
        item.row.metric = Metric::Available; item.row.threshold = 2.0; item.row.warning_threshold = 4.0;
        item.reading = Reading::gib(3 << 30, None, "test"); assert_eq!(level(&item), Level::Yellow);
        item.reading = Reading::gib(1 << 30, None, "test"); assert_eq!(level(&item), Level::Red);
        item.reading = Reading::missing("test"); assert_eq!(level(&item), Level::Missing);
        item.row.metric = Metric::Upload; item.reading = Reading::rate(100000.0, "test"); assert_eq!(level(&item), Level::Neutral);
    }
    #[test]
    fn two_gpus_have_independent_rows_and_pressure_state() {
        let mut snapshot = Snapshot::default();
        for (id, value) in [("integrated", 4.0), ("discrete", 98.0)] {
            let device = Device { id: id.into(), name: id.into() };
            snapshot.gpus.push(device.clone());
            snapshot.gpu_values.push(GpuReading { device, values: HashMap::from([(Metric::Gpu, Reading::percent(value, id))]) });
        }
        let config = Config::default(); let rows = rows(&config, &snapshot);
        let gpus: Vec<_> = rows.iter().filter(|r| r.row.metric == Metric::Gpu).collect();
        assert_eq!(gpus.len(), 2); assert_ne!(gpus[0].key, gpus[1].key);
        assert_eq!(gpus[0].reading.value, Some(4.0)); assert_eq!(gpus[1].reading.value, Some(98.0));
        assert!(height(&rows, false) < 300.0);
        let mut alerts = Alerts::default(); let start = Instant::now();
        alerts.sample(&rows, true, start); alerts.sample(&rows, true, start + Duration::from_secs(20));
        assert!(!alerts.active(&gpus[1].key));
    }
    #[test]
    fn brief_spikes_and_missing_samples_reset_alerts() {
        let mut snapshot = Snapshot::default();
        snapshot.values.insert(Metric::Memory, Reading::percent(95.0, "test"));
        let mut config = Config::default();
        config.rows.retain(|r| r.metric == Metric::Memory);
        let mut items = rows(&config, &snapshot); let key = items[0].key.clone();
        let start = Instant::now(); let mut alerts = Alerts::default();
        alerts.sample(&items, true, start); assert!(!alerts.active(&key));
        alerts.sample(&items, true, start + Duration::from_secs(10)); assert!(alerts.active(&key));
        items[0].reading = Reading::missing("disconnected");
        alerts.sample(&items, true, start + Duration::from_secs(11)); assert!(!alerts.active(&key));
        items[0].reading = Reading::percent(99.0, "test");
        alerts.sample(&items, true, start + Duration::from_secs(12)); assert!(!alerts.active(&key));
        alerts.sample(&items, false, start + Duration::from_secs(50)); assert!(!alerts.active(&key));
    }
}
