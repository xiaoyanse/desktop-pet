use super::native::Handle;
use crate::interfaces::FontSource;
use std::{
    borrow::Cow,
    fs::{File, OpenOptions},
    io::Read,
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    path::Path,
    sync::OnceLock,
};
use windows_sys::Win32::{Storage::FileSystem::*, System::Memory::*};

const MAX_FONT_BYTES: u64 = 64 * 1024 * 1024;
static FONT: OnceLock<Option<CachedFont>> = OnceLock::new();

struct CachedFont {
    name: &'static str,
    backing: Backing,
}
enum Backing {
    Mapped(Mapping),
    Owned(Vec<u8>),
}
struct Mapping {
    address: usize,
    len: usize,
    _file: File,
}
impl Mapping {
    fn open(path: &Path) -> Option<Self> {
        let file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
            .open(path)
            .ok()?;
        let metadata = file.metadata().ok()?;
        if !metadata.is_file() || !(1..=MAX_FONT_BYTES).contains(&metadata.len()) {
            return None;
        }
        let len = usize::try_from(metadata.len()).ok()?;
        unsafe {
            let section = Handle::new(CreateFileMappingW(
                file.as_raw_handle(),
                std::ptr::null(),
                PAGE_READONLY,
                0,
                0,
                std::ptr::null(),
            ))?;
            let view = MapViewOfFile(section.0, FILE_MAP_READ, 0, 0, len);
            if view.Value.is_null() {
                return None;
            }
            Some(Self {
                address: view.Value as usize,
                len,
                _file: file,
            })
        }
    }
    fn bytes(&self) -> &[u8] {
        // The view is read-only; retaining the file without write sharing prevents mutation/truncation.
        unsafe { std::slice::from_raw_parts(self.address as *const u8, self.len) }
    }
}
impl Drop for Mapping {
    fn drop(&mut self) {
        unsafe {
            UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                Value: self.address as *mut _,
            });
        }
    }
}
impl CachedFont {
    fn from_path(name: &'static str, path: &Path) -> Option<Self> {
        let backing = Mapping::open(path).map(Backing::Mapped).or_else(|| {
            let mut file = OpenOptions::new()
                .read(true)
                .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
                .open(path)
                .ok()?;
            let metadata = file.metadata().ok()?;
            if !metadata.is_file() || !(1..=MAX_FONT_BYTES).contains(&metadata.len()) {
                return None;
            }
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).ok()?;
            Some(Backing::Owned(bytes))
        })?;
        Some(Self { name, backing })
    }
    fn bytes(&self) -> &[u8] {
        match &self.backing {
            Backing::Mapped(mapping) => mapping.bytes(),
            Backing::Owned(bytes) => bytes,
        }
    }
}
pub fn load() -> Option<FontSource> {
    let font = FONT
        .get_or_init(|| {
            let root = std::env::var_os("WINDIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| "C:\\Windows".into());
            ["msyh.ttc", "msyh.ttf", "simhei.ttf", "simsun.ttc"]
                .into_iter()
                .find_map(|name| CachedFont::from_path(name, &root.join("Fonts").join(name)))
        })
        .as_ref()?;
    // OnceLock owns the backing until process exit, including the fallback heap buffer.
    Some(FontSource {
        name: Cow::Borrowed(font.name),
        bytes: Cow::Borrowed(font.bytes()),
    })
}

#[cfg(test)]
mod tests;
