use super::{
    artwork,
    components::surface::{self, Role},
    theme,
};
use eframe::egui::{self, Color32, Vec2};

#[derive(Clone, Copy)]
pub enum Icon {
    Settings,
    Pin,
    Unpin,
    Details,
    Compact,
    Restore,
    Up,
    Down,
    Clean,
    Cancel,
    Check,
    Monitor,
    Palette,
    Devices,
    General,
    History,
    Pet,
    Plus,
    Minus,
    Minimize,
    Maximize,
    Timer,
    Play,
    Pause,
    Report,
    Copy,
    Save,
    Screenshot,
    Annotate,
    PinnedImage,
    Note,
    Todo,
    Reminder,
    Edit,
    Meeting,
}

impl Icon {
    pub fn asset(self) -> &'static str {
        match self {
            Self::Settings => "nav-general",
            Self::Pin => "pin",
            Self::Unpin => "unpin",
            Self::Details => "details",
            Self::Compact => "compact",
            Self::Restore => "restore",
            Self::Up => "up",
            Self::Down => "down",
            Self::Clean => "nav-clean",
            Self::Cancel => "close",
            Self::Check => "check",
            Self::Monitor => "nav-monitor",
            Self::Palette | Self::Pet => "nav-appearance",
            Self::Devices => "nav-devices",
            Self::General => "nav-general",
            Self::History => "nav-history",
            Self::Plus => "plus",
            Self::Minus => "minus",
            Self::Minimize => "minimize",
            Self::Maximize => "maximize",
            Self::Timer => "nav-timer",
            Self::Play => "play",
            Self::Pause => "pause",
            Self::Report => "nav-report",
            Self::Copy => "clipboard",
            Self::Save => "save-local",
            Self::Screenshot => "screenshot",
            Self::Annotate => "capture-annotate",
            Self::PinnedImage => "pinned-image",
            Self::Note => "notes",
            Self::Todo => "todo",
            Self::Reminder => "reminder",
            Self::Edit => "capture-annotate",
            Self::Meeting => "meeting",
        }
    }
}

pub fn button(ui: &mut egui::Ui, icon: Icon, tip: &str) -> egui::Response {
    bare_button(ui, icon, tip).on_hover_text(tip)
}

pub fn bare_button(ui: &mut egui::Ui, icon: Icon, tip: &str) -> egui::Response {
    sized_button(ui, icon, tip, 26.0)
}

pub fn sized_button(ui: &mut egui::Ui, icon: Icon, tip: &str, size: f32) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    let response = ui.interact(
        rect,
        ui.id().with(("icon-button", tip)),
        egui::Sense::click(),
    );
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tip));
    surface::paint(
        ui,
        &response,
        Role::Quiet,
        theme::CONTROL_RADIUS.min((size * 0.3) as u8),
    );
    draw(
        ui.painter(),
        rect.shrink(2.0),
        icon,
        if ui.is_enabled() {
            Color32::WHITE
        } else {
            Color32::from_white_alpha(75)
        },
    );
    response
}

pub fn draw(painter: &egui::Painter, rect: egui::Rect, icon: Icon, tint: Color32) {
    artwork::draw(painter, rect, icon.asset(), tint);
}
