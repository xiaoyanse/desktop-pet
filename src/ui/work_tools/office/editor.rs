use super::Page;
use crate::{
    application::office::{self, OfficeService},
    domain::office::{Command, Kind},
    ui::common::{
        actions,
        components::{fields, selection},
        icons::Icon,
        theme,
    },
};
use eframe::egui;
pub fn show(ui: &mut egui::Ui, page: &mut Page, service: &mut OfficeService) {
    let Some(draft) = page.editor.as_mut() else {
        return;
    };
    let mut save = false;
    let mut cancel = false;
    theme::glass_panel(ui, |ui| {
        ui.label(if draft.id.is_some() {
            "编辑条目"
        } else {
            "记下一个想法或下一步行动"
        });
        ui.add_enabled_ui(!service.busy(), |ui| {
            fields::line(ui, &mut draft.title, "标题", 64);
            fields::multiline(ui, &mut draft.body, "正文、命令或工作线索", 8192, 4);
            selection::checkbox(ui, &mut draft.pinned, "收藏到列表顶部");
            if draft.kind == Kind::Task {
                ui.label("到期提醒 · 本地时间，留空则不提醒");
                let invalid =
                    !draft.due.trim().is_empty() && service.clock.parse(&draft.due).is_err();
                fields::entry(
                    ui,
                    egui::TextEdit::singleline(&mut draft.due)
                        .hint_text("YYYY-MM-DD HH:mm")
                        .char_limit(16)
                        .desired_width(ui.available_width()),
                    invalid,
                );
                ui.horizontal_wrapped(|ui| {
                    for (label, seconds) in [
                        ("无提醒", 0),
                        ("30分钟后", 1800),
                        ("1小时后", 3600),
                        ("24小时后", 86400),
                    ] {
                        if selection::choice(ui, label, seconds == 0 && draft.due.is_empty(), 98.0)
                            .clicked()
                        {
                            draft.due = if seconds == 0 {
                                String::new()
                            } else {
                                service.clock.format(office::now() + seconds)
                            };
                        }
                    }
                });
            }
            save = actions::primary(ui, Icon::Save, "保存到本地", !draft.title.trim().is_empty())
                .clicked();
            cancel = actions::secondary(ui, Icon::Cancel, "取消编辑").clicked();
        });
    });
    if save {
        let due = if draft.kind == Kind::Task && !draft.due.trim().is_empty() {
            match service.clock.parse(&draft.due) {
                Ok(due) => Some(due),
                Err(error) => {
                    service.message = error;
                    service.failed = true;
                    return;
                }
            }
        } else {
            None
        };
        if service.apply(Command::Save {
            id: draft.id,
            kind: draft.kind,
            title: draft.title.clone(),
            body: draft.body.clone(),
            pinned: draft.pinned,
            due,
        }) {
            page.waiting = Some(service.saved_revision + 1);
        }
    }
    if cancel {
        page.editor = None;
        page.waiting = None;
    }
}
