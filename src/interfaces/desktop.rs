use super::Wake;
use crate::domain::desktop::DesktopGeometry;

pub type DesktopFactory = Box<dyn FnOnce(isize, Wake) -> Box<dyn DesktopSession>>;

#[derive(Clone, Copy)]
pub enum DesktopAction {
    Unlock,
    Passthrough,
    Settings,
    ToggleMode,
    Pet,
    PetReact,
    PetDrop,
    Cleanup,
    Exit,
}

pub trait DesktopSession {
    fn configure_card(
        &self,
        collapsed: bool,
        draggable: bool,
        drag_height: f32,
        toolbar_width: f32,
    );
    fn geometry(&self) -> Option<DesktopGeometry>;
    fn cursor(&self) -> Option<[f32; 2]>;
    fn configure_auxiliary(&self);
    fn warning(&self) -> Option<String> {
        None
    }
    fn actions(&self) -> Vec<DesktopAction>;
    fn set_snapping(&self, enabled: bool);
    fn tooltip(&mut self, text: &str, active: bool);
    fn menu(&self, compact: bool) -> i32;
    fn autostart(&self) -> bool;
    fn set_autostart(&self, enable: bool) -> Result<(), String>;
    fn set_clean_hotkey(&self, enabled: bool) -> Result<(), String>;
}
