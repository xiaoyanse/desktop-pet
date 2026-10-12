use crate::{
    application::clipboard::ClipboardService,
    domain::{
        clipboard::{ClipboardConfig, Content, Packet},
        screenshot::Image,
    },
    interfaces::{ClipboardBackend, ClipboardEvent, ClipboardWatch, Wake},
    ui::work_tools::clipboard::Page,
};
use std::sync::Arc;
struct Synthetic;
struct Watch(bool);
impl ClipboardWatch for Watch {
    fn next(&mut self) -> Option<ClipboardEvent> {
        if self.0 {
            self.0 = false;
            Some(ClipboardEvent::Ready)
        } else {
            None
        }
    }
    fn stop(&mut self) {
        self.0 = false;
    }
    fn finished(&self) -> bool {
        false
    }
}
impl ClipboardBackend for Synthetic {
    fn watch(&self, _: usize, _: Wake) -> Result<Box<dyn ClipboardWatch>, String> {
        Ok(Box::new(Watch(true)))
    }
    fn copy(&self, _: &Content) -> Result<(), String> {
        Err("合成预览不访问系统剪贴板".into())
    }
}
pub struct Preview {
    page: Page,
    service: ClipboardService,
}
impl Default for Preview {
    fn default() -> Self {
        Self {
            page: Page::default(),
            service: ClipboardService::new(
                Arc::new(Synthetic),
                Arc::new(|| {}),
                ClipboardConfig::default(),
            ),
        }
    }
}
impl Preview {
    pub fn stage(&mut self, stage: usize) {
        self.page.release();
        if stage == 16 {
            return;
        }
        if self.service.history.items.is_empty() {
            let at = crate::application::office::now();
            for text in [
                "整理阶段交付：先核对真实入口，再检查取消与错误反馈。",
                "今天的工作要点\n保留重要片段，完成下一步。",
            ] {
                self.service
                    .history
                    .insert(Packet::text(text.into()).unwrap(), at, &self.service.config)
                    .unwrap();
            }
            self.service.history.pin(1);
            let mut pixels = vec![0; 320 * 180 * 4];
            for y in 0..180 {
                for x in 0..320 {
                    let i = (y * 320 + x) * 4;
                    pixels[i..i + 4].copy_from_slice(&[
                        45 + (x / 8) as u8,
                        65 + (y / 4) as u8,
                        115,
                        255,
                    ]);
                }
            }
            self.service
                .history
                .insert(
                    Packet::picture(Image::new([320, 180], pixels).unwrap()).unwrap(),
                    at,
                    &self.service.config,
                )
                .unwrap();
            self.service.start();
            self.service.tick();
        }
        if stage == 18 {
            self.page.preview_policy(&self.service.config);
        }
        if stage == 19 {
            self.page.preview_clear();
        }
    }
    pub fn show(&mut self, ctx: &eframe::egui::Context) {
        crate::ui::settings::clipboard_preview(ctx, &mut self.page, &mut self.service);
    }
}
