pub(super) mod backdrop;
pub(super) mod navigation;
pub(super) mod window_chrome;

pub fn content_frame() -> eframe::egui::Frame {
    eframe::egui::Frame::new().inner_margin(eframe::egui::Margin {
        left: 24,
        right: 24,
        top: 8,
        bottom: 20,
    })
}
