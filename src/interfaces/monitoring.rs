use crate::domain::{config::Config, resources::Snapshot};

pub type SamplerFactory = Box<dyn FnOnce() -> Box<dyn ResourceSampler> + Send>;

pub trait ResourceSampler {
    fn sample(&mut self, config: &Config, details: bool) -> Snapshot;
}
