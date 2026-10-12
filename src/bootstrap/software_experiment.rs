use crate::{
    infrastructure::windows::{fonts, presentation},
    ui::{development::software_preview::Preview, rendering::software::Painter},
};
use eframe::egui;
use std::{
    cell::RefCell,
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    rc::Rc,
    time::{Duration, Instant},
};

struct Log {
    file: File,
    rows: usize,
    last: Option<Instant>,
}

pub fn run(path: &Path) -> Result<(), String> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let log = Rc::new(RefCell::new(Log {
        file,
        rows: 0,
        last: None,
    }));
    let root = draw(true, log.clone())?;
    let settings = draw(false, log.clone())?;
    let result = presentation::run(root, settings);
    let mut log = log.borrow_mut();
    serde_json::to_writer(
        &mut log.file,
        &serde_json::json!({"event":"host-exited","result":result}),
    )
    .map_err(|e| e.to_string())?;
    writeln!(log.file).map_err(|e| e.to_string())?;
    result
}

fn draw(root: bool, log: Rc<RefCell<Log>>) -> Result<presentation::Draw, String> {
    let ctx = egui::Context::default();
    let font = fonts::load().ok_or("软件绘制实验：没有可用中文字体")?;
    let mut preview = Preview::new(&ctx, root, font);
    let mut painter = Painter::default();
    let started = Instant::now();
    Ok(Box::new(move |pixels, input| {
        let frame_start = Instant::now();
        let rect = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(input.size[0] as f32, input.size[1] as f32) / input.scale,
        );
        let mut raw = egui::RawInput {
            screen_rect: Some(rect),
            max_texture_side: Some(2048),
            time: Some(started.elapsed().as_secs_f64()),
            ..Default::default()
        };
        let viewport = raw
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .expect("默认根视口");
        viewport.native_pixels_per_point = Some(input.scale);
        viewport.inner_rect = Some(rect);
        viewport.maximized = Some(input.maximized);
        for event in input.events {
            match event {
                presentation::PointerEvent::Move([x, y]) => raw
                    .events
                    .push(egui::Event::PointerMoved(egui::pos2(x, y) / input.scale)),
                presentation::PointerEvent::Button([x, y], pressed) => {
                    raw.events.push(egui::Event::PointerButton {
                        pos: egui::pos2(x, y) / input.scale,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    })
                }
                presentation::PointerEvent::Leave => raw.events.push(egui::Event::PointerGone),
            }
        }
        let mut action = Default::default();
        let output = ctx.run(raw, |ctx| action = preview.show(ctx, input.maximized));
        painter.update(&output.textures_delta)?;
        let meshes = ctx.tessellate(output.shapes, output.pixels_per_point);
        painter.paint(pixels, input.size, output.pixels_per_point, &meshes)?;
        painter.free(&output.textures_delta);
        let delay = output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map(|v| v.repaint_delay);
        let elapsed = frame_start.elapsed();
        let mut log = log.borrow_mut();
        if log.rows < 120
            && (action.close
                || action.open_settings
                || action.maximize.is_some()
                || log
                    .last
                    .is_none_or(|last| last.elapsed() >= Duration::from_secs(1)))
        {
            let row = serde_json::json!({ "mode":"native-software-experiment", "root":root,
                "size":input.size,"scale":input.scale,"maximized":input.maximized,
                "surfaceBytes":pixels.len(),"textureBytes":painter.texture_bytes(),
                "primitiveCount":meshes.len(),"frameMilliseconds":elapsed.as_secs_f64()*1000.0,
                "actionClose":action.close,"actionOpenSettings":action.open_settings,"actionMaximize":action.maximize });
            serde_json::to_writer(&mut log.file, &row).map_err(|e| e.to_string())?;
            writeln!(log.file).map_err(|e| e.to_string())?;
            log.rows += 1;
            log.last = Some(Instant::now());
        }
        Ok(presentation::FrameAction {
            maximize: action.maximize,
            close: action.close,
            open_settings: action.open_settings,
            drag: action.drag,
            repaint_after: delay.filter(|delay| *delay < Duration::from_secs(3600)),
        })
    }))
}
