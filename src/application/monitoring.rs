use crate::{
    domain::{config::Config, resources::Snapshot},
    interfaces::{SamplerFactory, Wake},
};
use std::{
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

enum Command {
    Configure(Box<Config>, bool),
    Stop,
}

pub struct MonitorWorker {
    shared: Arc<Mutex<Option<Snapshot>>>,
    tx: mpsc::Sender<Command>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl MonitorWorker {
    pub fn start(factory: SamplerFactory, config: Config, wake: Wake) -> Self {
        let shared = Arc::new(Mutex::new(None));
        let worker_shared = shared.clone();
        let (tx, rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let mut sampler = factory();
            let mut config = config;
            let mut details = false;
            loop {
                let sample = sampler.sample(&config, details);
                if let Ok(mut slot) = worker_shared.lock() {
                    *slot = Some(sample);
                } else {
                    break;
                }
                wake();
                match rx.recv_timeout(Duration::from_secs(config.seconds)) {
                    Ok(Command::Configure(next, expanded)) => {
                        config = *next;
                        details = expanded;
                    }
                    Ok(Command::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
            }
        });
        Self {
            shared,
            tx,
            worker: Some(worker),
        }
    }
    pub fn configure(&self, config: &Config, details: bool) {
        let _ = self
            .tx
            .send(Command::Configure(Box::new(config.clone()), details));
    }
    pub fn take_sample(&self) -> Option<Snapshot> {
        self.shared.lock().ok()?.take()
    }
}
impl Drop for MonitorWorker {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interfaces::ResourceSampler;

    struct TestSampler {
        samples: mpsc::Sender<bool>,
    }
    impl ResourceSampler for TestSampler {
        fn sample(&mut self, _: &Config, details: bool) -> Snapshot {
            self.samples.send(details).unwrap();
            Snapshot::default()
        }
    }
    #[test]
    fn worker_creates_sampler_on_worker_thread_and_applies_details_without_waiting_for_interval() {
        let caller = std::thread::current().id();
        let (created, thread_rx) = mpsc::channel();
        let (samples, sample_rx) = mpsc::channel();
        let (wakes, wake_rx) = mpsc::channel();
        let worker = MonitorWorker::start(
            Box::new(move || {
                created.send(std::thread::current().id()).unwrap();
                Box::new(TestSampler { samples })
            }),
            Config::default(),
            Arc::new(move || {
                wakes.send(()).unwrap();
            }),
        );
        assert_ne!(
            thread_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            caller
        );
        assert!(!sample_rx.recv_timeout(Duration::from_secs(2)).unwrap());
        wake_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(worker.take_sample().is_some());
        worker.configure(&Config::default(), true);
        assert!(sample_rx.recv_timeout(Duration::from_secs(2)).unwrap());
        drop(worker);
    }
}
