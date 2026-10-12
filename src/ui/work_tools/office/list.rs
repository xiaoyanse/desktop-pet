use super::Page;
use crate::{
    application::office::{self, OfficeService},
    domain::office::{Command, Kind},
    ui::common::{
        actions,
        components::{
            selection,
            status::{self, Tone},
        },
        icons::{self, Icon},
        theme,
    },
};
use eframe::egui::{self, Color32, RichText};
use std::cmp::Reverse;
pub fn show(ui: &mut egui::Ui, page: &mut Page, service: &mut OfficeService) {
    let query = page.query.to_lowercase();
    let mut items: Vec<_> = service
        .book
        .items
        .iter()
        .filter(|i| {
            i.kind == page.kind
                && (query.is_empty()
                    || i.title.to_lowercase().contains(&query)
                    || i.body.to_lowercase().contains(&query))
        })
        .collect();
    items.sort_by_key(|i| {
        (
            !i.pinned,
            i.done,
            if i.kind == Kind::Task {
                i.due.unwrap_or(u64::MAX)
            } else {
                0
            },
            Reverse(i.updated),
            i.id,
        )
    });
    if items.is_empty() {
        theme::glass_panel(ui, |ui| {
            ui.label(
                RichText::new(if page.query.is_empty() {
                    "给想法和行动留个位置"
                } else {
                    "没有匹配的条目"
                })
                .size(20.0),
            );
            ui.weak(if page.query.is_empty() {
                "新建便签记录线索；新建待办安排下一步和提醒。"
            } else {
                "试试更短的关键词，或切换便签与待办。"
            });
        });
        return;
    }
    let mut command = None;
    let mut edit = None;
    for item in items {
        ui.push_id(item.id, |ui| {
            theme::glass_panel(ui, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                    icons::draw(
                        ui.painter(),
                        rect,
                        if item.kind == Kind::Note {
                            Icon::Note
                        } else {
                            Icon::Todo
                        },
                        Color32::WHITE,
                    );
                    let mut done = item.done;
                    if item.kind == Kind::Task
                        && ui
                            .add_enabled_ui(!service.busy(), |ui| {
                                selection::checkbox(ui, &mut done, "")
                            })
                            .inner
                            .changed()
                    {
                        command = Some(Command::Complete(item.id));
                    }
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width() - 102.0, 28.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&item.title).size(16.0).color(if item.done {
                                        theme::MUTED
                                    } else {
                                        theme::TEXT
                                    }),
                                )
                                .truncate(),
                            )
                        },
                    )
                    .inner
                    .on_hover_text(&item.title);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_enabled_ui(!service.busy(), |ui| {
                            if icons::button(ui, Icon::Cancel, "删除条目").clicked() {
                                page.confirm_delete = Some(item.id);
                            }
                            if icons::button(ui, Icon::Edit, "编辑条目").clicked() {
                                edit = Some(item.clone());
                            }
                            if icons::button(
                                ui,
                                if item.pinned { Icon::Unpin } else { Icon::Pin },
                                if item.pinned {
                                    "取消收藏"
                                } else {
                                    "收藏到顶部"
                                },
                            )
                            .clicked()
                            {
                                command = Some(Command::Pin(item.id));
                            }
                        });
                    });
                });
                if !item.body.is_empty() {
                    ui.add(
                        egui::Label::new(
                            RichText::new(item.body.lines().take(2).collect::<Vec<_>>().join("\n"))
                                .size(12.0)
                                .color(theme::MUTED),
                        )
                        .truncate(),
                    );
                }
                if let Some(due) = item.due {
                    let now = office::now();
                    let color = if item.done {
                        theme::MUTED
                    } else if due <= now {
                        theme::DANGER
                    } else if due - now <= 3600 {
                        theme::CHAMPAGNE
                    } else {
                        theme::ACCENT
                    };
                    status::badge(
                        ui,
                        &format!(
                            "{} · {}",
                            service.clock.format(due),
                            if item.done {
                                "已完成"
                            } else if item.acknowledged_due == Some(due) {
                                "本次提醒已忽略"
                            } else if due <= now {
                                "已到期"
                            } else {
                                "到期提醒"
                            }
                        ),
                        if color == theme::DANGER {
                            Tone::Error
                        } else if color == theme::CHAMPAGNE {
                            Tone::Warning
                        } else if color == theme::ACCENT {
                            Tone::Success
                        } else {
                            Tone::Neutral
                        },
                    );
                }
                if page.confirm_delete == Some(item.id) {
                    ui.label("确认删除此条目？");
                    if actions::danger(ui, Icon::Cancel, "确认删除", !service.busy()).clicked()
                    {
                        command = Some(Command::Delete(item.id));
                    }
                    if actions::secondary(ui, Icon::Restore, "保留条目").clicked() {
                        page.confirm_delete = None;
                    }
                }
            });
        });
        ui.add_space(10.0);
    }
    if let Some(item) = edit {
        page.edit(&item, service);
    }
    if let Some(command) = command
        && service.apply(command)
    {
        page.confirm_delete = None;
    }
}
