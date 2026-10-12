mod display;
pub(crate) use display::status;
mod notice;
mod panel;
pub use notice::feedback;
pub use panel::show;

#[cfg(test)]
mod tests;
