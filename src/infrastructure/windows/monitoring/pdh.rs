use crate::infrastructure::windows::native::{self, wide};
use std::collections::HashMap;
use windows_sys::Win32::System::Performance::*;

pub struct Counters {
    query: PDH_HQUERY,
    counters: HashMap<&'static str, PDH_HCOUNTER>,
    ready: bool,
}
impl Counters {
    pub fn new(paths: &[(&'static str, &str)]) -> Self {
        let mut this = Self {
            query: std::ptr::null_mut(),
            counters: HashMap::new(),
            ready: false,
        };
        unsafe {
            if PdhOpenQueryW(std::ptr::null(), 0, &mut this.query) != 0 {
                return this;
            }
            for (id, path) in paths {
                let mut counter = std::ptr::null_mut();
                if PdhAddEnglishCounterW(this.query, wide(path).as_ptr(), 0, &mut counter) == 0 {
                    this.counters.insert(id, counter);
                }
            }
            PdhCollectQueryData(this.query);
        }
        this
    }
    pub fn collect(&mut self) {
        self.ready = !self.query.is_null() && unsafe { PdhCollectQueryData(self.query) } == 0;
    }
    pub fn values(&self, id: &str) -> Option<Vec<(String, f64)>> {
        if !self.ready {
            return None;
        }
        let counter = *self.counters.get(id)?;
        let mut size = 0;
        let mut count = 0;
        unsafe {
            let status = PdhGetFormattedCounterArrayW(
                counter,
                PDH_FMT_DOUBLE,
                &mut size,
                &mut count,
                std::ptr::null_mut(),
            );
            if status == 0 && size == 0 {
                return Some(Vec::new());
            }
            if status != PDH_MORE_DATA || size == 0 {
                return None;
            }
            // PDH structures need pointer alignment; a byte vector is not an aligned buffer.
            for _ in 0..3 {
                let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
                let ptr = buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
                let status = PdhGetFormattedCounterArrayW(
                    counter,
                    PDH_FMT_DOUBLE,
                    &mut size,
                    &mut count,
                    ptr,
                );
                if status == PDH_MORE_DATA {
                    continue;
                }
                if status != 0 {
                    return None;
                }
                let items = std::slice::from_raw_parts(ptr, count as usize);
                return Some(
                    items
                        .iter()
                        .filter_map(|item| {
                            if item.FmtValue.CStatus > 1 {
                                return None;
                            }
                            let value = item.FmtValue.Anonymous.doubleValue;
                            value
                                .is_finite()
                                .then(|| (native::from_wide_ptr(item.szName), value.max(0.0)))
                        })
                        .collect(),
                );
            }
        }
        None
    }
}
impl Drop for Counters {
    fn drop(&mut self) {
        if !self.query.is_null() {
            unsafe {
                PdhCloseQuery(self.query);
            }
        }
    }
}
