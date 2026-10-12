use super::{Confirmation, Page};
use crate::{
    application::clipboard::ClipboardService,
    domain::clipboard::{Content, Item},
    ui::common::{
        glass,
        icons::{self, Icon},
        theme,
    },
};
use eframe::egui::{self, Color32};
use std::collections::HashSet;

pub fn show(ui: &mut egui::Ui, page: &mut Page, service: &mut ClipboardService) {
    if page.filtered.is_empty() {
        page.textures.clear();
        theme::glass_panel(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(if service.history.items.is_empty() {
                    "还没有记录的内容"
                } else {
                    "没有匹配的内容"
                })
                .size(20.0),
            );
            ui.label(
                egui::RichText::new(if service.history.items.is_empty() {
                    "手动开启后，复制文字或图片即可记录。暂停和退出都不会清空系统剪贴板。"
                } else {
                    "更换关键词或筛选类型；图片可按尺寸查找。"
                })
                .color(theme::MUTED),
            );
        });
        return;
    }
    let mut keep = HashSet::new();
    let mut pin = None;
    let mut copy_id = None;
    egui::ScrollArea::vertical()
        .id_salt("clipboard-history")
        .auto_shrink([false, false])
        .show_rows(ui, 104.0, page.filtered.len(), |ui, rows| {
            for row in rows {
                let id = page.filtered[row];
                let Some(item) = service.history.items.iter().find(|i| i.id == id) else {
                    continue;
                };
                ui.push_id(id, |ui| {
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 104.0),
                        egui::Sense::click(),
                    );
                    let card = rect.shrink2(egui::vec2(0.0, 2.0));
                    ui.painter().rect_filled(card, 12, theme::SURFACE);
                    ui.painter().rect_stroke(
                        card,
                        12,
                        egui::Stroke::new(
                            1.0_f32,
                            if item.pinned {
                                theme::ACCENT.gamma_multiply(0.5)
                            } else {
                                theme::LINE
                            },
                        ),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter()
                        .add(glass::finish(card.shrink(1.0), 11.0, response.hovered()));
                    if response.double_clicked() {
                        page.detail = Some(id);
                    }
                    let header = egui::Rect::from_min_size(
                        card.min + egui::vec2(12.0, 8.0),
                        egui::vec2(card.width() - 24.0, 30.0),
                    );
                    let mut copy = false;
                    ui.scope_builder(
                        egui::UiBuilder::new()
                            .max_rect(header)
                            .layout(egui::Layout::right_to_left(egui::Align::Center)),
                        |ui| {
                            if icons::sized_button(ui, Icon::Cancel, "删除这一条", 30.0)
                                .on_hover_text("删除这一条历史")
                                .clicked()
                            {
                                page.confirm = Some(Confirmation::Delete(id));
                            }
                            if icons::sized_button(
                                ui,
                                if item.pinned { Icon::Unpin } else { Icon::Pin },
                                "切换固定",
                                30.0,
                            )
                            .on_hover_text(if item.pinned {
                                "取消固定"
                            } else {
                                "固定在顶部，避免自动淘汰"
                            })
                            .clicked()
                            {
                                pin = Some(id);
                            }
                            if ui
                                .add_enabled_ui(!service.copying(), |ui| {
                                    icons::sized_button(ui, Icon::Copy, "复制原始内容", 30.0)
                                })
                                .inner
                                .on_hover_text("复制原始内容")
                                .clicked()
                            {
                                copy = true;
                            }
                            if icons::sized_button(ui, Icon::Details, "查看内容", 30.0)
                                .on_hover_text("查看完整文字或图片信息")
                                .clicked()
                            {
                                page.detail = Some(id);
                            }
                        },
                    );
                    draw_text(ui, header, item);
                    let left = card.left() + 12.0;
                    let top = card.top() + 45.0;
                    let reserved = if let Some(thumb) = &item.packet.thumbnail {
                        keep.insert(id);
                        let texture = page.textures.entry(id).or_insert_with(|| {
                            ui.ctx().load_texture(
                                format!("clipboard-thumbnail-{id}"),
                                egui::ColorImage::from_rgba_unmultiplied(
                                    [thumb.size()[0] as usize, thumb.size()[1] as usize],
                                    thumb.pixels(),
                                ),
                                egui::TextureOptions::LINEAR,
                            )
                        });
                        let size = texture.size_vec2();
                        let scale = (74.0 / size.x).min(44.0 / size.y);
                        let image = egui::Rect::from_center_size(
                            egui::pos2(left + 37.0, top + 22.0),
                            size * scale,
                        );
                        ui.painter().image(
                            texture.id(),
                            image,
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                        86.0
                    } else {
                        0.0
                    };
                    let label = item.packet.label().replace(['\n', '\r'], " ");
                    let mut job = egui::text::LayoutJob::simple(
                        label,
                        egui::FontId::proportional(13.0),
                        theme::MUTED,
                        (card.width() - 24.0 - reserved).max(1.0),
                    );
                    job.wrap.max_rows = 2;
                    let text = ui.fonts(|f| f.layout_job(job));
                    ui.painter().with_clip_rect(card).galley(
                        egui::pos2(left + reserved, top),
                        text,
                        theme::MUTED,
                    );
                    if copy {
                        copy_id = Some(id);
                    }
                });
            }
        });
    page.textures.retain(|id, _| keep.contains(id));
    if let Some(id) = pin {
        service.history.pin(id);
    }
    if let Some(id) = copy_id {
        service.copy(id);
    }
}
fn draw_text(ui: &egui::Ui, header: egui::Rect, item: &Item) {
    let age = crate::application::office::now().saturating_sub(item.captured);
    let kind = match item.packet.content {
        Content::Text(_) => "文字",
        Content::Picture(_) => "图片",
    };
    let time = if age < 60 {
        "刚刚".into()
    } else if age < 3600 {
        format!("{} 分钟前", age / 60)
    } else {
        format!("{} 小时前", age / 3600)
    };
    let label = format!(
        "{kind} · {time}{}",
        if item.pinned { " · 已固定" } else { "" }
    );
    let mut job = egui::text::LayoutJob::simple(
        label,
        egui::FontId::proportional(13.0),
        theme::TEXT,
        (header.width() - 156.0).max(1.0),
    );
    job.wrap.max_rows = 1;
    let text = ui.fonts(|f| f.layout_job(job));
    ui.painter().with_clip_rect(header).galley(
        header.left_center() - egui::vec2(0.0, text.size().y * 0.5),
        text,
        theme::TEXT,
    );
}
