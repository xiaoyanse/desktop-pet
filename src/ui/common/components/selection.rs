use super::{
    super::{
        icons::{self, Icon},
        theme,
    },
    surface::{self, Role},
};
use eframe::egui::{self, Color32};

pub fn choice(ui: &mut egui::Ui, label: &str, selected: bool, width: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(width.max(24.0), theme::CONTROL_HEIGHT),
        egui::Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    surface::paint(
        ui,
        &response,
        if selected {
            Role::Selected
        } else {
            Role::Secondary
        },
        theme::CONTROL_RADIUS,
    );
    let text = ui.painter().layout(
        label.to_owned(),
        egui::FontId::proportional(13.0),
        if !response.enabled() {
            theme::DISABLED_TEXT
        } else if selected {
            theme::ACCENT
        } else {
            theme::MUTED
        },
        rect.width() - 16.0,
    );
    ui.painter().with_clip_rect(rect.shrink(4.0)).galley(
        rect.center() - text.size() * 0.5,
        text,
        theme::TEXT,
    );
    response
}

pub fn checkbox(ui: &mut egui::Ui, value: &mut bool, label: &str) -> egui::Response {
    let text = ui.painter().layout(
        label.to_owned(),
        egui::FontId::proportional(13.0),
        if ui.is_enabled() {
            theme::TEXT
        } else {
            theme::DISABLED_TEXT
        },
        (ui.available_width() - 36.0).max(1.0),
    );
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(
            (text.size().x + 36.0).min(ui.available_width()),
            text.size().y.max(28.0) + 4.0,
        ),
        egui::Sense::hover(),
    );
    let mut response = ui.interact(
        rect,
        ui.id().with(("checkbox", label)),
        egui::Sense::click(),
    );
    if response.clicked() && ui.is_enabled() {
        *value = !*value;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *value, label)
    });
    let square = egui::Rect::from_center_size(
        rect.left_center() + egui::vec2(12.0, 0.0),
        egui::Vec2::splat(24.0),
    );
    let fill = if !response.enabled() {
        theme::CONTROL_DISABLED
    } else if *value {
        theme::CONTROL_SELECTED
    } else {
        theme::FIELD
    };
    ui.painter().rect_filled(square, 7, fill);
    ui.painter().rect_stroke(
        square,
        7,
        egui::Stroke::new(
            1.0_f32,
            if !response.enabled() {
                theme::LINE.gamma_multiply(0.45)
            } else if response.has_focus() || response.hovered() || *value {
                theme::ACCENT
            } else {
                theme::LINE
            },
        ),
        egui::StrokeKind::Inside,
    );
    if *value {
        icons::draw(
            ui.painter(),
            square.shrink(2.0),
            Icon::Check,
            if response.enabled() {
                Color32::WHITE
            } else {
                Color32::from_white_alpha(75)
            },
        );
    }
    ui.painter().galley(
        egui::pos2(rect.left() + 36.0, rect.center().y - text.size().y * 0.5),
        text,
        theme::TEXT,
    );
    response
}

pub fn dropdown<R>(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    label: &str,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = theme::CONTROL_HEIGHT;
        egui::ComboBox::from_id_salt(id)
            .width(ui.available_width())
            .selected_text(label)
            .truncate()
            .icon(|ui, rect, _, _, _| icons::draw(ui.painter(), rect, Icon::Down, Color32::WHITE))
            .show_ui(ui, contents)
    })
    .inner
}
