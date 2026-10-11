use super::{panel, pet_motion};
use crate::domain::{
    config::{Config, Metric},
    desktop::PetKind,
    resources::Snapshot,
};
use eframe::egui::{self, Color32, RichText};
use std::time::{Duration, Instant};

pub const SIZE: egui::Vec2 = egui::vec2(72.0, 84.0);
const DATA: [&[u8]; 6] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/pet-0-0.rgba")),
    include_bytes!(concat!(env!("OUT_DIR"), "/pet-1-0.rgba")),
    include_bytes!(concat!(env!("OUT_DIR"), "/pet-2-0.rgba")),
    include_bytes!(concat!(env!("OUT_DIR"), "/pet-0-1.rgba")),
    include_bytes!(concat!(env!("OUT_DIR"), "/pet-1-1.rgba")),
    include_bytes!(concat!(env!("OUT_DIR"), "/pet-2-1.rgba")),
];
#[derive(Default)]
pub struct Sprites {
    textures: [Option<egui::TextureHandle>; 6],
    started: Option<Instant>,
    gaze: egui::Vec2,
    reaction: Option<(Instant, bool)>,
}
impl Sprites {
    pub fn react(&mut self, dropped: bool) {
        self.reaction = Some((Instant::now(), dropped));
    }
    pub fn texture(
        &mut self,
        ctx: &egui::Context,
        kind: PetKind,
        blink: bool,
    ) -> &egui::TextureHandle {
        let index = kind.index() + usize::from(blink) * 3;
        self.textures[index].get_or_insert_with(|| {
            ctx.load_texture(
                format!("pet-{index}"),
                egui::ColorImage::from_rgba_unmultiplied([96, 96], DATA[index]),
                egui::TextureOptions::LINEAR,
            )
        })
    }
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        config: &Config,
        snapshot: &Snapshot,
        cursor: Option<[f32; 2]>,
    ) -> (bool, bool) {
        let hovering = ctx.input(|i| i.pointer.hover_pos().is_some());
        let time = self
            .started
            .get_or_insert_with(Instant::now)
            .elapsed()
            .as_secs_f64();
        let (mut blink, delay) = animation(time, config.pet_animation, hovering);
        if let Some(delay) = delay {
            ctx.request_repaint_after(delay);
        }
        let mut offset = if hovering && config.pet_animation {
            (time * 2.0).sin() as f32 * 0.8 - 1.0
        } else {
            0.0
        };
        let mut tilt = 0.0;
        if config.pet_animation {
            let target = pet_motion::direction(cursor);
            self.gaze += (target - self.gaze) * 0.35;
            let moving = (target - self.gaze).length_sq() > 0.0005;
            ctx.request_repaint_after(Duration::from_millis(if moving || hovering {
                66
            } else {
                200
            }));
            if let Some((at, dropped)) = self.reaction {
                let phase = at.elapsed().as_secs_f32();
                if phase < 0.9 {
                    offset -= (phase * std::f32::consts::PI / 0.9).sin() * 3.0;
                    tilt = (phase * 22.0).sin()
                        * (1.0 - phase / 0.9)
                        * if dropped { 0.10 } else { 0.04 };
                    blink |= !dropped && phase < 0.14;
                    ctx.request_repaint_after(Duration::from_millis(66));
                } else {
                    self.reaction = None;
                }
            }
        } else {
            self.gaze = egui::Vec2::ZERO;
            self.reaction = None;
        }
        let gaze = self.gaze;
        let texture = self.texture(ctx, config.pet_kind, blink).id();
        let mut menu = false;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let response = ui.interact(
                    ui.max_rect(),
                    ui.id().with("pet-menu"),
                    egui::Sense::click(),
                );
                menu = response.secondary_clicked();
                if response.clicked() {
                    self.react(false);
                }
                pet_motion::paint(
                    ui.painter(),
                    texture,
                    config.pet_kind,
                    gaze,
                    offset,
                    tilt,
                    blink,
                );
                let badge =
                    egui::Rect::from_min_size(egui::pos2(5.0, 64.0), egui::vec2(62.0, 18.0));
                ui.painter()
                    .rect_filled(badge, 8, Color32::from_rgb(23, 32, 43));
                let item = snapshot
                    .values
                    .get(&Metric::Memory)
                    .map(|v| ("RAM", v))
                    .or_else(|| snapshot.values.get(&Metric::Cpu).map(|v| ("CPU", v)));
                let text = item
                    .map(|(label, v)| {
                        format!(
                            "{label} {}",
                            v.value
                                .map(|n| format!("{n:.0}%"))
                                .unwrap_or_else(|| "—".into())
                        )
                    })
                    .unwrap_or_else(|| "资源助手".into());
                let rows = panel::rows(config, snapshot);
                let color = rows
                    .iter()
                    .find(|row| {
                        row.row.metric
                            == item
                                .map(|(name, _)| {
                                    if name == "RAM" {
                                        Metric::Memory
                                    } else {
                                        Metric::Cpu
                                    }
                                })
                                .unwrap_or(Metric::Memory)
                    })
                    .map(|row| {
                        panel::tint(
                            panel::level(row),
                            config.status_colors,
                            Color32::from_rgb(config.color[0], config.color[1], config.color[2]),
                        )
                    })
                    .unwrap_or(Color32::LIGHT_GRAY);
                ui.painter().text(
                    badge.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(10.0),
                    color,
                );
            });
        (hovering, menu)
    }
    pub fn selection(&mut self, ui: &mut egui::Ui, config: &mut Config) {
        ui.horizontal(|ui| {
            for kind in PetKind::ALL {
                ui.vertical(|ui| {
                    let texture = self.texture(ui.ctx(), kind, false);
                    let response = ui.add(
                        egui::Image::new(texture)
                            .fit_to_exact_size(egui::vec2(60.0, 60.0))
                            .sense(egui::Sense::click()),
                    );
                    if response.clicked() {
                        config.pet_kind = kind;
                    }
                    ui.label(
                        RichText::new(kind.label()).color(if config.pet_kind == kind {
                            Color32::from_rgb(100, 210, 195)
                        } else {
                            Color32::LIGHT_GRAY
                        }),
                    );
                    if config.pet_kind == kind {
                        ui.painter().rect_stroke(
                            response.rect,
                            8,
                            egui::Stroke::new(1.0_f32, Color32::from_rgb(100, 210, 195)),
                            egui::StrokeKind::Inside,
                        );
                    }
                });
                ui.add_space(22.0);
            }
        });
        ui.checkbox(
            &mut config.pet_animation,
            "鼠标跟随、眨眼、悬停抬头、点击回应及拖放摇摆",
        );
        ui.weak("头和眼睛跟随全屏鼠标；悬停显示快捷操作，双击或托盘恢复完整卡片。移动动画最多 15 帧/秒；关闭动画后停止鼠标轮询。");
    }
}
fn animation(seconds: f64, enabled: bool, hovering: bool) -> (bool, Option<Duration>) {
    if !enabled {
        return (false, None);
    }
    let phase = seconds.rem_euclid(5.0);
    let blink = phase >= 4.82;
    let delay = if hovering {
        0.1
    } else if blink {
        5.0 - phase
    } else {
        4.82 - phase
    };
    (blink, Some(Duration::from_secs_f64(delay.max(0.02))))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idle_animation_is_scheduled_at_transitions_and_can_be_disabled() {
        assert_eq!(animation(1.0, false, true), (false, None));
        assert_eq!(
            animation(0.0, true, false),
            (false, Some(Duration::from_secs_f64(4.82)))
        );
        assert!(animation(4.9, true, false).0);
        assert!(!animation(5.1, true, false).0);
        assert_eq!(
            animation(1.0, true, true).1.unwrap(),
            Duration::from_millis(100)
        );
        assert!(DATA.iter().all(|data| data.len() == 96 * 96 * 4));
    }
}
