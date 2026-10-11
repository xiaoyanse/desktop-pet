use super::Wake;
use crate::domain::cleanup::{CleanupReport, MemoryStatus, Operation};
use std::sync::mpsc::Receiver;

pub trait MemoryCleaner {
    fn request(
        &self,
        operations: Vec<Operation>,
        automatic: bool,
        wake: Wake,
    ) -> Receiver<Result<CleanupReport, String>>;
    fn elevated(&self) -> bool;
    fn inspect(&self) -> Result<MemoryStatus, String>;
}
