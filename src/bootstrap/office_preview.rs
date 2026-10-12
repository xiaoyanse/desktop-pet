use crate::{
    application::office::OfficeService,
    domain::office::{Book, Command, Kind},
    interfaces::OfficeStore,
    ui::development::office_preview::Preview,
};
use eframe::egui;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};
struct Memory(Mutex<Book>);
const STAGES: &[&str] = &[
    "notes.png",
    "tasks.png",
    "task-editor.png",
    "reminder.png",
    "components.png",
    "metrics.png",
    "devices.png",
    "meeting.png",
    "meeting-overtime.png",
    "meeting-unavailable.png",
    "meeting-reminder.png",
    "control-icons.png",
];
impl OfficeStore for Memory {
    fn load(&self) -> Result<Book, String> {
        Ok(self.0.lock().map_err(|_| "合成数据锁异常")?.clone())
    }
    fn save(&self, book: &Book, previous: &Book) -> Result<(), String> {
        let mut current = self.0.lock().map_err(|_| "合成数据锁异常")?;
        if *current != *previous {
            return Err("合成数据变化".into());
        }
        *current = book.clone();
        Ok(())
    }
    fn location(&self) -> String {
        "合成预览 · 只在内存中保存".into()
    }
}
pub fn run(folder: &Path) -> Result<(), String> {
    if !folder.is_absolute() {
        return Err("合成预览需要E盘等指定绝对目录".into());
    }
    std::fs::create_dir(folder).map_err(|e| format!("需要尚不存在的输出目录：{e}"))?;
    let font = crate::infrastructure::windows::fonts::load().ok_or("未找到中文字体")?;
    let mut book = Book::default();
    for (kind, title, body, due) in [
        (
            Kind::Note,
            "开发现场的下一步",
            "保留需求、验证命令与交接线索。\n文字只保存在本地。",
            None,
        ),
        (
            Kind::Task,
            "核对这一阶段的交付",
            "检查功能、记录与阶段提交。",
            Some(crate::application::office::now().saturating_sub(60)),
        ),
        (
            Kind::Task,
            "整理明天的工作安排",
            "完成之后再开始下一项。",
            Some(crate::application::office::now() + 3600),
        ),
    ] {
        book = book.changed(
            Command::Save {
                id: None,
                kind,
                title: title.into(),
                body: body.into(),
                pinned: false,
                due,
            },
            1,
        )?;
    }
    let folder = folder.to_path_buf();
    let error = Arc::new(Mutex::new(None));
    let app_error = error.clone();
    eframe::run_native(
        "DesktopMeterOfficePreview",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("桌面仪表 · 便签合成预览")
                .with_decorations(false)
                .with_taskbar(false)
                .with_transparent(true)
                .with_active(false)
                .with_inner_size([1040.0, 820.0])
                .with_position([80.0, 60.0]),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            let wake = Arc::new(move || ctx.request_repaint());
            let service = OfficeService::new(
                Arc::new(Memory(Mutex::new(book))),
                Arc::new(crate::infrastructure::windows::local_time::WindowsLocalTime),
                wake,
            );
            Ok(Box::new(App {
                preview: Preview::new(&cc.egui_ctx, font),
                service,
                folder,
                stage: 0,
                ready: Instant::now() + Duration::from_millis(500),
                capture: None,
                error: app_error,
                started: Instant::now(),
            }))
        }),
    )
    .map_err(|e| e.to_string())?;
    if let Some(error) = error.lock().map_err(|_| "合成预览状态锁异常")?.clone() {
        Err(error)
    } else {
        Ok(())
    }
}
struct App {
    preview: Preview,
    service: OfficeService,
    folder: PathBuf,
    stage: usize,
    ready: Instant,
    capture: Option<mpsc::Receiver<Result<(), String>>>,
    error: Arc<Mutex<Option<String>>>,
    started: Instant,
}
impl eframe::App for App {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if self.stage >= STAGES.len() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if self.started.elapsed() > Duration::from_secs(20) {
            *self.error.lock().unwrap() = Some("合成预览超时".into());
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        self.service.tick(crate::application::office::now());
        if let Some(rx) = &self.capture {
            match rx.try_recv() {
                Ok(Ok(())) => {
                    self.capture = None;
                    self.stage += 1;
                    if self.stage == STAGES.len() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        return;
                    }
                    self.preview.stage(self.stage, &self.service);
                    if self.stage == 3 {
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                            360.0, 296.0,
                        )));
                    } else if self.stage == 4 || self.stage == 11 {
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                            1040.0, 820.0,
                        )));
                    } else if self.stage == 10 {
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                            360.0, 250.0,
                        )));
                    }
                    self.ready = Instant::now() + Duration::from_millis(300);
                }
                Ok(Err(error)) => {
                    *self.error.lock().unwrap() = Some(error);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    return;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    *self.error.lock().unwrap() = Some("自有窗口输出通道中断".into());
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    return;
                }
            }
        }
        self.preview.show(ctx, &mut self.service);
        if self.service.ready && self.capture.is_none() && Instant::now() >= self.ready {
            let path = self.folder.join(STAGES[self.stage]);
            let (tx, rx) = mpsc::channel();
            self.capture = Some(rx);
            ctx.layer_painter(egui::LayerId::new(
                egui::Order::Debug,
                egui::Id::new("own-office-buffer"),
            ))
            .add(egui::PaintCallback {
                rect: ctx.screen_rect(),
                callback: Arc::new(eframe::egui_glow::CallbackFn::new(move |_, painter| {
                    let _ = tx.send(save_own_framebuffer(painter.gl(), &path));
                })),
            });
        }
        ctx.request_repaint_after(Duration::from_millis(40));
    }
}
impl Drop for App {
    fn drop(&mut self) {
        let mut error = self.error.lock().unwrap();
        if self.stage < STAGES.len() && error.is_none() {
            *error = Some("合成预览未完成全部阶段".into());
        }
        let result = serde_json::json!({"pid":std::process::id(),"completedStages":self.stage,"complete":self.stage==STAGES.len(),"error":*error,"realDesktopCaptured":false,"persistentUserDataChanged":false});
        let _ = std::fs::write(self.folder.join("status.json"), result.to_string());
    }
}
fn save_own_framebuffer(gl: &eframe::glow::Context, path: &Path) -> Result<(), String> {
    use eframe::glow::{self, HasContext};
    let mut viewport = [0; 4];
    unsafe {
        gl.get_parameter_i32_slice(glow::VIEWPORT, &mut viewport);
    }
    let [_, _, w, h] = viewport;
    let len = (w.max(0) as usize)
        .checked_mul(h.max(0) as usize)
        .and_then(|p| p.checked_mul(4))
        .filter(|b| *b > 0 && *b <= 16 * 1024 * 1024)
        .ok_or("自有窗口缓冲超过16 MiB上限")?;
    let mut pixels = vec![0; len];
    unsafe {
        gl.read_pixels(
            viewport[0],
            viewport[1],
            w,
            h,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelPackData::Slice(Some(&mut pixels)),
        );
        if gl.get_error() != 0 {
            return Err("读取自有GL缓冲失败".into());
        }
    }
    let row = w as usize * 4;
    let mut rgba = vec![0; len];
    for y in 0..h as usize {
        rgba[y * row..(y + 1) * row]
            .copy_from_slice(&pixels[(h as usize - y - 1) * row..(h as usize - y) * row]);
    }
    for p in rgba.chunks_exact_mut(4) {
        if p[3] > 0 {
            for i in 0..3 {
                p[i] = ((p[i] as u32 * 255 / p[3] as u32).min(255)) as u8;
            }
        }
    }
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(file, w as u32, h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .map_err(|e| e.to_string())?
        .write_image_data(&rgba)
        .map_err(|e| e.to_string())
}
