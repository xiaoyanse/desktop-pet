use crate::{domain::screenshot::Image, interfaces::ScreenshotOutput};
use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub struct LocalScreenshots;
impl ScreenshotOutput for LocalScreenshots {
    fn default_folder(&self) -> String {
        let executable = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("E:\\"));
        let root = executable.ancestors().last().unwrap_or(Path::new("E:\\"));
        root.join("DesktopMeterData")
            .join("Screenshots")
            .to_string_lossy()
            .into_owned()
    }
    fn save(&self, image: &Image, folder: &str) -> Result<String, String> {
        save_in(image, Path::new(folder))
    }
    fn copy(&self, image: &Image) -> Result<(), String> {
        crate::infrastructure::windows::screenshot::copy(image)
    }
}
fn save_in(image: &Image, folder: &Path) -> Result<String, String> {
    if !folder.is_absolute() {
        return Err("请选择绝对路径的截图保存目录".into());
    }
    fs::create_dir_all(folder).map_err(|_| "截图目录无法创建，请检查位置及权限")?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    for sequence in 0..100 {
        let final_path = folder.join(format!("screenshot-{stamp}-{sequence}.png"));
        let temp_path = folder.join(format!("screenshot-{stamp}-{sequence}.partial"));
        if final_path.exists() {
            continue;
        }
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err("截图文件无法创建，请检查磁盘及目录权限".into()),
        };
        let result = (|| {
            let [w, h] = image.size();
            let mut encoder = png::Encoder::new(&mut file, w, h);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
            writer
                .write_image_data(image.pixels())
                .map_err(|e| e.to_string())?;
            writer.finish().map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            Ok::<_, String>(())
        })();
        drop(file);
        let result = result.and_then(|_| publish(&temp_path, &final_path));
        let _ = fs::remove_file(&temp_path);
        result.map_err(|e| format!("图片保存失败：{e}；已有截图不覆盖"))?;
        return Ok(final_path.to_string_lossy().into_owned());
    }
    Err("截图文件名冲突，请重试".into())
}

fn publish(from: &Path, to: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    // MoveFileW fails when the destination exists; unlike rename, it cannot replace it.
    if unsafe { windows_sys::Win32::Storage::FileSystem::MoveFileW(from.as_ptr(), to.as_ptr()) }
        == 0
    {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn png_roundtrip_preserves_pixels_and_publishing_keeps_previous_images() {
        let folder = std::env::temp_dir().join(format!(
            "desktop-meter-png-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let image = Image::new([2, 1], vec![250, 30, 10, 255, 10, 90, 240, 0]).unwrap();
        let first = save_in(&image, &folder).unwrap();
        let second = save_in(&image, &folder).unwrap();
        assert_ne!(first, second);
        assert_eq!(
            image::open(&first).unwrap().to_rgba8().as_raw(),
            image.pixels()
        );
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 2);
        fs::remove_file(first).unwrap();
        fs::remove_file(second).unwrap();
        fs::remove_dir(folder).unwrap();
        assert!(save_in(&image, Path::new("relative")).is_err());
    }
    #[test]
    fn publishing_never_overwrites_an_existing_destination() {
        let dir = std::env::temp_dir().join(format!(
            "desktop-meter-publish-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        let source = dir.join("source");
        let target = dir.join("target");
        fs::write(&source, b"new").unwrap();
        fs::write(&target, b"keep").unwrap();
        assert!(publish(&source, &target).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"keep");
        assert!(source.exists());
        fs::remove_file(source).unwrap();
        fs::remove_file(target).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
