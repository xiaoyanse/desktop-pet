use crate::ui::common::{
    icons::{self, Icon},
    theme,
};
use eframe::egui::{self, Color32, ResizeDirection as Direction};

pub fn show(ctx: &egui::Context) -> bool {
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    let mut close = false;
    egui::TopBottomPanel::top("settings-titlebar")
        .show_separator_line(false)
        .exact_height(36.0)
        .frame(egui::Frame::new().stroke(egui::Stroke::NONE))
        .show(ctx, |ui| {
            let (rect, _) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 36.0), egui::Sense::hover());
            let drag_rect = egui::Rect::from_min_max(
                rect.min + egui::vec2(6.0, 6.0),
                egui::pos2(rect.right() - 138.0, rect.bottom()),
            );
            let drag = ui.interact(
                drag_rect,
                ui.id().with("title-drag"),
                egui::Sense::click_and_drag(),
            );
            if drag.double_clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            } else if ui.input(|i| {
                i.pointer.button_pressed(egui::PointerButton::Primary)
                    && i.pointer
                        .press_origin()
                        .is_some_and(|p| drag_rect.contains(p))
            }) {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            for (index, label) in ["关闭设置", "最大化或还原", "最小化设置"]
                .iter()
                .enumerate()
            {
                let button = egui::Rect::from_min_size(
                    egui::pos2(
                        rect.right() - 46.0 * (index + 1) as f32 - 6.0,
                        rect.top() + 3.0,
                    ),
                    egui::vec2(40.0, 30.0),
                );
                let response = ui.interact(button, ui.id().with(label), egui::Sense::click());
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, *label)
                });
                ui.painter().circle_filled(
                    button.center(),
                    15.0,
                    if response.hovered() || response.has_focus() {
                        if index == 0 {
                            Color32::from_rgb(108, 57, 64)
                        } else {
                            theme::HOVER
                        }
                    } else {
                        Color32::from_rgba_unmultiplied(16, 28, 41, 160)
                    },
                );
                ui.painter().circle_stroke(
                    button.center(),
                    15.0,
                    egui::Stroke::new(1.0_f32, theme::LINE),
                );
                let glyph = egui::Rect::from_center_size(button.center(), egui::vec2(18.0, 18.0));
                let icon = match index {
                    0 => Icon::Cancel,
                    1 if maximized => Icon::Restore,
                    1 => Icon::Maximize,
                    _ => Icon::Minimize,
                };
                icons::draw(ui.painter(), glyph, icon, Color32::WHITE);
                if response.clicked() {
                    match index {
                        0 => close = true,
                        1 => ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized)),
                        _ => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                    }
                }
                response.on_hover_text(*label);
            }
        });
    close
}

fn edges(rect: egui::Rect) -> [(egui::Rect, Direction); 8] {
    let [x0, x1, y0, y1] = [rect.left(), rect.right(), rect.top(), rect.bottom()];
    let region = |a, b, c, d| egui::Rect::from_min_max(egui::pos2(a, b), egui::pos2(c, d));
    [
        (region(x0, y0, x0 + 10.0, y0 + 10.0), Direction::NorthWest),
        (region(x1 - 10.0, y0, x1, y0 + 10.0), Direction::NorthEast),
        (region(x0, y1 - 10.0, x0 + 10.0, y1), Direction::SouthWest),
        (region(x1 - 10.0, y1 - 10.0, x1, y1), Direction::SouthEast),
        (region(x0 + 10.0, y0, x1 - 10.0, y0 + 6.0), Direction::North),
        (region(x0 + 10.0, y1 - 6.0, x1 - 10.0, y1), Direction::South),
        (region(x0, y0 + 10.0, x0 + 6.0, y1 - 10.0), Direction::West),
        (region(x1 - 6.0, y0 + 10.0, x1, y1 - 10.0), Direction::East),
    ]
}

