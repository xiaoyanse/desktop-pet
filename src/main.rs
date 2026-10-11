#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod cleanup;
mod config;
mod docking;
mod diagnostics;
mod gpu;
mod metrics;
mod model;
mod native;
mod pdh;
mod panel;
mod processes;
mod settings;
mod tray;
mod tooltip;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "--inspect") {
        let path = args.get(2).map(std::path::PathBuf::from).unwrap_or_else(|| config::data_dir().join("window-report.json"));
        if let Err(e) = diagnostics::inspect(&path) { native::message(&e); }
        return;
    }
    if args.get(1).is_some_and(|a| a == "--clean-low-priority") {
        cleanup::helper(&args[2..]); return;
    }
    if args.get(1).is_some_and(|a| a == "--probe") {
        let mut sampler = metrics::Sampler::new();
        let config = config::Config { rows: config::Metric::ALL.into_iter().map(|metric| config::Row { metric, ..Default::default() }).collect(), ..Default::default() };
        sampler.sample(&config, true);
        std::thread::sleep(std::time::Duration::from_secs(2));
        let sample = sampler.sample(&config, true);
        let path = args.get(2).map(std::path::PathBuf::from).unwrap_or_else(|| config::data_dir().join("probe.json"));
        if let Some(parent) = path.parent() { let _ = std::fs::create_dir_all(parent); }
        if let Err(e) = std::fs::write(path, serde_json::to_vec_pretty(&sample).unwrap()) { eprintln!("{e}"); }
        return;
    }
    let Some(_instance) = native::single_instance() else { return; };
    let (config, warning) = config::Config::load();
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("桌面资源监视器").with_decorations(false).with_transparent(true)
        .with_always_on_top().with_inner_size([240.0, 280.0]).with_resizable(false);
    if let Some(p) = config.position { viewport = viewport.with_position(p); }
    let options = eframe::NativeOptions { viewport, renderer: eframe::Renderer::Glow,
        vsync: true, multisampling: 0, ..Default::default() };
    if let Err(e) = eframe::run_native("DesktopMeter", options, Box::new(move |cc| {
        Ok(Box::new(app::MonitorApp::new(cc, config, warning)))
    })) { native::message(&format!("启动失败：{e}")); }
}
