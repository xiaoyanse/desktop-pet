use crate::{metrics::filetime, model::{Process, Snapshot}, native::{Handle, from_wide}};
use std::{collections::HashMap, time::Instant};
use windows_sys::Win32::{Foundation::FILETIME, System::{Diagnostics::ToolHelp::*, Threading::*, ProcessStatus::*}};

pub struct Processes { previous: HashMap<u32, (u64, u64)>, time: Option<Instant> }
impl Processes {
    pub fn new() -> Self { Self { previous: HashMap::new(), time: None } }
    pub fn reset(&mut self) { self.previous.clear(); self.time = None; }
    pub fn sample(&mut self, snapshot: &mut Snapshot) {
        let now = Instant::now();
        let dt = self.time.map(|time| now.duration_since(time).as_secs_f64());
        let mut next = HashMap::new(); let mut processes = Vec::new(); let mut skipped = 0;
        unsafe {
            let Some(list) = Handle::new(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)) else {
                snapshot.process_status = "进程快照失败".into(); return;
            };
            let mut entry = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
            let count = GetActiveProcessorCount(ALL_PROCESSOR_GROUPS).max(1) as f64;
            let mut present = Process32FirstW(list.0, &mut entry) != 0;
            while present {
                if entry.th32ProcessID != 0 {
                    let p = Handle::new(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, 0, entry.th32ProcessID));
                    if let Some(p) = p {
                        let (mut creation, mut exit, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
                        let mut m = PROCESS_MEMORY_COUNTERS::default();
                        if GetProcessTimes(p.0, &mut creation, &mut exit, &mut kernel, &mut user) != 0
                            && GetProcessMemoryInfo(p.0, &mut m, std::mem::size_of_val(&m) as u32) != 0 {
                            let born = filetime(creation); let ticks = filetime(kernel) + filetime(user);
                            let cpu = dt.and_then(|seconds| self.previous.get(&entry.th32ProcessID).and_then(|&(old_born, old_ticks)| {
                                (born == old_born && seconds > 0.0).then(|| ticks.saturating_sub(old_ticks) as f64 / 10_000_000.0 / seconds / count * 100.0)
                            })).unwrap_or(0.0).clamp(0.0, 100.0);
                            next.insert(entry.th32ProcessID, (born, ticks));
                            processes.push(Process { name: from_wide(&entry.szExeFile), pid: entry.th32ProcessID, cpu, memory: m.WorkingSetSize as f64 / 1073741824.0 });
                        } else { skipped += 1; }
                    } else { skipped += 1; }
                }
                present = Process32NextW(list.0, &mut entry) != 0;
            }
        }
        self.time = Some(now); self.previous = next;
        processes.sort_by(|a, b| b.memory.total_cmp(&a.memory));
        snapshot.memory_top = processes.iter().take(5).cloned().collect();
        if dt.is_some() {
            processes.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
            snapshot.cpu_top = processes.into_iter().take(5).collect();
        }
        snapshot.process_status = format!("物理工作集含共享页，不可相加；CPU 为全机平均。{skipped} 个进程受权限/退出限制{}", if dt.is_none() { "；CPU 正在预热" } else { "" });
    }
}
