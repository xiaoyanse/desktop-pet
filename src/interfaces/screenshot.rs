use crate::domain::screenshot::{Image, Mode, Region, Selection};
use std::sync::atomic::AtomicBool;

pub trait ScreenCapture: Send + Sync {
    fn bounds(&self) -> Result<Region, String>;
    fn select(&self, mode: Mode, cancel: &AtomicBool) -> Result<Option<Selection>, String>;
    fn capture(&self, region: Region, cancel: &AtomicBool) -> Result<Image, String>;
    fn validate(&self, selection: &Selection) -> Result<(), String> {
        selection.region.bytes()?;
        if !self.bounds()?.contains(selection.region) {
            return Err("选区已超出桌面，请重新选择".into());
        }
        Ok(())
    }
}
pub trait ScreenshotOutput: Send + Sync {
    fn default_folder(&self) -> String;
    fn save(&self, image: &Image, folder: &str) -> Result<String, String>;
    fn copy(&self, image: &Image) -> Result<(), String>;
}
