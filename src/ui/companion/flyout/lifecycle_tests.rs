use super::{Flyout, tests::services};
use crate::domain::{config::Config, resources::Snapshot};
use eframe::egui;
use std::time::Duration;

#[test]
fn leaving_a_closed_pet_rearms_hover_even_without_popup_geometry() {
    let mut flyout = Flyout::default();
    let mut services = services();
    let ctx = egui::Context::default();
    for cycle in 0..100 {
        let now = Duration::from_secs(cycle + 1);
        flyout.state.open(now);
        flyout.state.close();
        flyout.state.update(now, true, false, false);
        assert!(
            !flyout.active(),
            "closing must not reopen under the same hover"
        );
        assert_eq!(
            flyout.show(
                &ctx,
                None,
                &Config::default(),
                &Snapshot::default(),
                &mut services,
                false
            ),
            None
        );
        flyout.state.update(now, true, false, false);
        assert!(flyout.active(), "entry {cycle} must reopen after leaving");
    }
}

#[test]
fn dismiss_clears_pin_preview_review_and_regions_without_running_cleanup() {
    let mut flyout = Flyout::default();
    flyout.open_preview();
    flyout.last_regions.push([0.0, 0.0, 20.0, 20.0]);
    flyout.review.begin(&Config::default().cleanup);
    flyout.dismiss();
    assert!(!flyout.active());
    assert!(!flyout.pinned && !flyout.preview);
    assert!(flyout.review.operations().is_none());
    assert!(flyout.last_regions.is_empty());
    flyout.open();
    assert!(flyout.active() && flyout.pinned);
}
