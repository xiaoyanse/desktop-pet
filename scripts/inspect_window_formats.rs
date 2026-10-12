use std::ffi::c_void;

#[repr(C)]
#[derive(Default)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}
#[link(name = "user32")]
unsafe extern "system" {
    fn EnumWindows(
        callback: unsafe extern "system" fn(*mut c_void, isize) -> i32,
        arg: isize,
    ) -> i32;
    fn GetWindowThreadProcessId(window: *mut c_void, pid: *mut u32) -> u32;
    fn GetClientRect(window: *mut c_void, rect: *mut Rect) -> i32;
    fn IsWindowVisible(window: *mut c_void) -> i32;
    fn GetDC(window: *mut c_void) -> *mut c_void;
    fn ReleaseDC(window: *mut c_void, dc: *mut c_void) -> i32;
}
#[link(name = "gdi32")]
unsafe extern "system" {
    fn GetPixelFormat(dc: *mut c_void) -> i32;
    fn DescribePixelFormat(dc: *mut c_void, index: i32, size: u32, descriptor: *mut u8) -> i32;
}

struct Query {
    pid: u32,
    rows: usize,
    failed: bool,
    output: String,
}

pub fn read(pid: u32) -> Result<String, String> {
    let mut query = Query {
        pid,
        rows: 0,
        failed: false,
        output: "windowFormats\nwindowIndex\tvisible\tclientWidth\tclientHeight\tpixelFormat\tcolorBits\talphaBits\tdepthBits\tstencilBits\tdoubleBuffered\tgenericSoftware\n".into(),
    };
    let result = unsafe { EnumWindows(window, &mut query as *mut _ as isize) };
    if result == 0 || query.failed {
        return Err("自身窗口格式查询不完整或超过64窗上限".into());
    }
    Ok(query.output)
}

unsafe extern "system" fn window(window: *mut c_void, arg: isize) -> i32 {
    let query = unsafe { &mut *(arg as *mut Query) };
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(window, &mut pid) };
    if pid != query.pid {
        return 1;
    }
    if query.rows >= 64 {
        query.failed = true;
        return 0;
    }
    let mut rect = Rect::default();
    if unsafe { GetClientRect(window, &mut rect) } == 0 {
        query.failed = true;
        return 0;
    }
    let dc = unsafe { GetDC(window) };
    if dc.is_null() {
        query.failed = true;
        return 0;
    }
    let index = unsafe { GetPixelFormat(dc) };
    // PIXELFORMATDESCRIPTOR has a fixed 40-byte Win32 ABI; only scalar fields are read.
    let mut descriptor = [0u8; 40];
    let described =
        index == 0 || unsafe { DescribePixelFormat(dc, index, 40, descriptor.as_mut_ptr()) } != 0;
    unsafe { ReleaseDC(window, dc) };
    if !described {
        query.failed = true;
        return 0;
    }
    let flags = u32::from_le_bytes(descriptor[4..8].try_into().unwrap());
    let bits = |offset: usize| {
        if index == 0 {
            "unknown".to_owned()
        } else {
            descriptor[offset].to_string()
        }
    };
    let flag = |mask: u32| {
        if index == 0 {
            "unknown".to_owned()
        } else {
            (flags & mask != 0).to_string()
        }
    };
    query.output.push_str(&format!(
        "{}\t{}\t{}\t{}\t{index}\t{}\t{}\t{}\t{}\t{}\t{}\n",
        query.rows,
        unsafe { IsWindowVisible(window) } != 0,
        rect.right - rect.left,
        rect.bottom - rect.top,
        bits(9),
        bits(16),
        bits(23),
        bits(24),
        flag(1),
        flag(64),
    ));
    query.rows += 1;
    1
}
