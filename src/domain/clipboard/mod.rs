#[cfg(test)]
mod tests;
use super::screenshot::Image;
use serde::{Deserialize, Serialize};
use std::{
    collections::{VecDeque, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    sync::Arc,
};
pub const MAX_TEXT: usize = 256 * 1024;
pub const MAX_IMAGE: usize = 32 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ClipboardConfig {
    pub count: usize,
    pub mib: usize,
}
impl Default for ClipboardConfig {
    fn default() -> Self {
        Self { count: 30, mib: 32 }
    }
}
impl ClipboardConfig {
    pub fn normalize(&mut self) {
        self.count = self.count.clamp(1, 200);
        self.mib = self.mib.clamp(4, 64);
    }
    pub fn bytes(&self) -> usize {
        self.mib.clamp(4, 64) * 1024 * 1024
    }
}
#[derive(Clone)]
pub enum Content {
    Text(Arc<str>),
    Picture(Image),
}
#[derive(Clone)]
pub struct Packet {
    pub content: Content,
    pub thumbnail: Option<Image>,
    pub signature: u64,
    pub bytes: usize,
}
impl Packet {
    pub fn text(text: String) -> Result<Self, String> {
        if text.is_empty() || text.len() > MAX_TEXT {
            return Err("文字为空或超过256 KiB限制".into());
        }
        let mut hash = DefaultHasher::new();
        0u8.hash(&mut hash);
        text.hash(&mut hash);
        Ok(Self {
            bytes: text.len(),
            content: Content::Text(text.into()),
            thumbnail: None,
            signature: hash.finish(),
        })
    }
    pub fn picture(image: Image) -> Result<Self, String> {
        if image.pixels().len() > MAX_IMAGE {
            return Err("图片超过32 MiB单项限制".into());
        }
        let [width, height] = image.size();
        let scale = (160.0 / width as f64).min(120.0 / height as f64).min(1.0);
        let [w, h] = [
            (width as f64 * scale).round().max(1.0) as u32,
            (height as f64 * scale).round().max(1.0) as u32,
        ];
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let at = (((y as u64 * height as u64 / h as u64) * width as u64
                    + x as u64 * width as u64 / w as u64)
                    * 4) as usize;
                rgba.extend_from_slice(&image.pixels()[at..at + 4]);
            }
        }
        let thumb = Image::new([w, h], rgba)?;
        let mut hash = DefaultHasher::new();
        1u8.hash(&mut hash);
        image.size().hash(&mut hash);
        image.pixels().hash(&mut hash);
        Ok(Self {
            bytes: image.pixels().len() + thumb.pixels().len(),
            content: Content::Picture(image),
            thumbnail: Some(thumb),
            signature: hash.finish(),
        })
    }
    pub fn same(&self, other: &Self) -> bool {
        self.signature == other.signature
            && match (&self.content, &other.content) {
                (Content::Text(a), Content::Text(b)) => a == b,
                (Content::Picture(a), Content::Picture(b)) => {
                    a.size() == b.size() && a.pixels() == b.pixels()
                }
                _ => false,
            }
    }
    pub fn label(&self) -> String {
        match &self.content {
            Content::Text(t) => t.chars().take(120).collect(),
            Content::Picture(i) => format!("图片 {} × {}", i.size()[0], i.size()[1]),
        }
    }
}
pub struct Item {
    pub id: u64,
    pub captured: u64,
    pub pinned: bool,
    pub packet: Packet,
}
#[derive(Default)]
pub struct History {
    pub items: VecDeque<Item>,
    next: u64,
    revision: u64,
}
impl History {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn bytes(&self) -> usize {
        self.items.iter().map(|i| i.packet.bytes).sum()
    }
    pub fn insert(
        &mut self,
        packet: Packet,
        captured: u64,
        limits: &ClipboardConfig,
    ) -> Result<(), String> {
        if let Some(at) = self.items.iter().position(|i| i.packet.same(&packet)) {
            let mut item = self.items.remove(at).unwrap();
            item.captured = captured;
            self.items.push_front(item);
            self.revision = self.revision.wrapping_add(1);
            return Ok(());
        }
        let pinned_count = self.items.iter().filter(|i| i.pinned).count();
        let pinned_bytes: usize = self
            .items
            .iter()
            .filter(|i| i.pinned)
            .map(|i| i.packet.bytes)
            .sum();
        if pinned_count + 1 > limits.count || pinned_bytes + packet.bytes > limits.bytes() {
            return Err("容量不足：请取消部分固定项目或增加容量".into());
        }
        let next = self.next.checked_add(1).ok_or("剪贴板条目编号耗尽")?;
        while self.items.len() + 1 > limits.count || self.bytes() + packet.bytes > limits.bytes() {
            let at = self
                .items
                .iter()
                .rposition(|i| !i.pinned)
                .ok_or("固定项目占满容量")?;
            self.items.remove(at);
        }
        self.next = next;
        self.items.push_front(Item {
            id: self.next,
            captured,
            pinned: false,
            packet,
        });
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }
    pub fn trim(&mut self, limits: &ClipboardConfig) -> Result<(), String> {
        let before = self.items.len();
        let pinned: Vec<_> = self.items.iter().filter(|i| i.pinned).collect();
        if pinned.len() > limits.count
            || pinned.iter().map(|i| i.packet.bytes).sum::<usize>() > limits.bytes()
        {
            return Err("固定项目超过新容量，原设置保留".into());
        }
        while self.items.len() > limits.count || self.bytes() > limits.bytes() {
            let at = self
                .items
                .iter()
                .rposition(|i| !i.pinned)
                .ok_or("固定项目超出容量")?;
            self.items.remove(at);
        }
        if before != self.items.len() {
            self.revision = self.revision.wrapping_add(1);
        }
        Ok(())
    }
    pub fn delete(&mut self, id: u64) {
        let before = self.items.len();
        self.items.retain(|i| i.id != id);
        if before != self.items.len() {
            self.revision = self.revision.wrapping_add(1);
        }
    }
    pub fn pin(&mut self, id: u64) {
        if let Some(i) = self.items.iter_mut().find(|i| i.id == id) {
            i.pinned = !i.pinned;
            self.revision = self.revision.wrapping_add(1);
        }
    }
    pub fn clear(&mut self) {
        if !self.items.is_empty() {
            self.items.clear();
            self.revision = self.revision.wrapping_add(1);
        }
    }
}
