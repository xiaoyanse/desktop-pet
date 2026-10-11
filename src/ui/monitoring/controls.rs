use crate::ui::common::icons::{self, Icon};
use eframe::egui::{self, Color32};

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Settings,
    Lock,
    Details,
    ToggleMode,
    Cleanup,
    Pet,
}

pub fn toolbar(
    ctx: &egui::Context,
    compact: bool,
    locked: bool,
    expanded: bool,
) -> (Option<Action>, Option<&'static str>) {
    let visible = ctx.input(|i| i.pointer.hover_pos().is_some());
    let mut action = None;
    let mut tip = None;
    egui::Area::new(egui::Id::new("hover-controls"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(3.0, 3.0))
        .movable(false)
        .sense(egui::Sense::hover())
        .show(ctx, |ui| {
            if !visible {
                ui.set_opacity(0.0);
            }
            egui::Frame::new()
                .fill(Color32::from_rgb(28, 37, 48))
                .corner_radius(6)
                .inner_margin(4)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(3.0, 0.0);
                    ui.horizontal(|ui| {
                        let mut icon = |ui: &mut egui::Ui, icon, help: &'static str, target| {
                            let response = icons::bare_button(ui, icon, help);
                            if response.hovered() {
                                tip = Some(help);
                            }
                            if response.clicked() {
                                action = Some(target);
                            }
                        };
                        icon(ui, Icon::Settings, "打开设置", Action::Settings);
                        icon(
                            ui,
                            if locked { Icon::Unpin } else { Icon::Pin },
                            if locked {
                                "解锁位置"
                            } else {
                                "固定位置"
                            },
                            Action::Lock,
                        );
                        if !compact {
                            icon(
                                ui,
                                Icon::Details,
                                if expanded {
                                    "收起进程详情"
                                } else {
                                    "展开进程详情"
                                },
                                Action::Details,
                            );
                        }
                        icon(
                            ui,
                            if compact {
                                Icon::Restore
                            } else {
                                Icon::Compact
                            },
                            if compact {
                                "恢复完整视图"
                            } else {
                                "切换精简视图，保留名称和数值"
                            },
                            Action::ToggleMode,
                        );
                        icon(ui, Icon::Clean, "快速清理低优先级待机缓存", Action::Cleanup);
                        icon(ui, Icon::Pet, "切换桌面宠物", Action::Pet);
                    });
                });
        });
    (action, tip)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(ctx: &egui::Context, events: Vec<egui::Event>, compact: bool) -> Option<Action> {
        let mut action = None;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(240.0, 112.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.interact(
                        ui.max_rect(),
                        ui.id().with("move-card"),
                        egui::Sense::click_and_drag(),
                    );
                });
                action = toolbar(ctx, compact, false, false).0;
            },
        );
        action
    }
    #[test]
    fn hidden_toolbar_keeps_hit_targets_and_mode_icon_works_in_both_modes() {
        for compact in [true, false] {
            let ctx = egui::Context::default();
            frame(&ctx, vec![], compact);
            frame(&ctx, vec![], compact);
            let point = egui::pos2(if compact { 78.0 } else { 107.0 }, 20.0);
            frame(&ctx, vec![egui::Event::PointerMoved(point)], compact);
            frame(
                &ctx,
                vec![egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                }],
                compact,
            );
            assert_eq!(
                frame(
                    &ctx,
                    vec![egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: Default::default()
                    }],
                    compact
                ),
                Some(Action::ToggleMode)
            );
        }
    }
    #[test]
    fn body_drag_remains_available_under_scrollable_metrics() {
        let ctx = egui::Context::default();
        let frame = |events| {
            let mut dragging = false;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(240.0, 142.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let response = ui.interact(
                            ui.max_rect(),
                            ui.id().with("move-card"),
                            egui::Sense::click_and_drag(),
                        );
                        dragging = response.drag_started();
                        egui::ScrollArea::vertical()
                            .drag_to_scroll(false)
                            .show(ui, |ui| {
                                for _ in 0..4 {
                                    ui.label("资源名称 90%");
                                }
                            });
                    });
                    toolbar(ctx, false, false, false);
                },
            );
            dragging
        };
        frame(vec![]);
        frame(vec![]);
        let pos = egui::pos2(150.0, 80.0);
        frame(vec![egui::Event::PointerMoved(pos)]);
        frame(vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        }]);
        assert!(frame(vec![egui::Event::PointerMoved(egui::pos2(
            180.0, 80.0
        ))]));
    }
}
