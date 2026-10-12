mod codec;
mod io;
use crate::{
    domain::clipboard::Content,
    interfaces::{ClipboardBackend, ClipboardEvent, ClipboardWatch, Wake},
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU32, Ordering},
};
use windows_sys::Win32::{
    System::{DataExchange::*, Threading::GetCurrentThreadId},
    UI::WindowsAndMessaging::*,
};
pub struct WindowsClipboard;
struct Watch {
    slot: Arc<Mutex<Option<ClipboardEvent>>>,
    cancel: Arc<AtomicBool>,
    tid: Arc<AtomicU32>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl ClipboardBackend for WindowsClipboard {
    fn copy(&self, content: &Content) -> Result<(), String> {
        io::copy(content)
    }
    fn watch(&self, max_bytes: usize, wake: Wake) -> Result<Box<dyn ClipboardWatch>, String> {
        self.watch_impl(max_bytes, wake, true)
    }
}
impl WindowsClipboard {
    fn watch_impl(
        &self,
        max_bytes: usize,
        wake: Wake,
        read_content: bool,
    ) -> Result<Box<dyn ClipboardWatch>, String> {
        let slot = Arc::new(Mutex::new(None));
        let cancel = Arc::new(AtomicBool::new(false));
        let tid = Arc::new(AtomicU32::new(0));
        let (output, stop, id) = (slot.clone(), cancel.clone(), tid.clone());
        let worker = std::thread::Builder::new()
            .name("clipboard-events".into())
            .spawn(move || {
                let publish = |event| {
                    if !stop.load(Ordering::Acquire) {
                        if let Ok(mut out) = output.lock() {
                            *out = Some(event);
                        }
                        wake();
                    }
                };
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
                    let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
                    let hwnd = CreateWindowExW(
                        0,
                        class.as_ptr(),
                        std::ptr::null(),
                        0,
                        0,
                        0,
                        0,
                        0,
                        HWND_MESSAGE,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null(),
                    );
                    if hwnd.is_null() {
                        return Err("无法创建剪贴板监听窗口".to_string());
                    }
                    let _window = Window(hwnd);
                    let mut message = std::mem::zeroed();
                    PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
                    id.store(GetCurrentThreadId(), Ordering::Release);
                    if stop.load(Ordering::Acquire) {
                        return Ok(());
                    }
                    if AddClipboardFormatListener(hwnd) == 0 {
                        return Err("无法注册剪贴板监听".into());
                    }
                    let _listener = Listener(hwnd);
                    let mut sequence = GetClipboardSequenceNumber();
                    publish(ClipboardEvent::Ready);
                    while !stop.load(Ordering::Acquire) {
                        let got = GetMessageW(&mut message, std::ptr::null_mut(), 0, 0);
                        if got == 0 {
                            break;
                        }
                        if got < 0 {
                            return Err("剪贴板消息通道异常".into());
                        }
                        if message.message == WM_CLIPBOARDUPDATE {
                            let current = GetClipboardSequenceNumber();
                            if current != sequence && read_content && !stop.load(Ordering::Acquire)
                            {
                                sequence = current;
                                match io::read(hwnd, max_bytes) {
                                    Ok(Some(packet)) => publish(ClipboardEvent::Item(packet)),
                                    Ok(None) => publish(ClipboardEvent::Failed(
                                        "已跳过空内容或未支持的剪贴板格式".into(),
                                    )),
                                    Err(e) => publish(ClipboardEvent::Failed(e)),
                                }
                            }
                        } else {
                            TranslateMessage(&message);
                            DispatchMessageW(&message);
                        }
                    }
                    Ok(())
                }))
                .unwrap_or_else(|_| Err("剪贴板监听发生异常".into()));
                if let Err(error) = result {
                    publish(ClipboardEvent::Failed(error));
                }
                wake();
            })
            .map_err(|e| format!("监听线程无法启动：{e}"))?;
        Ok(Box::new(Watch {
            slot,
            cancel,
            tid,
            worker: Some(worker),
        }))
    }
}
impl ClipboardWatch for Watch {
    fn next(&mut self) -> Option<ClipboardEvent> {
        self.slot.lock().ok()?.take()
    }
    fn stop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        let tid = self.tid.load(Ordering::Acquire);
        if tid != 0 {
            unsafe {
                PostThreadMessageW(tid, WM_QUIT, 0, 0);
            }
        }
    }
    fn finished(&self) -> bool {
        self.worker.as_ref().is_none_or(|w| w.is_finished())
    }
}
impl Drop for Watch {
    fn drop(&mut self) {
        self.stop();
        if let Some(w) = self.worker.take()
            && w.is_finished()
        {
            let _ = w.join();
        }
    }
}
struct Window(windows_sys::Win32::Foundation::HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0);
        }
    }
}
struct Listener(windows_sys::Win32::Foundation::HWND);
impl Drop for Listener {
    fn drop(&mut self) {
        unsafe {
            RemoveClipboardFormatListener(self.0);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    #[test]
    fn native_listener_registration_and_stop_release_thread_without_reading_clipboard() {
        for early_stop in [false, false, true] {
            let mut watch = WindowsClipboard
                .watch_impl(0, Arc::new(|| {}), false)
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            if !early_stop {
                loop {
                    match watch.next() {
                        Some(ClipboardEvent::Ready) => break,
                        Some(ClipboardEvent::Failed(e)) => panic!("{e}"),
                        Some(ClipboardEvent::Item(_)) => panic!("metadata-only probe read content"),
                        None => {}
                    }
                    assert!(Instant::now() < deadline, "registration timed out");
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
            watch.stop();
            while !watch.finished() {
                assert!(Instant::now() < deadline, "stop timed out");
                std::thread::sleep(Duration::from_millis(2));
            }
        }
    }
}
