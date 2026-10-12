use eframe::egui;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    fs::OpenOptions,
    io::Write,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, IsWindow, IsWindowVisible, SW_SHOWNOACTIVATE, ShowWindowAsync,
    WS_EX_TRANSPARENT,
};

pub fn run(path: &Path, blank: bool) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("开发日志需要绝对路径".into());
    }
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("日志无法创建（不覆盖已有文件）：{e}"))?;
    let log = Arc::new(Mutex::new(file));
    eframe::run_native(
        "DesktopMeterScreenshotLifecycle",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("截图生命周期验证 · 合成窗口")
                .with_decorations(false)
                .with_transparent(true)
                .with_taskbar(false)
                .with_inner_size([72.0, 84.0])
                .with_position([16.0, 16.0])
                .with_resizable(false),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| {
            let hwnd = cc
                .window_handle()
                .ok()
                .and_then(|h| match h.as_raw() {
                    RawWindowHandle::Win32(h) => Some(h.hwnd.get()),
                    _ => None,
                })
                .ok_or("无法读取自有验证窗口句柄")?;
            Ok(Box::new(Probe {
                log,
                blank,
                hwnd,
                frames: 0,
                started: Instant::now(),
                ready: None,
                finish: None,
                stop: Arc::new(AtomicBool::new(false)),
                watchdog_fired: Arc::new(AtomicBool::new(false)),
                watchdog: None,
            }))
        }),
    )
    .map_err(|e| e.to_string())
}
struct Probe {
    log: Arc<Mutex<std::fs::File>>,
    blank: bool,
    hwnd: isize,
    frames: u32,
    started: Instant,
    ready: Option<Instant>,
    finish: Option<Instant>,
    stop: Arc<AtomicBool>,
    watchdog_fired: Arc<AtomicBool>,
    watchdog: Option<std::thread::JoinHandle<()>>,
}
fn record(log: &Mutex<std::fs::File>, value: serde_json::Value) {
    if let Ok(mut file) = log.lock() {
        let _ = writeln!(file, "{value}");
        let _ = file.flush();
    }
}
impl Probe {
    fn begin(&mut self, ctx: &egui::Context) {
        self.ready = Some(Instant::now() + Duration::from_millis(200));
        record(
            &self.log,
            serde_json::json!({"event":"prepare","pid":std::process::id(),"strategy":if self.blank{"transparent-paint"}else{"hidden-window"},"frames":self.frames}),
        );
        let stop = self.stop.clone();
        let fired = self.watchdog_fired.clone();
        let hwnd = self.hwnd;
        let log = self.log.clone();
        let watchdog_ctx = ctx.clone();
        let watchdog = std::thread::Builder::new()
            .name("own-window-lifecycle-watchdog".into())
            .spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(3);
                while !stop.load(Ordering::Relaxed) && Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(10));
                }
                if stop.load(Ordering::Relaxed) {
                    return;
                }
                fired.store(true, Ordering::Relaxed);
                record(
                    &log,
                    serde_json::json!({"event":"watchdog-restored-own-window"}),
                );
                unsafe {
                    if IsWindow(hwnd as _) != 0 {
                        ShowWindowAsync(hwnd as _, SW_SHOWNOACTIVATE);
                    }
                }
                watchdog_ctx.request_repaint();
            });
        match watchdog {
            Ok(worker) => {
                self.watchdog = Some(worker);
                if !self.blank {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                } else {
                    crate::ui::monitor::suspend_screenshot_root(ctx);
                }
            }
            Err(error) => {
                record(
                    &self.log,
                    serde_json::json!({"event":"watchdog-start-failed","error":error.to_string()}),
                );
                self.ready = None;
                self.finish = Some(Instant::now());
            }
        }
    }
    fn sample_own_framebuffer(&self, ctx: &egui::Context, phase: &'static str) {
        let log = self.log.clone();
        ctx.layer_painter(egui::LayerId::background()).add(egui::PaintCallback{rect:ctx.screen_rect(),callback:Arc::new(eframe::egui_glow::CallbackFn::new(move |_,painter|{
            use eframe::glow::{self,HasContext};
            let gl=painter.gl();let mut viewport=[0;4];
            unsafe {
                gl.get_parameter_i32_slice(glow::VIEWPORT,&mut viewport);
                let w=viewport[2].clamp(1,128);let h=viewport[3].clamp(1,128);
                let mut pixels=vec![0;(w*h*4) as usize];
                gl.read_pixels(viewport[0],viewport[1],w,h,glow::RGBA,glow::UNSIGNED_BYTE,glow::PixelPackData::Slice(Some(&mut pixels)));
                let error=gl.get_error();
                record(&log,serde_json::json!({"event":"own-framebuffer","phase":phase,"size":[w,h],"nonzeroAlphaPixels":pixels.chunks_exact(4).filter(|p|p[3]!=0).count(),"glError":error}));
            }
        }))});
    }
}
impl eframe::App for Probe {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.frames += 1;
        if self.frames == 1 {
            ctx.layer_painter(egui::LayerId::background()).rect_filled(
                egui::Rect::from_min_size(egui::pos2(8.0, 8.0), egui::vec2(52.0, 64.0)),
                12,
                egui::Color32::from_rgb(140, 200, 175),
            );
            ctx.request_repaint_after(Duration::from_millis(100));
            return;
        }
        if self.ready.is_none() {
            self.begin(ctx);
        }
        if self.blank && self.finish.is_none() {
            self.sample_own_framebuffer(ctx, "suspended");
        }
        if self.finish.is_none() && self.ready.is_some_and(|ready| Instant::now() >= ready) {
            self.stop.store(true, Ordering::Relaxed);
            record(
                &self.log,
                serde_json::json!({"event":"delayed-step-reached","frames":self.frames,"elapsedMs":self.started.elapsed().as_millis(),"watchdogFired":self.watchdog_fired.load(Ordering::Relaxed),"windowVisible":unsafe{IsWindowVisible(self.hwnd as _)!=0},"mousePassthrough":unsafe{GetWindowLongPtrW(self.hwnd as _,GWL_EXSTYLE) as u32 & WS_EX_TRANSPARENT !=0}}),
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            if self.blank {
                crate::ui::monitor::restore_screenshot_root(ctx, false);
            }
            self.finish = Some(Instant::now() + Duration::from_millis(100));
        }
        if self.finish.is_some() {
            ctx.layer_painter(egui::LayerId::background()).rect_filled(
                egui::Rect::from_min_size(egui::pos2(8.0, 8.0), egui::vec2(52.0, 64.0)),
                12,
                egui::Color32::from_rgb(140, 200, 175),
            );
            if self.blank {
                self.sample_own_framebuffer(ctx, "restored");
            }
        }
        if self.finish.is_some_and(|at| Instant::now() >= at) {
            record(
                &self.log,
                serde_json::json!({"event":"complete","frames":self.frames,"watchdogFired":self.watchdog_fired.load(Ordering::Relaxed),"mousePassthrough":unsafe{GetWindowLongPtrW(self.hwnd as _,GWL_EXSTYLE) as u32 & WS_EX_TRANSPARENT!=0}}),
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(40));
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.watchdog.take() {
            let _ = worker.join();
        }
    }
}
