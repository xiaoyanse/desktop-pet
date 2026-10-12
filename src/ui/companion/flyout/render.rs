use super::{Action, components};
use crate::{
    application::Services,
    domain::{
        cleanup_profile::Review,
        companion::spiral::{Group, Layout, MenuState, Rect, motion},
        config::Config,
        resources::Snapshot,
    },
    ui::common::{
        artwork, cleanup_choice,
        icons::{self, Icon},
        theme,
    },
};
use eframe::egui::{self, Color32};
use std::time::Duration;
pub struct Content<'a> {
    pub config: &'a Config,
    pub snapshot: &'a Snapshot,
    pub services: &'a mut Services,
    pub review: &'a mut Review,
}
#[derive(Default)]
pub struct Output {
    pub close: bool,
    pub toggle_pin: bool,
    pub action: Option<Action>,
    pub group: Option<Group>,
    pub regions: Vec<Rect>,
}
pub fn rect(raw: Rect) -> egui::Rect {
    egui::Rect::from_min_max(egui::pos2(raw[0], raw[1]), egui::pos2(raw[2], raw[3]))
}
pub fn show(
    ui: &mut egui::Ui,
    layout: &Layout,
    state: &MenuState,
    now: Duration,
    content: &mut Content<'_>,
    pinned: bool,
) -> Output {
    let mut out = Output::default();
    if content.review.operations().is_some() {
        let raw = layout.rect([35.0, 18.0, 512.0, 443.0]);
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect(raw)), |ui| {
            let frame = theme::glass_frame().fill(theme::SURFACE).show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(ui.available_height())
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if let Some(request) = cleanup_choice::review(
                            ui,
                            content.review,
                            !content.services.cleaner.busy(),
                        ) {
                            content.services.cleaner.start(&request, false);
                        }
                    });
            });
            let r = frame.response.rect;
            out.regions.push([r.left(), r.top(), r.right(), r.bottom()]);
        });
        return out;
    }
    for group in Group::ALL {
        let raw = layout.group(
            group,
            now.saturating_sub(state.opened),
            content.config.pet_animation,
        );
        let r = rect(raw);
        let image_rect = egui::Rect::from_center_size(
            r.center() - egui::vec2(0.0, 7.0),
            egui::vec2(r.width(), r.width()),
        );
        let visible_image = image_rect.shrink(image_rect.width() * 0.12);
        out.regions.push([
            visible_image.left(),
            visible_image.top(),
            visible_image.right(),
            visible_image.bottom(),
        ]);
        let response = ui.interact(r, ui.id().with(group.index()), egui::Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, group.label())
        });
        artwork::draw(
            ui.painter(),
            egui::Rect::from_center_size(
                r.center() - egui::vec2(0.0, 7.0),
                egui::vec2(r.width(), r.width()),
            ),
            group.artwork(state.group == Some(group) || response.hovered()),
            Color32::WHITE,
        );
        let label = egui::Rect::from_center_size(
            r.center_bottom() - egui::vec2(0.0, 12.0),
            egui::vec2(54.0, 24.0),
        );
        out.regions
            .push([label.left(), label.top(), label.right(), label.bottom()]);
        ui.painter().rect_filled(label, 12, theme::SURFACE);
        ui.painter().text(
            label.center(),
            egui::Align2::CENTER_CENTER,
            group.label(),
            egui::FontId::proportional(13.0),
            theme::TEXT,
        );
        if response.clicked() {
            out.group = Some(group);
        }
    }
    for (raw, icon, label, is_pin) in [
        ([8.0, 207.0, 48.0, 247.0], Icon::Cancel, "收起入口", false),
        (
            [8.0, 255.0, 48.0, 295.0],
            if pinned { Icon::Unpin } else { Icon::Pin },
            if pinned {
                "取消保持展开"
            } else {
                "保持展开"
            },
            true,
        ),
    ] {
        let raw = layout.rect(raw);
        out.regions.push(raw);
        let r = rect(raw);
        ui.painter().rect_filled(r, 20, theme::SURFACE);
        let response = ui.interact(r, ui.id().with(label), egui::Sense::click());
        icons::draw(ui.painter(), r.shrink(8.0), icon, Color32::WHITE);
        if response.on_hover_text(label).clicked() {
            if is_pin {
                out.toggle_pin = true;
            } else {
                out.close = true;
            }
        }
    }
    if let Some(group) = state.group {
        let progress = motion(
            0,
            now.saturating_sub(state.selected_at),
            content.config.pet_animation,
        );
        let center = rect(layout.group(group, Duration::from_secs(1), false)).center();
        for (index, raw) in components::rects(group).iter().enumerate() {
            let final_rect = rect(layout.rect(*raw));
            let end = final_rect.center();
            let angle = 0.25 * (1.0 - progress);
            let delta = end - center;
            let position = center
                + egui::vec2(
                    delta.x * angle.cos() - delta.y * angle.sin(),
                    delta.x * angle.sin() + delta.y * angle.cos(),
                ) * progress;
            let r = egui::Rect::from_center_size(position, final_rect.size());
            ui.scope_builder(egui::UiBuilder::new().max_rect(r), |ui| {
                let frame = theme::glass_frame().fill(theme::SURFACE).show(ui, |ui| {
                    components::show(ui, group, index, content, &mut out.action);
                });
                let r = frame.response.rect;
                out.regions.push([r.left(), r.top(), r.right(), r.bottom()]);
            });
        }
    }
    out
}
