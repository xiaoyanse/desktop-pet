use crate::{
    application::office::OfficeService,
    domain::{
        desktop::{DesktopGeometry, flyout_position},
        office::{Command, Item},
    },
    ui::common::{actions, icons::Icon, theme},
};
use eframe::egui::{self, RichText};
pub fn show(
    ctx: &egui::Context,
    service: &mut OfficeService,
    geometry: Option<DesktopGeometry>,
) -> bool {
    let Some(item) = service.notice().cloned() else {
        return false;
    };
    let Some(geometry) = geometry else {
        return false;
    };
    let size = [360.0, 296.0];
    let mut open = false;
    let mut command = None;
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("office-reminder"),
        egui::ViewportBuilder::default()
            .with_title("资源助手 · 待办提醒")
            .with_decorations(false)
            .with_taskbar(false)
            .with_transparent(true)
            .with_active(false)
            .with_always_on_top()
            .with_resizable(false)
            .with_inner_size(size)
            .with_position(flyout_position(geometry, size)),
        |ctx, _| {
            if ctx.input(|i| i.viewport().close_requested()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                command = Some(Command::Dismiss(item.id));
            }
            egui::CentralPanel::default()
                .frame(theme::glass_frame().fill(theme::SURFACE))
                .show(ctx, |ui| {
                    let (next, opened) = panel(ui, service, &item);
                    command = next.or(command.take());
                    open |= opened;
                });
        },
    );
    if let Some(command) = command {
        service.apply(command);
    }
    open
}

pub(crate) fn panel(
    ui: &mut egui::Ui,
    service: &OfficeService,
    item: &Item,
) -> (Option<Command>, bool) {
    let mut command = None;
    let mut open = false;
    ui.label(
        RichText::new("有一项行动到时间了")
            .size(20.0)
            .color(theme::CHAMPAGNE),
    );
    ui.add(egui::Label::new(RichText::new(&item.title).size(16.0)).truncate())
        .on_hover_text(&item.title);
    ui.weak(format!("{} 项提醒待处理", service.reminder_count()));
    if actions::primary(ui, Icon::Check, "标记完成", !service.busy()).clicked() {
        command = Some(Command::Complete(item.id));
    }
    ui.horizontal(|ui| {
        ui.add_enabled_ui(!service.busy(), |ui| {
            if actions::secondary(ui, Icon::Reminder, "10分钟后提醒").clicked() {
                command = Some(Command::Snooze(item.id, 600));
            }
            if actions::secondary(ui, Icon::Cancel, "本次忽略").clicked() {
                command = Some(Command::Dismiss(item.id));
            }
        });
    });
    if actions::secondary(ui, Icon::Todo, "打开待办列表").clicked() {
        open = true;
    }
    if service.failed {
        ui.add(
            egui::Label::new(
                RichText::new(&service.message)
                    .size(11.0)
                    .color(theme::CHAMPAGNE),
            )
            .truncate(),
        )
        .on_hover_text(&service.message);
    }
    (command, open)
}
