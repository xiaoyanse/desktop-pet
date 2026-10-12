use std::{
    collections::BTreeMap,
    ffi::c_void,
    fs::OpenOptions,
    io::Write,
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
    fn GetLastError() -> u32;
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
}
#[link(name = "psapi")]
unsafe extern "system" {
    fn QueryWorkingSet(handle: *mut c_void, buffer: *mut c_void, size: u32) -> i32;
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
        return Err("需要64位工具".into());
    }
    let handle = unsafe { OpenProcess(0x0400 | 0x0010, 0, pid) };
    if handle.is_null() {
        return Err("无法打开目标进程".into());
    }
    let handle = Handle(handle);
    let mut name = vec![0u16; 32768];
    let mut len = name.len() as u32;
    if unsafe { QueryFullProcessImageNameW(handle.0, 0, name.as_mut_ptr(), &mut len) } == 0 {
        return Err("无法核实进程来源".into());
    }
    let actual = PathBuf::from(String::from_utf16_lossy(&name[..len as usize]));
    if std::fs::canonicalize(actual).map_err(|e| e.to_string())?
        != std::fs::canonicalize(expected).map_err(|e| e.to_string())?
    {
        return Err("目标不属于指定的项目程序".into());
    }
    let mut buffer = vec![0usize; 8193];
    loop {
        if unsafe {
            QueryWorkingSet(
                handle.0,
                buffer.as_mut_ptr().cast(),
                (buffer.len() * 8) as u32,
            )
        } != 0
        {
            break;
        }
        let error = unsafe { GetLastError() };
        if error != 24 && error != 122 {
            return Err(format!("工作集查询失败：{error}"));
        }
        if buffer.len() >= 262145 {
            return Err("工作集超过工具的1 GiB查询上限".into());
        }
        buffer.resize(((buffer.len() - 1) * 2 + 1).min(262145), 0);
    }
    let count = buffer[0];
    if count >= buffer.len() {
        return Err("工作集返回长度越界".into());
    }
    let mut pages = buffer[1..1 + count].to_vec();
    pages.sort_unstable_by_key(|entry| entry & !0xfff);
    let mut summaries: BTreeMap<String, [usize; 2]> = BTreeMap::new();
    let mut region = Region::default();
    let mut region_end = 0usize;
    let mut label = String::new();
    for entry in pages {
        let address = entry & !0xfff;
        if address >= region_end || address < (region.base as usize) {
            region = Region::default();
            if unsafe {
                VirtualQueryEx(
                    handle.0,
                    address as *const _,
                    &mut region,
                    std::mem::size_of::<Region>(),
                )
            } == 0
            {
                return Err("查询期间地址布局发生变化，拒绝将不完整数据作为归因".into());
            }
            region_end = (region.base as usize)
                .checked_add(region.size)
                .ok_or("区域越界")?;
            let category = match region.kind {
                0x20000 => "private",
                0x40000 => "mapped",
                0x1000000 => "image",
                _ => "other",
            };
            label = category.into();
            if category == "mapped" || category == "image" {
                let len = unsafe {
                    GetMappedFileNameW(handle.0, region.base, name.as_mut_ptr(), name.len() as u32)
                } as usize;
                if len > 0 && len < name.len() {
                    let path = String::from_utf16_lossy(&name[..len]);
                    let file = path.rsplit('\\').next().unwrap_or("unknown");
                    label = format!(
                        "{category}/{}",
                        file.chars().filter(|c| !c.is_control()).collect::<String>()
                    );
                }
            }
        }
        let shared = ((entry >> 8) & 1) as usize;
        summaries.entry(label.clone()).or_default()[shared] += 4096;
    }
    let mut output = format!(
        "pid\t{pid}\nverifiedProjectExecutable\ttrue\nsnapshotWorkingSetBytes\t{}\ncategory\tprivateResidentBytes\tshareableResidentBytes\n",
        count * 4096
    );
    let mut summaries: Vec<_> = summaries.into_iter().collect();
    summaries.sort_unstable_by_key(|(_, bytes)| std::cmp::Reverse(bytes.iter().sum::<usize>()));
    for (name, [private, shared]) in summaries {
        output.push_str(&format!("{name}\t{private}\t{shared}\n"));
    }
    Ok(output)
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let result = (|| {
        if args.len() != 4 {
            return Err("参数：PID 预期项目EXE 尚不存在的TSV输出文件".to_string());
        }
        let text = inspect(args[1].parse().map_err(|_| "无效PID")?, Path::new(&args[2]))?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args[3])
            .map_err(|e| e.to_string())?
            .write_all(text.as_bytes())
            .map_err(|e| e.to_string())
    })();
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
