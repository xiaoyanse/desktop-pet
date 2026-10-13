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
    if args.get(1).is_some_and(|arg| {
        arg == "--make-update-delta"
            || arg == "--update-delta-probe"
            || arg == "--apply-update-delta"
            || arg == "--update-delta-fallback-probe"
    }) {
        let result = (|| -> Result<(), String> {
            if args.len() != 5 {
                return Err("差分工具参数数量无效".into());
            }
            if args[1] == "--make-update-delta" {
                infrastructure::update::make_delta(
                    std::path::Path::new(&args[2]),
                    std::path::Path::new(&args[3]),
                    std::path::Path::new(&args[4]),
                )
            } else if args[1] == "--apply-update-delta" {
                infrastructure::update::apply_delta(
                    std::path::Path::new(&args[2]),
                    std::path::Path::new(&args[3]),
                    std::path::Path::new(&args[4]),
                )
            } else {
                let report = infrastructure::update::delta_probe(
                    &args[2],
                    std::path::Path::new(&args[3]),
                    args[1] == "--update-delta-fallback-probe",
                )?;
                use std::io::Write;
                std::fs::File::options()
                    .write(true)
                    .create_new(true)
                    .open(&args[4])
                    .and_then(|mut file| {
                        file.write_all(&serde_json::to_vec_pretty(&report).unwrap())
                    })
                    .map_err(|_| "增量探针输出不可写".into())
            }
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if args
        .get(1)
        .is_some_and(|arg| arg == "--update-check-probe" || arg == "--update-download-probe")
    {
        let result = (|| {
            let source = args.get(2).ok_or("更新探针需要 GitHub 或 Gitee 来源")?;
            let output = args
                .get(3)
                .map(std::path::PathBuf::from)
                .ok_or("更新探针需要新的输出文件")?;
            if !output.is_absolute() {
                return Err("更新探针输出必须为绝对路径".to_string());
            }
            let report =
                infrastructure::update::probe(source, args[1] == "--update-download-probe")?;
            let data = serde_json::to_vec_pretty(&report).map_err(|_| "更新探针结果无法编码")?;
            use std::io::Write;
            std::fs::File::options()
                .write(true)
                .create_new(true)
                .open(&output)
                .and_then(|mut file| file.write_all(&data))
                .map_err(|_| "更新探针输出不可写".to_string())
                .map(|_| ())
        })();
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if args.get(1).is_some_and(|a| a == "--apply-update") {
        if let Err(error) = infrastructure::update::helper(&args[2..]) {
            if args.iter().any(|arg| arg == "--quiet-helper") {
                eprintln!("{error}");
            } else {
                native::message(&error);
            }
            std::process::exit(1);
        }
        return;
    }
    if args.get(1).is_some_and(|a| a == "--microphone-probe") {
        let result = args
            .get(2)
            .ok_or("麦克风探针需要新文件的绝对路径".to_string())
            .and_then(|path| bootstrap::microphone_probe::run(std::path::Path::new(path)));
        if let Err(error) = result {
            native::message(&error);
            std::process::exit(1);
        }
        return;
    }
    if args.get(1).is_some_and(|a| a == "--office-preview") {
        let Some(folder) = args.get(2) else {
            native::message("合成预览需要尚不存在的绝对输出目录");
            return;
        };
        if let Err(error) = bootstrap::office_preview::run(std::path::Path::new(folder)) {
            native::message(&error);
            std::process::exit(1);
        }
        return;
    }
    if let Some(index) = args
        .iter()
        .position(|a| a == "--screenshot-lifecycle-probe")
    {
        let Some(path) = args.get(index + 1).filter(|p| !p.starts_with("--")) else {
            native::message("生命周期验证需要新日志的绝对路径");
            return;
        };
        if let Err(e) = bootstrap::screenshot_lifecycle::run(
            std::path::Path::new(path),
            args.iter().any(|a| a == "--blank-root"),
        ) {
            native::message(&e);
            std::process::exit(1);
        }
        return;
    }
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
    let update_ack = args
        .iter()
        .position(|a| a == "--update-ack")
        .and_then(|index| args.get(index + 1))
        .map(std::path::PathBuf::from);
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
    if update_ack.is_none() && !args.iter().any(|arg| arg.starts_with("--")) {
        match infrastructure::update::pending() {
            Ok(true) => return,
            Ok(false) => {}
            Err(error) => native::message(&error),
        }
    }
    let (config, warning) = JsonSettings.load();
    let snapping = config.snap_edges;
    let dependencies = (!renderer_baseline).then(|| application::Dependencies {
        updates: Some(std::sync::Arc::new(
            infrastructure::update::RepositoryUpdater::new(update_ack),
        )),
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
        screen_capture: std::sync::Arc::new(infrastructure::windows::screenshot::WindowsCapture),
        screenshot_output: std::sync::Arc::new(
            infrastructure::storage::screenshots::LocalScreenshots,
        ),
        office_store: std::sync::Arc::new(
            infrastructure::storage::office::LocalOfficeStore::default(),
        ),
        local_time: std::sync::Arc::new(infrastructure::windows::local_time::WindowsLocalTime),
        microphone: std::sync::Arc::new(infrastructure::windows::microphone::WindowsMicrophone),
        clipboard: std::sync::Arc::new(infrastructure::windows::clipboard::WindowsClipboard),
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
