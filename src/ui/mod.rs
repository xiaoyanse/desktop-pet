mod common;
mod companion;
pub(crate) mod development;
pub mod monitor;
mod monitoring;
pub(crate) mod rendering;
mod settings;
mod work_tools;
pub(crate) use common::fonts::configure_atlas as configure_framework_atlas;
#[cfg(test)]
pub(crate) use common::fonts::definitions as test_font_definitions;
#[cfg(test)]
pub(crate) use common::fonts::install as install_test_font;
