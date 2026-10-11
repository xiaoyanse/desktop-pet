use super::{panel, pet, theme};
use crate::domain::config::Config;
use eframe::egui::{self, Color32};

pub fn frame(config: &Config) -> egui::Frame {
    egui::Frame::new()
        .fill(if config.background {
            Color32::from_rgba_unmultiplied(
                theme::BACKGROUND.r(),
                theme::BACKGROUND.g(),
                theme::BACKGROUND.b(),
                (config.opacity * 255.0) as u8,
            )
        } else {
            Color32::TRANSPARENT
        })
        .corner_radius(9)
        .inner_margin(if config.compact { 6 } else { 9 })
        .stroke(if config.border {
            egui::Stroke::new(1.0_f32, theme::LINE.gamma_multiply(1.5))
        } else {
            egui::Stroke::NONE
        })
}

pub fn size(
    config: &Config,
    rows: &[panel::DisplayRow],
    expanded: bool,
    screen_height: f32,
) -> egui::Vec2 {
    if config.pet_mode {
        pet::SIZE
    } else if config.compact {
        panel::compact_size(rows)
    } else {
        egui::vec2(
            240.0,
            panel::height(rows, expanded).min((screen_height - 80.0).max(180.0)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::resources::Snapshot;

    #[test]
    fn background_and_border_are_independent_and_opacity_applies_only_to_fill() {
        let mut config = Config {
            opacity: 0.47,
            background: true,
            border: false,
            ..Default::default()
        };
        assert_eq!(frame(&config).fill.a(), 119);
        assert_eq!(frame(&config).stroke, egui::Stroke::NONE);
        config.background = false;
        config.border = true;
        assert_eq!(frame(&config).fill, Color32::TRANSPARENT);
        assert!(frame(&config).stroke.width > 0.0);
    }

    #[test]
    fn expanded_card_is_bounded_and_pet_and_compact_remain_small() {
        let mut config = Config::default();
        for row in &mut config.rows {
            row.enabled = true;
        }
        let rows = panel::rows(&config, &Snapshot::default());
        let full = size(&config, &rows, true, 600.0);
        assert_eq!(full.x, 240.0);
        assert!(full.y <= 520.0);
        config.compact = true;
        let compact = size(&config, &rows, false, 600.0);
        assert!(compact.y < full.y);
        config.compact = false;
        config.pet_mode = true;
        assert_eq!(size(&config, &rows, true, 600.0), egui::vec2(72.0, 84.0));
    }
}