pub fn resize(ctx: &egui::Context) {
    if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
        return;
    }
    let rect = ctx.screen_rect();
    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("settings-outline"),
    ))
    .rect_stroke(
        rect.shrink(0.5),
        18,
        egui::Stroke::new(1.0_f32, theme::LINE),
        egui::StrokeKind::Inside,
    );
    for (index, (edge, direction)) in edges(rect).into_iter().enumerate() {
        egui::Area::new(egui::Id::new(("settings-resize", index)))
            .order(egui::Order::Foreground)
            .fixed_pos(edge.min)
            .movable(false)
            .constrain(false)
            .show(ctx, |ui| {
                let response = ui.allocate_exact_size(edge.size(), egui::Sense::drag()).1;
                if response.hovered() {
                    ctx.set_cursor_icon(match direction {
                        Direction::North | Direction::South => egui::CursorIcon::ResizeVertical,
                        Direction::East | Direction::West => egui::CursorIcon::ResizeHorizontal,
                        Direction::NorthEast | Direction::SouthWest => egui::CursorIcon::ResizeNeSw,
                        Direction::NorthWest | Direction::SouthEast => egui::CursorIcon::ResizeNwSe,
                    });
                }
                if ui.input(|i| {
                    i.pointer.button_pressed(egui::PointerButton::Primary)
                        && i.pointer.press_origin().is_some_and(|p| edge.contains(p))
                }) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ctx: &egui::Context, events: Vec<egui::Event>, close: &mut bool) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(920.0, 780.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                *close = show(ctx);
                resize(ctx);
            },
        )
    }

    #[test]
    fn title_controls_close_only_settings_and_emit_native_window_commands() {
        for (point, expected) in [
            (egui::pos2(891.0, 23.0), None),
            (
                egui::pos2(845.0, 23.0),
                Some(egui::ViewportCommand::Maximized(true)),
            ),
            (
                egui::pos2(799.0, 23.0),
                Some(egui::ViewportCommand::Minimized(true)),
            ),
        ] {
            let ctx = egui::Context::default();
            let mut closed = false;
            frame(&ctx, Vec::new(), &mut closed);
            let mut commands = Vec::new();
            for pressed in [true, false] {
                let output = frame(
                    &ctx,
                    vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    &mut closed,
                );
                commands.extend(
                    output.viewport_output[&egui::ViewportId::ROOT]
                        .commands
                        .clone(),
                );
            }
            assert_eq!(closed, expected.is_none());
            if let Some(command) = expected {
                assert!(commands.contains(&command));
            }
            assert!(!commands.contains(&egui::ViewportCommand::Close));
            assert!(!commands.contains(&egui::ViewportCommand::StartDrag));
        }
    }

    #[test]
    fn frameless_title_and_edge_begin_native_drag_on_mouse_press() {
        for (point, expected) in [
            (egui::pos2(300.0, 24.0), egui::ViewportCommand::StartDrag),
            (
                egui::pos2(918.0, 390.0),
                egui::ViewportCommand::BeginResize(Direction::East),
            ),
        ] {
            let ctx = egui::Context::default();
            let mut close = false;
            frame(&ctx, vec![egui::Event::PointerMoved(point)], &mut close);
            frame(&ctx, Vec::new(), &mut close);
            let output = frame(
                &ctx,
                vec![egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                }],
                &mut close,
            );
            assert!(
                output.viewport_output[&egui::ViewportId::ROOT]
                    .commands
                    .contains(&expected)
            );
        }
    }

    #[test]
    fn coalesced_pointer_motion_does_not_lose_the_original_resize_press() {
        let ctx = egui::Context::default();
        let mut closed = false;
        frame(&ctx, Vec::new(), &mut closed);
        let edge = egui::pos2(918.0, 390.0);
        let out = frame(
            &ctx,
            vec![
                egui::Event::PointerMoved(edge),
                egui::Event::PointerButton {
                    pos: edge,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerMoved(egui::pos2(700.0, 390.0)),
            ],
            &mut closed,
        );
        assert!(
            out.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::BeginResize(Direction::East))
        );
    }
    #[test]
    fn resize_regions_cover_all_edges_without_hiding_content_or_title_buttons() {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(920.0, 780.0));
        let regions = edges(rect);
        for (point, expected) in [
            (egui::pos2(2.0, 2.0), Direction::NorthWest),
            (egui::pos2(918.0, 2.0), Direction::NorthEast),
            (egui::pos2(2.0, 778.0), Direction::SouthWest),
            (egui::pos2(918.0, 778.0), Direction::SouthEast),
            (egui::pos2(460.0, 2.0), Direction::North),
            (egui::pos2(460.0, 778.0), Direction::South),
            (egui::pos2(2.0, 390.0), Direction::West),
            (egui::pos2(918.0, 390.0), Direction::East),
        ] {
            assert_eq!(
                regions
                    .iter()
                    .filter(|(r, _)| r.contains(point))
                    .map(|(_, d)| *d)
                    .collect::<Vec<_>>(),
                vec![expected]
            );
        }
        for point in [
            rect.center(),
            egui::pos2(890.0, 24.0),
            egui::pos2(70.0, 110.0),
        ] {
            assert!(!regions.iter().any(|(r, _)| r.contains(point)));
        }
    }
}
