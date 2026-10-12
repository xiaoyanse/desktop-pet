use crate::{
    domain::report::{Process, Report, privacy::SafePath, runtime_kind},
    infrastructure::windows::native::{Handle, from_wide},
};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::FILETIME,
    System::{Diagnostics::ToolHelp::*, ProcessStatus::*, Threading::*},
};

#[derive(Debug)]
pub struct Sample {
    name: String,
    pid: u32,
    parent_pid: u32,
    born: Option<u64>,
    ticks: Option<u64>,
    working: Option<u64>,
    private: Option<u64>,
}
fn ticks(time: FILETIME) -> u64 {
    ((time.dwHighDateTime as u64) << 32) | time.dwLowDateTime as u64
}
unsafe fn times(handle: windows_sys::Win32::Foundation::HANDLE) -> Option<(u64, u64)> {
    let (mut born, mut exit, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    (unsafe { GetProcessTimes(handle, &mut born, &mut exit, &mut kernel, &mut user) } != 0)
        .then(|| (ticks(born), ticks(kernel) + ticks(user)))
}
pub fn capture(cancel: &AtomicBool) -> Result<Vec<Sample>, String> {
    unsafe {
        let list = Handle::new(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0))
            .ok_or("进程快照创建失败，请重试")?;
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut present = Process32FirstW(list.0, &mut entry) != 0;
        let mut result = Vec::new();
        while present {
            if cancel.load(Ordering::Relaxed) {
                return Err("已取消采集".into());
            }
            if entry.th32ProcessID != 0 {
                let mut sample = Sample {
                    name: from_wide(&entry.szExeFile),
                    pid: entry.th32ProcessID,
                    parent_pid: entry.th32ParentProcessID,
                    born: None,
                    ticks: None,
                    working: None,
                    private: None,
                };
                let handle = Handle::new(OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ,
                    0,
                    sample.pid,
                ))
                .or_else(|| {
                    Handle::new(OpenProcess(
                        PROCESS_QUERY_LIMITED_INFORMATION,
                        0,
                        sample.pid,
                    ))
                });
                if let Some(handle) = handle {
                    if let Some((born, cpu)) = times(handle.0) {
                        sample.born = Some(born);
                        sample.ticks = Some(cpu);
                    }
                    let mut memory = PROCESS_MEMORY_COUNTERS_EX::default();
                    if GetProcessMemoryInfo(
                        handle.0,
                        (&mut memory as *mut PROCESS_MEMORY_COUNTERS_EX).cast(),
                        std::mem::size_of_val(&memory) as u32,
                    ) != 0
                    {
                        sample.working = Some(memory.WorkingSetSize as u64);
                        sample.private = Some(memory.PrivateUsage as u64);
                    }
                }
                result.push(sample);
            }
            present = Process32NextW(list.0, &mut entry) != 0;
        }
        if result.is_empty() {
            return Err("未取得进程列表，请重试".into());
        }
        Ok(result)
    }
}
fn safe_path(sample: &Sample) -> SafePath {
    unsafe {
        let Some(handle) = Handle::new(OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            sample.pid,
        )) else {
            return SafePath::unavailable();
        };
        if sample.born.is_none() || times(handle.0).map(|t| t.0) != sample.born {
            return SafePath::unavailable();
        }
        let mut buffer = vec![0u16; 32768];
        let mut length = buffer.len() as u32;
        if QueryFullProcessImageNameW(handle.0, 0, buffer.as_mut_ptr(), &mut length) == 0 {
            return SafePath::unavailable();
        }
        SafePath::from_raw(&String::from_utf16_lossy(&buffer[..length as usize]))
    }
}
pub fn assemble(
    first: Vec<Sample>,
    second: Vec<Sample>,
    elapsed: Duration,
    cancel: &AtomicBool,
) -> Result<Report, String> {
    let previous: HashMap<_, _> = first.iter().map(|p| (p.pid, p)).collect();
    let current: HashMap<_, _> = second.iter().map(|p| (p.pid, p)).collect();
    let cores = unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) }.max(1) as f64;
    let mut rows = Vec::with_capacity(second.len());
    for sample in &second {
        let cpu = previous
            .get(&sample.pid)
            .and_then(|old| {
                let born = sample.born?;
                if old.born != Some(born) || elapsed.is_zero() {
                    return None;
                }
                Some(
                    sample.ticks?.saturating_sub(old.ticks?) as f64
                        / 10_000_000.0
                        / elapsed.as_secs_f64()
                        / cores
                        * 100.0,
                )
            })
            .map(|v| v.clamp(0.0, 100.0));
        let parent = current.get(&sample.parent_pid).and_then(|p| {
            (p.born? <= sample.born? && p.pid != sample.pid).then(|| (p.pid, p.name.clone()))
        });
        rows.push(Process {
            name: sample.name.clone(),
            pid: sample.pid,
            parent,
            path: SafePath::unavailable(),
            cpu,
            working_set: sample.working,
            private: sample.private,
        });
    }
    let mut memory: Vec<_> = rows
        .iter()
        .filter(|p| p.working_set.is_some())
        .cloned()
        .collect();
    memory.sort_by(|a, b| b.working_set.cmp(&a.working_set).then(a.pid.cmp(&b.pid)));
    memory.truncate(5);
    let mut cpu: Vec<_> = rows.iter().filter(|p| p.cpu.is_some()).cloned().collect();
    cpu.sort_by(|a, b| {
        b.cpu
            .unwrap()
            .total_cmp(&a.cpu.unwrap())
            .then(a.pid.cmp(&b.pid))
    });
    cpu.truncate(5);
    let mut runtimes: Vec<_> = rows
        .iter()
        .filter(|p| runtime_kind(&p.name).is_some())
        .cloned()
        .collect();
    runtimes.sort_by(|a, b| b.private.cmp(&a.private).then(a.pid.cmp(&b.pid)));
    let omitted_runtimes = runtimes.len().saturating_sub(512);
    runtimes.truncate(512);
    let mut paths = HashMap::new();
    for p in memory
        .iter_mut()
        .chain(cpu.iter_mut())
        .chain(runtimes.iter_mut())
    {
        if cancel.load(Ordering::Relaxed) {
            return Err("已取消采集".into());
        }
        let path = paths
            .entry(p.pid)
            .or_insert_with(|| safe_path(current[&p.pid]));
        p.path = path.clone();
    }
    Ok(Report {
        cpu_top: cpu,
        memory_top: memory,
        runtimes,
        enumerated: rows.len(),
        unreadable: second
            .iter()
            .filter(|p| p.ticks.is_none() || p.private.is_none())
            .count(),
        omitted_runtimes,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(pid: u32, born: u64, cpu: u64, parent: u32) -> Sample {
        Sample {
            name: "python.exe".into(),
            pid,
            parent_pid: parent,
            born: Some(born),
            ticks: Some(cpu),
            working: Some(42),
            private: Some(24),
        }
    }
    #[test]
    fn reused_pid_never_inherits_cpu_or_a_younger_parent() {
        let first = vec![sample(900_001, 1, 0, 0)];
        let second = vec![
            sample(900_001, 2, 2_000_000, 900_002),
            sample(900_002, 3, 1, 0),
        ];
        let report = assemble(
            first,
            second,
            Duration::from_secs(2),
            &AtomicBool::new(false),
        )
        .unwrap();
        let child = report.runtimes.iter().find(|p| p.pid == 900_001).unwrap();
        assert!(child.cpu.is_none());
        assert!(child.parent.is_none());
        assert!(child.path.as_str().contains("不可读取"));
    }
    #[test]
    fn cancelled_enumeration_never_starts_a_second_pass() {
        assert!(
            capture(&AtomicBool::new(true))
                .unwrap_err()
                .contains("取消")
        );
    }
}
