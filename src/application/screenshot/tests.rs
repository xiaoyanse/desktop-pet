use super::*;
use crate::domain::screenshot::{Mark, Region, Selection, Tool};
use std::{
    sync::{Mutex, atomic::AtomicUsize},
    time::{Duration, Instant},
};

pub(crate) struct Fake {
    pub calls: AtomicUsize,
    pub copied: Mutex<Option<Vec<u8>>>,
    pub fail: AtomicBool,
}
impl Default for Fake {
    fn default() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            copied: Mutex::new(None),
            fail: AtomicBool::new(false),
        }
    }
}
impl ScreenCapture for Fake {
    fn bounds(&self) -> Result<Region, String> {
        Ok(Region {
            left: -2,
            top: 4,
            width: 8,
            height: 8,
        })
    }
    fn select(&self, _: Mode, _: &AtomicBool) -> Result<Option<Selection>, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Some(Selection {
            region: self.bounds()?,
            polygon: vec![],
            window: None,
        }))
    }
    fn capture(&self, r: Region, _: &AtomicBool) -> Result<Image, String> {
        if self.fail.load(Ordering::SeqCst) {
            return Err("synthetic failure".into());
        }
        Image::new([r.width, r.height], vec![255; r.bytes()?])
    }
}
impl ScreenshotOutput for Fake {
    fn default_folder(&self) -> String {
        "E:\\TestScreenshots".into()
    }
    fn save(&self, image: &Image, folder: &str) -> Result<String, String> {
        if self.fail.load(Ordering::SeqCst) {
            return Err("synthetic output error".into());
        }
        *self.copied.lock().unwrap() = Some(image.pixels().to_vec());
        Ok(folder.into())
    }
    fn copy(&self, image: &Image) -> Result<(), String> {
        self.save(image, "").map(|_| ())
    }
}
pub(crate) fn fake_service() -> (ScreenshotService, Arc<Fake>) {
    let backend = Arc::new(Fake::default());
    (
        ScreenshotService::new(backend.clone(), backend.clone(), Arc::new(|| {})),
        backend,
    )
}
fn finish(service: &mut ScreenshotService) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while service.busy() {
        service.tick();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
}
#[test]
fn all_modes_are_explicit_single_task_and_errors_preserve_previous_preview() {
    let (mut service, backend) = fake_service();
    assert_eq!(backend.calls.load(Ordering::SeqCst), 0);
    for mode in Mode::ALL {
        service.capture(mode);
        service.capture(mode);
        finish(&mut service);
        assert!(service.document.is_some());
    }
    assert_eq!(backend.calls.load(Ordering::SeqCst), 4);
    let generation = service.generation;
    backend.fail.store(true, Ordering::SeqCst);
    service.capture(Mode::Window);
    finish(&mut service);
    assert_eq!(service.generation, generation);
    assert!(service.message.contains("failure"));
}
#[test]
fn copy_save_and_pin_use_same_annotated_pixels_and_closing_drops_original() {
    let (mut service, backend) = fake_service();
    service.capture(Mode::Rectangle);
    finish(&mut service);
    service
        .document
        .as_mut()
        .unwrap()
        .add(Mark {
            tool: Tool::Cover,
            points: vec![[1, 1], [5, 5]],
            color: [12, 23, 34],
            width: 1,
        })
        .unwrap();
    let expected = service.document.as_ref().unwrap().render().unwrap();
    for kind in [Output::Copy, Output::Save, Output::Pin] {
        service.export(kind);
        finish(&mut service);
    }
    assert_eq!(
        backend.copied.lock().unwrap().as_deref(),
        Some(expected.pixels())
    );
    assert_eq!(service.pinned.as_ref().unwrap().pixels(), expected.pixels());
    service.discard();
    assert!(service.document.is_none());
    assert!(service.pinned.is_some());
}
#[test]
fn cancellation_wins_even_when_capture_result_already_arrived() {
    let (mut service, _) = fake_service();
    service.capture(Mode::Rectangle);
    finish(&mut service);
    let generation = service.generation;
    service.capture(Mode::Fullscreen);
    let deadline = Instant::now() + Duration::from_secs(3);
    while !service.job.as_ref().unwrap().worker.is_finished() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    service.cancel();
    finish(&mut service);
    assert_eq!(service.generation, generation);
    assert!(service.message.contains("取消"));
}
#[test]
fn closing_preview_during_export_releases_it_after_io_and_failure_never_says_saved() {
    let (mut service, backend) = fake_service();
    service.capture(Mode::Rectangle);
    finish(&mut service);
    backend.fail.store(true, Ordering::SeqCst);
    service.export(Output::Save);
    service.discard();
    finish(&mut service);
    assert!(service.document.is_none());
    assert!(service.message.contains("error"));
    assert!(!service.message.contains("已保存"));
}

#[test]
fn window_change_during_capture_does_not_replace_the_previous_preview() {
    struct Changed(AtomicUsize);
    impl ScreenCapture for Changed {
        fn bounds(&self) -> Result<Region, String> {
            Ok(Region {
                left: 0,
                top: 0,
                width: 2,
                height: 2,
            })
        }
        fn select(&self, _: Mode, _: &AtomicBool) -> Result<Option<Selection>, String> {
            Ok(Some(Selection {
                region: self.bounds()?,
                polygon: vec![],
                window: Some(crate::domain::screenshot::WindowTarget {
                    handle: 123,
                    pid: 456,
                }),
            }))
        }
        fn capture(&self, _: Region, _: &AtomicBool) -> Result<Image, String> {
            Image::new([2, 2], vec![255; 16])
        }
        fn validate(&self, _: &Selection) -> Result<(), String> {
            if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
                Ok(())
            } else {
                Err("window changed".into())
            }
        }
    }
    let backend = Arc::new(Fake::default());
    let mut service = ScreenshotService::new(
        Arc::new(Changed(AtomicUsize::new(0))),
        backend,
        Arc::new(|| {}),
    );
    service.document = Some(Document::new(
        Image::new([1, 1], vec![11, 22, 33, 255]).unwrap(),
    ));
    service.capture(Mode::Window);
    finish(&mut service);
    assert_eq!(
        service.document.as_ref().unwrap().image.pixels(),
        &[11, 22, 33, 255]
    );
    assert!(service.message.contains("changed"));
}
