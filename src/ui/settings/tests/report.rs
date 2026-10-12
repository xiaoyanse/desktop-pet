use crate::domain::config::Config;
use crate::ui::{common::theme, work_tools::diagnostic_report::Page};
use crate::{
    application::report::{Document, ReportService},
    domain::report::{Process, Report, privacy::SafePath},
    interfaces::{ReportCollector, ReportStore},
};
use eframe::egui;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

struct Fake(AtomicUsize);
impl ReportCollector for Fake {
    fn collect(&self, _: &Config, _: &AtomicBool) -> Result<Report, String> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(Report::default())
    }
}
impl ReportStore for Fake {
    fn save(&self, _: &str) -> Result<String, String> {
        panic!("copying must never save")
    }
}
fn paint(
    ctx: &egui::Context,
    page: &mut Page,
    service: &mut ReportService,
    size: egui::Vec2,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            events,
            ..Default::default()
        },
        |ctx| {
            crate::ui::settings::shell::window_chrome::show(ctx);
            crate::ui::settings::shell::navigation::navigation(ctx, 7, &Config::default());
            egui::CentralPanel::default()
                .frame(egui::Frame::new().inner_margin(28))
                .show(ctx, |ui| {
                    crate::ui::settings::shell::navigation::header(
                        ui,
                        7,
                        &Config::default(),
                        &Default::default(),
                    );
                    page.show(ui, &Config::default(), service);
                });
        },
    )
}
fn position(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|s| {
            if let egui::Shape::Text(t) = &s.shape
                && t.galley.text() == label
            {
                let r = egui::Rect::from_min_size(t.pos, t.galley.size());
                assert!(
                    s.clip_rect.contains_rect(r),
                    "action {label} clipped at {r:?}, clip {:?}",
                    s.clip_rect
                );
                Some(r.center())
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("missing action {label}"))
}
#[test]
fn fixed_actions_stay_visible_and_copy_exports_the_exact_frozen_preview() {
    for size in [egui::vec2(1040.0, 820.0), egui::vec2(840.0, 600.0)] {
        let ctx = egui::Context::default();
        theme::setup(&ctx);
        let backend = Arc::new(Fake(AtomicUsize::new(0)));
        let mut service = ReportService::new(backend.clone(), backend.clone(), Arc::new(|| {}));
        let mut page = Page::default();
        let mut report = Report {
            captured_at: "fixture".into(),
            ..Default::default()
        };
        report.resources = vec![
            (crate::domain::config::Metric::Cpu, "27.2%".into()),
            (crate::domain::config::Metric::Memory, "83.5%".into()),
            (crate::domain::config::Metric::Paged, "4.10 GB".into()),
            (crate::domain::config::Metric::Nonpaged, "10.40 GB".into()),
        ];
        for pid in 1..=43 {
            report.runtimes.push(Process {
                name: "node.exe".into(),
                pid,
                parent: None,
                path: SafePath::from_raw("E:\\敏感项目\\node.exe"),
                cpu: None,
                working_set: None,
                private: None,
            });
        }
        let expected = report.markdown();
        service.document = Some(Document {
            report,
            markdown: expected.clone(),
        });
        let mut output = paint(&ctx, &mut page, &mut service, size, vec![]);
        for _ in 0..2 {
            output = paint(&ctx, &mut page, &mut service, size, vec![]);
        }
        position(&output, "重新生成");
        assert!(
            position(&output, "10.40 GB").x - position(&output, "27.2%").x
                > (size.x - 340.0) * 0.55,
            "resource columns must fill the available card width"
        );
        for (label, value) in [
            ("CPU 占用", "27.2%"),
            ("内存占用", "83.5%"),
            ("分页池", "4.10 GB"),
            ("非分页池", "10.40 GB"),
        ] {
            assert!(
                position(&output, value).y - position(&output, label).y > 15.0,
                "resource label and value must use separate lines: {label}"
            );
        }
        position(&output, "保存到本地");
        let point = position(&output, "复制报告");
        for pressed in [true, false] {
            output = paint(
                &ctx,
                &mut page,
                &mut service,
                size,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    },
                ],
            );
        }
        assert!(output.platform_output.commands.iter().any(
            |command| matches!(command,egui::OutputCommand::CopyText(text) if text==&expected)
        ));
        assert!(!expected.contains("敏感项目"));
        assert_eq!(backend.0.load(Ordering::SeqCst), 0);
        assert!(!service.busy());
    }
}
