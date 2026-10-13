mod fonts;
mod software_rendering;
pub(crate) fn font_source() -> crate::interfaces::FontSource {
    crate::infrastructure::windows::fonts::load().expect("集成测试需要系统中文字体")
}
