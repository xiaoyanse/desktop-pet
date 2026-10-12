mod common;
mod companion;
pub mod monitor;
mod monitoring;
mod settings;
mod work_tools;
#[cfg(test)]
pub(crate) use common::fonts::definitions as test_font_definitions;
#[cfg(test)]
pub(crate) use common::fonts::install as install_test_font;
