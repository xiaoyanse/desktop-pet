#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod application;
mod assets;
mod bootstrap;
mod domain;
mod infrastructure;
#[cfg(test)]
mod integration_tests;
mod interfaces;
mod ui;

use domain::config;
use infrastructure::{
    storage::settings::{JsonSettings, data_dir},
    windows::{
        desktop::WindowsDesktop, diagnostics, memory::cleanup, monitoring::sampler::Sampler, native,
    },
};
use interfaces::{ReportCollector, ResourceSampler, SettingsStore};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "--report-probe") {
        let path = args
            .get(2)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| data_dir().join("report-probe.md"));
        let result = infrastructure::windows::report::WindowsReportCollector
            .collect(
                &JsonSettings.load().0,
                &std::sync::atomic::AtomicBool::new(false),
            )
            .and_then(|report| {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                std::fs::write(&path, report.markdown()).map_err(|e| e.to_string())
            });
        if let Err(e) = result {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    if args.get(1).is_some_and(|a| a == "--inspect") {
        let path = args
            .get(2)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| data_dir().join("window-report.json"));
        if let Err(e) = diagnostics::inspect(&path) {
            native::message(&e);
        }
        return;
    }
    if args.get(1).is_some_and(|a| a == "--clean-helper") {
        cleanup::helper(&args[2..]);
        return;
    }
    if args.get(1).is_some_and(|a| a == "--probe") {
        let mut sampler = Sampler::new();
        let config = config::Config {
            rows: config::Metric::ALL
                .into_iter()
                .map(|metric| config::Row {
                    metric,
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        sampler.sample(&config, true);
        std::thread::sleep(std::time::Duration::from_secs(2));
        let sample = sampler.sample(&config, true);
        let path = args
            .get(2)
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| data_dir().join("probe.json"));
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(path, serde_json::to_vec_pretty(&sample).unwrap()) {
            eprintln!("{e}");
        }
        return;
    }
    if args.iter().any(|a| a == "--quit") {
        native::request_instance(false, true);
        return;
    }
    let open_settings = args.iter().any(|a| a == "--settings");
    if let Some(index) = args.iter().position(|a| a == "--software-render-probe") {
        let Some(path) = args.get(index + 1).filter(|path| !path.starts_with("--")) else {
            native::message("软件绘制实验需要一个尚不存在的输出文件路径");
            return;
        };
        let Some(_instance) = native::single_instance(false) else {
            return;
        };
        if let Err(error) = bootstrap::software_experiment::run(std::path::Path::new(path)) {
            native::message(&error);
        }
        return;
    }
    let resource_probe = if let Some(index) = args.iter().position(|a| a == "--ui-resource-probe") {
        let Some(path) = args.get(index + 1).filter(|path| !path.starts_with("--")) else {
            native::message("开发资源探针需要一个尚不存在的输出文件路径");
            return;
        };
        Some(std::path::PathBuf::from(path))
    } else {
        None
    };
    let renderer_baseline = args.iter().any(|a| a == "--renderer-baseline");
    if renderer_baseline && resource_probe.is_none() {
        native::message("渲染基线仅用于开发取证，必须显式提供 --ui-resource-probe 输出文件");
        return;
    }
    let Some(_instance) = native::single_instance(open_settings) else {
        return;
    };
    let (config, warning) = JsonSettings.load();
    let snapping = config.snap_edges;
    let dependencies = (!renderer_baseline).then(|| application::Dependencies {
        sampler: Box::new(|| Box::new(Sampler::new())),
        settings: Box::new(JsonSettings),
        cleaner: Box::new(cleanup::WindowsCleaner),
        cleanup_history: Box::new(infrastructure::storage::cleanup_history::JsonCleanupHistory),
        desktop: Box::new(move |hwnd, wake| Box::new(WindowsDesktop::attach(hwnd, snapping, wake))),
        font: infrastructure::windows::fonts::load(),
        report_collector: std::sync::Arc::new(
            infrastructure::windows::report::WindowsReportCollector,
        ),
        report_store: std::sync::Arc::new(infrastructure::storage::reports::LocalReports),
    });
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("桌面资源监视器")
        .with_decorations(false)
        .with_taskbar(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_inner_size([240.0, 280.0])
        .with_resizable(false);
    viewport = viewport.with_icon(eframe::egui::IconData {
        rgba: include_bytes!(concat!(env!("OUT_DIR"), "/icon-64.rgba")).to_vec(),
        width: 64,
        height: 64,
    });
    if let Some(p) = config.position {
        viewport = viewport.with_position(p);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        vsync: true,
        multisampling: 0,
        ..Default::default()
    };
    if let Err(e) = eframe::run_native(
        "DesktopMeter",
        options,
        Box::new(move |cc| {
            if let Some(path) = resource_probe {
                bootstrap::resource_probe::install(&cc.egui_ctx, &path, cc.gl.as_deref())?;
            }
            if renderer_baseline {
                return Ok(Box::new(bootstrap::render_baseline::RenderBaseline::new(
                    &cc.egui_ctx,
                    open_settings,
                )));
            }
            let mut app = ui::monitor::MonitorApp::new(
                cc,
                config,
                warning,
                dependencies.expect("普通启动必须装配应用依赖"),
            );
            if open_settings {
                app.open_settings();
            }
            Ok(Box::new(app))
        }),
    ) {
        native::message(&format!("启动失败：{e}"));
    }
}
