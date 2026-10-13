use super::*;
use crate::{
    domain::update::{Manifest, Release},
    interfaces::update::UpdateBackend,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};

struct Fake {
    checks: AtomicUsize,
    schedules: AtomicUsize,
}
impl UpdateBackend for Fake {
    fn check(&self, _: &Preferences) -> Result<Option<Release>, String> {
        self.checks.fetch_add(1, Ordering::Relaxed);
        Ok(None)
    }
    fn download(&self, _: &Release, _: &AtomicBool, _: &AtomicU64) -> Result<(), String> {
        Ok(())
    }
    fn launch_install(&self) -> Result<(), String> {
        Ok(())
    }
    fn schedule_install(&self) -> Result<(), String> {
        self.schedules.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}
fn paint(
    ctx: &egui::Context,
    prefs: &mut Preferences,
    service: &mut UpdateService,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(700.0, 600.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(ui, prefs, service);
            });
        },
    )
}
fn click(ctx: &egui::Context, prefs: &mut Preferences, service: &mut UpdateService, label: &str) {
    let output = paint(ctx, prefs, service, vec![]);
    let position = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                Some(text.pos + text.galley.size() * 0.5)
            }
            _ => None,
        })
        .expect("production action visible");
    assert!(position.x >= 0.0 && position.x <= 700.0 && position.y >= 0.0 && position.y <= 600.0);
    for pressed in [true, false] {
        paint(
            ctx,
            prefs,
            service,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        );
    }
}
fn setup() -> (egui::Context, Preferences, UpdateService, Arc<Fake>) {
    let ctx = egui::Context::default();
    crate::ui::common::theme::setup(&ctx);
    let backend = Arc::new(Fake {
        checks: AtomicUsize::new(0),
        schedules: AtomicUsize::new(0),
    });
    let service = UpdateService::new(backend.clone(), Arc::new(|| {}));
    (ctx, Preferences::default(), service, backend)
}
#[test]
fn fixed_version_disables_real_check_button_and_manual_check_clears_skip() {
    let (ctx, mut prefs, mut service, backend) = setup();
    prefs.pinned = true;
    click(&ctx, &mut prefs, &mut service, "检查更新");
    assert!(!service.busy());
    assert_eq!(backend.checks.load(Ordering::Relaxed), 0);
    prefs.pinned = false;
    prefs.skipped = "99.0.0".into();
    click(&ctx, &mut prefs, &mut service, "检查更新");
    assert!(service.busy());
    assert!(prefs.skipped.is_empty());
}
#[test]
fn ready_page_routes_immediate_install_intent_and_deferred_install_separately() {
    let (ctx, mut prefs, mut service, backend) = setup();
    service.state = State::Ready(Release {
        version: "0.3.0".into(),
        notes: String::new(),
        url: String::new(),
        page: String::new(),
        source: "GitHub".into(),
        manifest: Manifest {
            schema: 1,
            version: "0.3.0".into(),
            file_name: "DesktopMeter-0.3.0-win-x64.zip".into(),
            size: 100,
            sha256: "a".repeat(64),
            executable_sha256: "b".repeat(64),
        },
    });
    click(&ctx, &mut prefs, &mut service, "下次启动安装");
    assert!(service.scheduled);
    assert_eq!(backend.schedules.load(Ordering::Relaxed), 1);
    assert!(!service.install_requested);
    click(&ctx, &mut prefs, &mut service, "保存并安装");
    assert!(service.install_requested);
}
