use crate::{
    application::screenshot::ScreenshotService,
    domain::screenshot::{Document, Image, Mode, Region, Selection},
    interfaces::{ScreenCapture, ScreenshotOutput},
};
use eframe::egui;
use std::sync::{Arc, atomic::AtomicBool};

struct Synthetic;
impl ScreenCapture for Synthetic {
    fn bounds(&self) -> Result<Region, String> {
        Err("合成预览不会访问真实桌面".into())
    }
    fn select(&self, _: Mode, _: &AtomicBool) -> Result<Option<Selection>, String> {
        Err("合成预览不会访问真实桌面".into())
    }
    fn capture(&self, _: Region, _: &AtomicBool) -> Result<Image, String> {
        Err("合成预览不会访问真实桌面".into())
    }
}
impl ScreenshotOutput for Synthetic {
    fn default_folder(&self) -> String {
        "E:\\Screenshots · 合成预览".into()
    }
    fn save(&self, _: &Image, _: &str) -> Result<String, String> {
        Err("合成预览不保存用户截图".into())
    }
    fn copy(&self, _: &Image) -> Result<(), String> {
        Err("合成预览不访问系统剪贴板".into())
    }
}
pub struct Preview {
    page: crate::ui::work_tools::screenshot::Page,
    service: ScreenshotService,
}
impl Default for Preview {
    fn default() -> Self {
        let backend = Arc::new(Synthetic);
        let mut service = ScreenshotService::new(backend.clone(), backend, Arc::new(|| {}));
        let [w, h] = [640u32, 220u32];
        let mut pixels = vec![0; w as usize * h as usize * 4];
        for y in 0..h {
            for x in 0..w {
                let p = (y as usize * w as usize + x as usize) * 4;
                let bar = (80..180).contains(&y)
                    && ((50..160).contains(&x)
                        || (220..360).contains(&x)
                        || (420..580).contains(&x));
                pixels[p..p + 4].copy_from_slice(if bar {
                    &[165, 213, 200, 255]
                } else {
                    &[23, 37, 49, 255]
                });
            }
        }
        service.document = Some(Document::new(
            Image::new([w, h], pixels).expect("bounded synthetic pixels"),
        ));
        service.message = "合成图片 · 工具栏使用生产组件 · 不访问真实桌面或剪贴板".into();
        Self {
            page: Default::default(),
            service,
        }
    }
}
impl Preview {
    pub fn show(&mut self, ctx: &egui::Context) {
        self.service.tick();
        crate::ui::settings::screenshot_preview(ctx, &mut self.page, &mut self.service);
    }
}
