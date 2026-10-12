use super::{Confirmation, Page};
use crate::{
    application::clipboard::ClipboardService,
    domain::clipboard::{ClipboardConfig, Content},
    ui::common::{
        actions,
        components::{
            fields,
            status::{self, Tone},
        },
        icons::Icon,
        theme, widgets,
    },
};
use eframe::egui::{self, RichText};

pub fn policy(
    ui: &mut egui::Ui,
    page: &mut Page,
    config: &mut ClipboardConfig,
    service: &mut ClipboardService,
) {
    let mut apply = false;
    let mut cancel = false;
    theme::glass_panel(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("历史容量").size(16.0).strong());
        let draft = page.policy.as_mut().unwrap();
        let mut count = draft.count as u64;
        let mut mib = draft.mib as u64;
        if widgets::integer(ui, "最多条数", &mut count, 1..=200, " 条") {
            draft.count = count as usize;
        }
        if widgets::integer(ui, "历史容量", &mut mib, 4..=64, " MiB") {
            draft.mib = mib as usize;
        }
        ui.label(
            RichText::new(
                "默认30条 / 32 MiB；降低容量会淘汰最旧的未固定项，固定项无法容纳时保留原设置。",
            )
            .size(12.0)
            .color(theme::MUTED),
        );
        ui.horizontal(|ui| {
            apply = actions::secondary(ui, Icon::Check, "应用容量").clicked();
            cancel = actions::secondary(ui, Icon::Restore, "取消调整").clicked();
        });
    });
    if apply {
        let mut draft = page.policy.take().unwrap();
        service.configure(&mut draft);
        *config = service.config.clone();
    } else if cancel {
        page.policy = None;
    }
}
pub fn confirmation(ui: &mut egui::Ui, page: &mut Page, service: &mut ClipboardService) {
    let missing = matches!(page.confirm,Some(Confirmation::Delete(id)) if !service.history.items.iter().any(|i|i.id==id));
    if missing || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        page.confirm = None;
        return;
    }
    let all = matches!(page.confirm, Some(Confirmation::Clear));
    let mut accept = false;
    let mut cancel = false;
    theme::glass_panel(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            RichText::new(if all {
                "清空本次历史？"
            } else {
                "删除这条历史？"
            })
            .size(18.0)
            .color(theme::DANGER),
        );
        ui.label(if all {
            format!(
                "将清空当前{}条历史（包括固定项），无法撤销；系统当前剪贴板保持原样。",
                service.history.items.len()
            )
        } else {
            "只删除本工具中的这一条记录，系统当前剪贴板保持原样。".into()
        });
        accept = actions::danger(
            ui,
            Icon::Cancel,
            if all {
                "确认清空历史"
            } else {
                "确认删除这一条"
            },
            true,
        )
        .clicked();
        cancel = actions::secondary(ui, Icon::Restore, "保留内容").clicked();
    });
    if accept {
        if let Some(c) = page.confirm.take() {
            match c {
                Confirmation::Clear => service.history.clear(),
                Confirmation::Delete(id) => service.history.delete(id),
            }
        }
        page.textures.clear();
        page.detail = None;
    } else if cancel {
        page.confirm = None;
    }
}
pub fn detail(ui: &mut egui::Ui, page: &mut Page, service: &mut ClipboardService) {
    let Some(item) = service
        .history
        .items
        .iter()
        .find(|i| Some(i.id) == page.detail)
    else {
        page.detail = None;
        return;
    };
    let id = item.id;
    let mut copy = false;
    let mut close = false;
    theme::glass_panel(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("内容预览").size(16.0).strong());
        match &item.packet.content {
            Content::Text(text) => {
                let mut body = text.as_ref();
                egui::ScrollArea::vertical()
                    .id_salt("clipboard-full-text")
                    .max_height(180.0)
                    .show(ui, |ui| {
                        fields::entry(
                            ui,
                            egui::TextEdit::multiline(&mut body)
                                .desired_rows(5)
                                .desired_width(ui.available_width()),
                            false,
                        );
                    });
            }
            Content::Picture(image) => {
                ui.label(format!(
                    "原图 {} × {} · {:.2} MiB",
                    image.size()[0],
                    image.size()[1],
                    image.pixels().len() as f64 / 1048576.0
                ));
                status::badge(ui, "列表显示缩略图，复制保留原始分辨率", Tone::Neutral);
            }
        }
        ui.horizontal(|ui| {
            copy = ui
                .add_enabled_ui(!service.copying(), |ui| {
                    actions::secondary(ui, Icon::Copy, "复制原始内容")
                })
                .inner
                .clicked();
            close = actions::secondary(ui, Icon::Restore, "收起预览").clicked();
        });
    });
    if copy {
        service.copy(id);
    }
    if close {
        page.detail = None;
    }
}
