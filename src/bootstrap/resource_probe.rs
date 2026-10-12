use eframe::egui;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use windows_sys::Win32::System::{
    ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX},
    Threading::GetCurrentProcess,
};

const MAX_ROWS: usize = 120;

struct Probe {
    file: File,
    started: Instant,
    last: Option<Instant>,
    rows: usize,
    renderer: Option<serde_json::Value>,
}

pub fn install(
    ctx: &egui::Context,
    path: &Path,
    gl: Option<&eframe::glow::Context>,
) -> std::io::Result<()> {
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let probe = Arc::new(Mutex::new(Probe {
        file,
        started: Instant::now(),
        last: None,
        rows: 0,
        renderer: gl.map(renderer_metadata),
    }));
    ctx.on_end_pass(
        "bounded-ui-resource-probe",
        Arc::new(move |ctx| {
            if let Ok(mut probe) = probe.lock() {
                probe.record(ctx);
            }
        }),
    );
    Ok(())
}

fn renderer_metadata(gl: &eframe::glow::Context) -> serde_json::Value {
    use eframe::glow::{self, HasContext};
    // CreationContext guarantees a current root GL context; never inspect another app's GL state.
    unsafe {
        let default_framebuffer = gl.get_parameter_i32(glow::FRAMEBUFFER_BINDING) == 0;
        let double_buffered = gl.get_parameter_i32(glow::DOUBLEBUFFER) != 0;
        let color = if double_buffered {
            glow::BACK_LEFT
        } else {
            glow::FRONT_LEFT
        };
        let bits = |attachment, parameter| {
            default_framebuffer.then(|| {
                gl.get_framebuffer_attachment_parameter_i32(
                    glow::FRAMEBUFFER,
                    attachment,
                    parameter,
                )
            })
        };
        let mut metadata = serde_json::json!({
            "scope":"creation-root-default-framebuffer", "defaultFramebufferBound":default_framebuffer,
            "renderer":gl.get_parameter_string(glow::RENDERER),
            "version":gl.get_parameter_string(glow::VERSION),
            "doubleBuffered":double_buffered,"samples":gl.get_parameter_i32(glow::SAMPLES),
            "redBits":bits(color,glow::FRAMEBUFFER_ATTACHMENT_RED_SIZE),
            "alphaBits":bits(color,glow::FRAMEBUFFER_ATTACHMENT_ALPHA_SIZE),
            "depthBits":bits(glow::DEPTH,glow::FRAMEBUFFER_ATTACHMENT_DEPTH_SIZE),
            "stencilBits":bits(glow::STENCIL,glow::FRAMEBUFFER_ATTACHMENT_STENCIL_SIZE)
        });
        metadata["queryError"] = serde_json::json!(gl.get_error());
        metadata
    }
}

impl Probe {
    fn record(&mut self, ctx: &egui::Context) {
        if self.rows >= MAX_ROWS
            || self
                .last
                .is_some_and(|last| last.elapsed() < Duration::from_secs(1))
        {
            return;
        }
        let mut memory = PROCESS_MEMORY_COUNTERS_EX::default();
        memory.cb = std::mem::size_of_val(&memory) as u32;
        let measured = unsafe {
            GetProcessMemoryInfo(
                GetCurrentProcess(),
                &mut memory as *mut _ as *mut _,
                memory.cb,
            ) != 0
        };
        let (textures, texture_bytes, texture_count) = {
            let manager = ctx.tex_manager();
            let manager = manager.read();
            let textures: Vec<_> = manager.allocated().take(128).map(|(_, meta)| {
                serde_json::json!({"name":meta.name,"size":meta.size,"bytes":meta.bytes_used()})
            }).collect();
            (
                textures,
                manager
                    .allocated()
                    .map(|(_, meta)| meta.bytes_used())
                    .sum::<usize>(),
                manager.num_allocated(),
            )
        };
        let size = ctx.screen_rect().size();
        let row = serde_json::json!({
            "pid":std::process::id(), "elapsedSeconds":self.started.elapsed().as_secs_f64(),
            "viewport":format!("{:?}",ctx.viewport_id()),
            "screen":[size.x,size.y], "scale":ctx.pixels_per_point(),
            "maxTextureSide":ctx.input(|input|input.max_texture_side),
            "fontImageSize":ctx.fonts(|fonts|fonts.font_image_size()),
            "fontAtlasFill":ctx.fonts(|fonts|fonts.font_atlas_fill_ratio()),
            "textureBytes":texture_bytes,"textureCount":texture_count,"texturesTruncated":texture_count > 128,
            "textures":textures,
            "renderer":self.renderer,
            "memoryMeasured":measured, "workingSetBytes":memory.WorkingSetSize,
            "privateBytes":memory.PrivateUsage,"peakWorkingSetBytes":memory.PeakWorkingSetSize,
            "peakCommitBytes":memory.PeakPagefileUsage
        });
        // Diagnostic mode only: stream bounded metadata, never retain pixel buffers or user data.
        if writeln!(self.file, "{row}").is_err() {
            self.rows = MAX_ROWS;
            return;
        }
        self.rows += 1;
        self.last = Some(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_streams_metadata_without_overwriting_and_stops_at_a_finite_row_limit() {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/resource-probe-tests");
        std::fs::create_dir_all(&folder).unwrap();
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = folder.join(format!("{}-{suffix}.jsonl", std::process::id()));
        let ctx = egui::Context::default();
        install(&ctx, &path, None).unwrap();
        for _ in 0..5 {
            let _ = ctx.run(egui::RawInput::default(), |_| {});
        }
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            content.lines().count(),
            1,
            "repaints must not create unbounded frame logs"
        );
        let row: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(row["memoryMeasured"], true);
        assert!(row["workingSetBytes"].as_u64().unwrap() > 0);
        assert!(row["fontImageSize"][0].as_u64().unwrap() > 0);
        assert!(install(&ctx, &path, None).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), content);

        let limit_path = folder.join(format!("{}-{suffix}-limit.jsonl", std::process::id()));
        let mut probe = Probe {
            file: OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&limit_path)
                .unwrap(),
            started: Instant::now(),
            last: None,
            rows: 0,
            renderer: None,
        };
        for _ in 0..MAX_ROWS + 10 {
            probe.last = None;
            probe.record(&ctx);
        }
        assert_eq!(
            std::fs::read_to_string(&limit_path)
                .unwrap()
                .lines()
                .count(),
            MAX_ROWS
        );
    }
}
