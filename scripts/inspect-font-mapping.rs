use std::{
    collections::HashSet,
    ffi::c_void,
    path::{Path, PathBuf},
};

#[repr(C)]
#[derive(Default)]
struct Region {
    base: *mut c_void,
    allocation_base: *mut c_void,
    allocation_protect: u32,
    partition: u16,
    size: usize,
    state: u32,
    protect: u32,
    kind: u32,
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
    fn CloseHandle(handle: *mut c_void) -> i32;
    fn QueryFullProcessImageNameW(
        handle: *mut c_void,
        flags: u32,
        name: *mut u16,
        size: *mut u32,
    ) -> i32;
    fn VirtualQueryEx(
        handle: *mut c_void,
        address: *const c_void,
        region: *mut Region,
        size: usize,
    ) -> usize;
    fn GetLastError() -> u32;
}
#[link(name = "psapi")]
unsafe extern "system" {
    fn GetMappedFileNameW(
        handle: *mut c_void,
        address: *mut c_void,
        name: *mut u16,
        size: u32,
    ) -> u32;
}
struct Handle(*mut c_void);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
fn inspect(pid: u32, expected: &Path) -> Result<String, String> {
    if std::mem::size_of::<usize>() != 8 {
        return Err("验证工具要求 64 位构建".into());
    }
    let handle = unsafe { OpenProcess(0x0400 | 0x0010, 0, pid) };
    if handle.is_null() {
        return Err("无法打开目标进程".into());
    }
    let handle = Handle(handle);
    let mut name = vec![0u16; 32768];
    let mut len = name.len() as u32;
    if unsafe { QueryFullProcessImageNameW(handle.0, 0, name.as_mut_ptr(), &mut len) } == 0 {
        return Err("无法核实进程可执行路径".into());
    }
    let actual = PathBuf::from(String::from_utf16_lossy(&name[..len as usize]));
    if std::fs::canonicalize(actual).map_err(|e| e.to_string())?
        != std::fs::canonicalize(expected).map_err(|e| e.to_string())?
    {
        return Err("目标进程不属于指定的项目程序，停止查询".into());
    }
    let mut address = 0usize;
    let mut views = HashSet::new();
    let mut fonts = Vec::new();
    let mut visited = 0;
    let mut end_error = None;
    let mut mapped_bytes = 0usize;
    while visited < 20000 {
        let mut region = Region::default();
        if unsafe {
            VirtualQueryEx(
                handle.0,
                address as *const _,
                &mut region,
                std::mem::size_of::<Region>(),
            )
        } == 0
        {
            end_error = Some(unsafe { GetLastError() });
            break;
        }
        visited += 1;
        if region.kind == 0x40000 && region.state == 0x1000 {
            let len = unsafe {
                GetMappedFileNameW(handle.0, region.base, name.as_mut_ptr(), name.len() as u32)
            } as usize;
            if len > 0 && len < name.len() {
                let path = String::from_utf16_lossy(&name[..len]);
                let file = path.rsplit('\\').next().unwrap_or("").to_ascii_lowercase();
                if ["msyh.ttc", "msyh.ttf", "simhei.ttf", "simsun.ttc"].contains(&file.as_str()) {
                    mapped_bytes += region.size;
                    views.insert(region.allocation_base as usize);
                    fonts.push(format!(
                        "{{\"name\":\"{file}\",\"bytes\":{},\"readOnly\":{}}}",
                        region.size,
                        region.protect & 0xff == 2
                    ));
                }
            }
        }
        let next = (region.base as usize)
            .checked_add(region.size)
            .ok_or("地址越界")?;
        if next <= address {
            return Err("地址查询未前进".into());
        }
        address = next;
    }
    if visited == 20000 {
        return Err("超过查询上限，未完成地址空间检查".into());
    }
    if end_error != Some(87) {
        return Err(format!("地址空间查询异常：{end_error:?}"));
    }
    Ok(format!(
        "{{\"pid\":{pid},\"verifiedProjectExecutable\":true,\"regionsVisited\":{visited},\"fontViews\":{},\"mappedBytes\":{mapped_bytes},\"fonts\":[{}]}}",
        views.len(),
        fonts.join(",")
    ))
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let result = (|| {
        if args.len() != 4 {
            return Err("参数：PID 项目程序路径 输出文件".to_string());
        }
        let text = inspect(
            args[1].parse().map_err(|_| "无效 PID")?,
            Path::new(&args[2]),
        )?;
        std::fs::write(&args[3], text).map_err(|e| e.to_string())
    })();
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
