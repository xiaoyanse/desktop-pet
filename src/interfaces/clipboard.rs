use super::Wake;
use crate::domain::clipboard::{Content, Packet};
pub enum ClipboardEvent {
    Ready,
    Item(Packet),
    Failed(String),
}
pub trait ClipboardWatch: Send {
    fn next(&mut self) -> Option<ClipboardEvent>;
    fn stop(&mut self);
    fn finished(&self) -> bool;
}
pub trait ClipboardBackend: Send + Sync {
    fn watch(&self, max_bytes: usize, wake: Wake) -> Result<Box<dyn ClipboardWatch>, String>;
    fn copy(&self, content: &Content) -> Result<(), String>;
}
