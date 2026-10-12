pub mod privacy;
mod render;
#[cfg(test)]
mod tests;

use crate::domain::{config::Metric, resources::Snapshot};
use privacy::SafePath;

#[derive(Clone, Default)]
pub struct Report {
    pub captured_at: String,
    pub resources: Vec<(Metric, String)>,
    pub cpu_top: Vec<Process>,
    pub memory_top: Vec<Process>,
    pub runtimes: Vec<Process>,
    pub enumerated: usize,
    pub unreadable: usize,
    pub omitted_runtimes: usize,
}

#[derive(Clone)]
pub struct Process {
    pub name: String,
    pub pid: u32,
    pub parent: Option<(u32, String)>,
    pub path: SafePath,
    pub cpu: Option<f64>,
    pub working_set: Option<u64>,
    pub private: Option<u64>,
}

impl Report {
    pub fn set_resources(&mut self, snapshot: &Snapshot) {
        self.resources = Metric::ALL
            .into_iter()
            .map(|metric| {
                let text = snapshot
                    .values
                    .get(&metric)
                    .map_or("暂不可用", |r| r.text.as_str());
                (metric, text.to_owned())
            })
            .collect();
    }
    pub fn markdown(&self) -> String {
        render::markdown(self)
    }
    pub fn runtime_count(&self, kind: &str) -> usize {
        self.runtimes
            .iter()
            .filter(|p| runtime_kind(&p.name) == Some(kind))
            .count()
    }
}

pub fn runtime_kind(name: &str) -> Option<&'static str> {
    let name = name.to_ascii_lowercase();
    let stem = name.strip_suffix(".exe").unwrap_or(&name);
    if stem == "node" || stem == "nodejs" {
        Some("Node")
    } else if stem == "python"
        || stem == "pythonw"
        || stem.strip_prefix("python").is_some_and(|suffix| {
            suffix.chars().any(|c| c.is_ascii_digit())
                && suffix.chars().all(|c| c.is_ascii_digit() || c == '.')
        })
    {
        Some("Python")
    } else {
        None
    }
}
