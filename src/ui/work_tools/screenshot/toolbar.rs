use super::Page;
use crate::{
    application::screenshot::{Output, ScreenshotService},
    domain::screenshot::Tool,
    ui::common::{
        actions,
        components::surface::{self, Role},
        icons::{self, Icon},
        theme,
    },
};
use eframe::egui::{self, Color32};

pub fn show(ui: &mut egui::Ui, page: &mut Page, service: &mut ScreenshotService) {
    ui.add_enabled_ui(!service.busy(), |ui| {
        ui.horizontal_wrapped(|ui| {
            let (icon_rect, _) =
                ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
            icons::draw(ui.painter(), icon_rect, Icon::Annotate, Color32::WHITE);
            for tool in [Tool::Rectangle, Tool::Arrow, Tool::Pen, Tool::Cover] {
                if tool_button(ui, tool, page.tool == tool).clicked() {
                    page.tool = tool;
                    page.stroke.clear();
                }
            }
            ui.separator();
            ui.color_edit_button_srgb(&mut page.color);
            ui.label("笔宽");
            ui.add(egui::Slider::new(&mut page.width, 1..=24).suffix(" px"));
            if let Some(document) = service.document.as_mut() {
                ui.add_enabled_ui(document.can_undo(), |ui| {
                    if icons::button(ui, Icon::Restore, "撤销 · Ctrl+Z").clicked() {
                        document.undo();
                    }
                });
                ui.add_enabled_ui(document.can_redo(), |ui| {
                    if icons::button(ui, Icon::Check, "重做 · Ctrl+Y").clicked() {
                        document.redo();
                    }
                });
                if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
                    document.undo();
                }
                if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Y)) {
                    document.redo();
                }
            }
        });
    });
    ui.horizontal(|ui| {
        let width = ((ui.available_width() - 60.0) / 3.0).max(90.0);
        for (icon, label, kind) in [
            (Icon::Copy, "复制图片", Output::Copy),
            (Icon::Save, "保存 PNG", Output::Save),
            (Icon::PinnedImage, "置顶贴图", Output::Pin),
        ] {
            ui.allocate_ui_with_layout(
                egui::vec2(width, 36.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.add_enabled_ui(!service.busy(), |ui| {
                        if actions::secondary(ui, icon, label).clicked() {
                            page.stroke.clear();
                            service.export(kind);
                        }
                    });
                },
            );
        }
        ui.add_enabled_ui(!service.busy(), |ui| {
            if icons::button(ui, Icon::Cancel, "关闭预览并释放图片").clicked() {
                service.discard();
                page.release();
            }
        });
    });
    ui.label(
        egui::RichText::new("遮挡会永久写入导出图；编辑预览缩放不会降低输出分辨率。")
            .size(11.0)
            .color(theme::MUTED),
    );
}
fn tool_button(ui: &mut egui::Ui, tool: Tool, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(36.0, 36.0), egui::Sense::click());
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tool.label()));
    surface::paint(
        ui,
        &response,
        if selected {
            Role::Selected
        } else {
            Role::Quiet
        },
        theme::CONTROL_RADIUS,
    );
    let r = rect.shrink(9.0);
    let stroke = egui::Stroke::new(
        2.0_f32,
        if selected {
            theme::ACCENT
        } else {
            Color32::WHITE
        },
    );
    match tool {
        Tool::Rectangle => {
            ui.painter()
                .rect_stroke(r, 2, stroke, egui::StrokeKind::Inside);
        }
        Tool::Arrow => {
            ui.painter()
                .arrow(r.left_bottom(), r.right_top() - r.left_bottom(), stroke);
        }
        Tool::Pen => {
            ui.painter().line_segment(
                [r.left_bottom(), r.right_top()],
                egui::Stroke::new(4.0_f32, stroke.color),
            );
            ui.painter()
                .circle_filled(r.left_bottom(), 2.0, stroke.color);
        }
        Tool::Cover => {
            ui.painter().rect_filled(r, 2, stroke.color);
        }
    }
    response.on_hover_text(tool.label())
}
