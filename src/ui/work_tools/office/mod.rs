mod editor;
mod list;
pub mod notice;
#[cfg(test)]
mod tests;
use crate::{
    application::office::OfficeService,
    domain::office::{Item, Kind},
    ui::common::{
        actions,
        components::{
            fields, selection,
            status::{self, Tone},
        },
        icons::Icon,
        theme,
    },
};
use eframe::egui::{self, RichText};
pub(super) struct Draft {
    id: Option<u64>,
    kind: Kind,
    title: String,
    body: String,
    pinned: bool,
    due: String,
}
pub struct Page {
    kind: Kind,
    query: String,
    editor: Option<Draft>,
    confirm_delete: Option<u64>,
    waiting: Option<u64>,
}
impl Default for Page {
    fn default() -> Self {
        Self {
            kind: Kind::Note,
            query: String::new(),
            editor: None,
            confirm_delete: None,
            waiting: None,
        }
    }
}
impl Page {
    pub fn open(&mut self, kind: Kind) {
        self.kind = kind;
        self.confirm_delete = None;
    }
    fn edit(&mut self, item: &Item, service: &OfficeService) {
        self.editor = Some(Draft {
            id: Some(item.id),
            kind: item.kind,
            title: item.title.clone(),
            body: item.body.clone(),
            pinned: item.pinned,
            due: item
                .due
                .map(|d| service.clock.format(d))
                .unwrap_or_default(),
        });
    }
    pub(crate) fn edit_item(&mut self, id: u64, service: &OfficeService) {
        if let Some(item) = service.book.items.iter().find(|i| i.id == id) {
            self.edit(item, service);
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, service: &mut OfficeService) {
        if self
            .waiting
            .is_some_and(|revision| service.saved_revision >= revision)
        {
            self.editor = None;
            self.waiting = None;
        }
        if self.waiting.is_some() && !service.busy() {
            self.waiting = None;
        }
        ui.horizontal(|ui| {
            for kind in [Kind::Note, Kind::Task] {
                if selection::choice(ui, kind.label(), self.kind == kind, 72.0).clicked() {
                    self.kind = kind;
                    self.confirm_delete = None;
                }
            }
            ui.add_space(14.0);
            fields::line(ui, &mut self.query, "搜索标题或正文", 128);
        });
        ui.add_space(12.0);
        let notes = service
            .book
            .items
            .iter()
            .filter(|i| i.kind == Kind::Note)
            .count();
        let unfinished = service
            .book
            .items
            .iter()
            .filter(|i| i.kind == Kind::Task && !i.done)
            .count();
        ui.label(
            RichText::new(format!(
                "{notes} 张便签    {unfinished} 项待办    {} 项到期提醒",
                service.reminder_count()
            ))
            .color(theme::CHAMPAGNE),
        );
        ui.add_space(8.0);
        if actions::primary(
            ui,
            if self.kind == Kind::Note {
                Icon::Note
            } else {
                Icon::Todo
            },
            &format!("新建{}", self.kind.label()),
            service.ready && !service.busy(),
        )
        .clicked()
        {
            self.editor = Some(Draft {
                id: None,
                kind: self.kind,
                title: String::new(),
                body: String::new(),
                pinned: false,
                due: String::new(),
            });
            self.confirm_delete = None;
        }
        if (!service.ready || service.failed)
            && !service.busy()
            && actions::secondary(ui, Icon::Restore, "重新载入本地文件").clicked()
        {
            service.reload();
        }
        if !service.message.is_empty() {
            if service.failed {
                status::message(ui, &service.message, Tone::Error);
            } else {
                status::badge(ui, &service.message, Tone::Success);
            }
        }
        ui.add(
            egui::Label::new(
                RichText::new(service.location())
                    .size(11.0)
                    .color(theme::MUTED),
            )
            .truncate(),
        )
        .on_hover_text(service.location());
        ui.add_space(12.0);
        egui::ScrollArea::vertical()
            .id_salt("office-list")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.editor.is_some() {
                    editor::show(ui, self, service);
                    ui.add_space(12.0);
                }
                list::show(ui, self, service);
            });
    }
}
