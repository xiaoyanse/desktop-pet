use super::{gpu::Gpus, pdh::Counters, processes::Processes};
use crate::infrastructure::windows::native::{from_wide, wide};
use crate::{
    domain::{
        config::{Config, Metric},
        resources::{Device, Reading, Snapshot},
    },
    interfaces::ResourceSampler,
};
use std::{
    collections::HashMap,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use windows_sys::Win32::{
    Foundation::FILETIME,
    NetworkManagement::IpHelper::*,
    Storage::FileSystem::*,
    System::{Power::*, ProcessStatus::*, SystemInformation::*, Threading::*},
};

pub struct Sampler {
    cpu: Option<(u64, u64, u64)>,
    gpu: Option<Gpus>,
    disk: Option<Counters>,
    network: HashMap<u32, (u64, u64, Instant)>,
    processes: Processes,
}
impl Sampler {
    pub fn new() -> Self {
        Self {
            cpu: None,
            gpu: None,
            disk: None,
            network: HashMap::new(),
            processes: Processes::new(),
        }
    }
}
impl ResourceSampler for Sampler {
    fn sample(&mut self, config: &Config, expanded: bool) -> Snapshot {
        let mut snapshot = Snapshot {
            sampled_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            ..Default::default()
        };
        if config.enabled("CPU") {
            self.cpu(&mut snapshot);
        } else {
            self.cpu = None;
        }
        if config.enabled("内存") {
            memory(&mut snapshot);
        }
        if config.enabled("GPU") {
            self.gpu
                .get_or_insert_with(Gpus::new)
                .sample(config, &mut snapshot);
        } else {
            self.gpu = None;
        }
        if config.enabled("磁盘") {
            self.disks(config, &mut snapshot);
        } else {
            self.disk = None;
        }
        if config.enabled("网络") {
            self.network(config, &mut snapshot);
        } else {
            self.network.clear();
        }
        if expanded {
            self.processes.sample(&mut snapshot);
        } else {
            self.processes.reset();
        }
        snapshot
    }
}
impl Sampler {
    fn cpu(&mut self, snapshot: &mut Snapshot) {
        unsafe {
            let (mut idle, mut kernel, mut user) = (
                FILETIME::default(),
                FILETIME::default(),
                FILETIME::default(),
            );
            let reading = if GetSystemTimes(&mut idle, &mut kernel, &mut user) != 0 {
                let next = (filetime(idle), filetime(kernel), filetime(user));
                let value = self.cpu.and_then(|prev| {
                    let all = next.1.saturating_sub(prev.1) + next.2.saturating_sub(prev.2);
                    (all > 0).then(|| {
                        100.0 * (all.saturating_sub(next.0.saturating_sub(prev.0))) as f64
                            / all as f64
                    })
                });
                self.cpu = Some(next);
                value
                    .map(|v| Reading::percent(v, "GetSystemTimes，全部逻辑处理器平均"))
                    .unwrap_or_else(|| Reading::missing("正在采集首个 CPU 差分样本"))
            } else {
                Reading::missing("CPU 时间查询失败")
            };
            snapshot.values.insert(Metric::Cpu, reading);
            let count = GetActiveProcessorCount(ALL_PROCESSOR_GROUPS);
            let mut info = vec![PROCESSOR_POWER_INFORMATION::default(); count as usize];
            let reading = if count > 0
                && CallNtPowerInformation(
                    ProcessorInformation,
                    std::ptr::null(),
                    0,
                    info.as_mut_ptr().cast(),
                    std::mem::size_of_val(info.as_slice()) as u32,
                ) >= 0
            {
                let mhz = info.iter().map(|i| i.CurrentMhz as f64).sum::<f64>() / count as f64;
                if mhz > 0.0 {
                    Reading::number(
                        mhz / 1000.0,
                        format!("{:.2} GHz", mhz / 1000.0),
                        None,
                        "系统报告的各核心当前频率平均；可能不同于任务管理器有效频率",
                    )
                } else {
                    Reading::missing("系统未返回 CPU 频率")
                }
            } else {
                Reading::missing("CPU 电源信息接口不可用")
            };
            snapshot.values.insert(Metric::Frequency, reading);
        }
    }
    fn disks(&mut self, config: &Config, snapshot: &mut Snapshot) {
        let drives = unsafe { GetLogicalDrives() };
        for index in 0..26 {
            if drives & (1 << index) == 0 {
                continue;
            }
            let id = format!("{}:", (b'A' + index) as char);
            if unsafe { GetDriveTypeW(wide(format!("{id}\\")).as_ptr()) } == 3 {
                snapshot.disks.push(Device {
                    id: id.clone(),
                    name: id,
                });
            }
        }
        let id = if config.disk.is_empty() {
            snapshot
                .disks
                .first()
                .map(|d| d.id.as_str())
                .unwrap_or("C:")
        } else {
            &config.disk
        };
        let (mut free, mut total, mut available) = (0u64, 0u64, 0u64);
        unsafe {
            if GetDiskFreeSpaceExW(
                wide(format!("{id}\\")).as_ptr(),
                &mut available,
                &mut total,
                &mut free,
            ) != 0
                && total > 0
            {
                let used = total.saturating_sub(free);
                snapshot.values.insert(
                    Metric::DiskSpace,
                    Reading::percent(
                        100.0 * used as f64 / total as f64,
                        &format!("卷 {id} 空间使用率（容量），与磁盘活动率不同"),
                    ),
                );
                snapshot.values.insert(
                    Metric::DiskFree,
                    Reading::gib(free, None, &format!("卷 {id} 剩余空间")),
                );
            } else {
                for metric in [Metric::DiskSpace, Metric::DiskFree] {
                    snapshot
                        .values
                        .insert(metric, Reading::missing("所选卷未连接或容量查询失败"));
                }
            }
        }
        let counter = self.disk.get_or_insert_with(|| {
            Counters::new(&[
                ("read", r"\LogicalDisk(*)\Disk Read Bytes/sec"),
                ("write", r"\LogicalDisk(*)\Disk Write Bytes/sec"),
                ("idle", r"\LogicalDisk(*)\% Idle Time"),
            ])
        });
        counter.collect();
        for (metric, key) in [
            (Metric::DiskRead, "read"),
            (Metric::DiskWrite, "write"),
            (Metric::DiskBusy, "idle"),
        ] {
            let value = counter.values(key).and_then(|rows| {
                rows.into_iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(id))
                    .map(|(_, v)| v)
            });
            snapshot.values.insert(
                metric,
                value
                    .map(|v| {
                        if metric == Metric::DiskBusy {
                            Reading::percent(
                                (100.0 - v).clamp(0.0, 100.0),
                                &format!("卷 {id}，100% 减空闲时间；与物理磁盘合计可能不同"),
                            )
                        } else {
                            Reading::rate(v, &format!("卷 {id}，PDH 实际字节率"))
                        }
                    })
                    .unwrap_or_else(|| Reading::missing("磁盘性能计数器不可用或正在预热")),
            );
        }
    }
    fn network(&mut self, config: &Config, snapshot: &mut Snapshot) {
        unsafe {
            let mut table = std::ptr::null_mut();
            if GetIfTable2(&mut table) != 0 || table.is_null() {
                for metric in [Metric::Upload, Metric::Download] {
                    snapshot
                        .values
                        .insert(metric, Reading::missing("网卡查询失败"));
                }
                return;
            }
            let now = Instant::now();
            let rows =
                std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize);
            let mut automatic = Vec::new();
            let mut selected = None;
            let mut current = HashMap::new();
            for row in rows {
                if row.Type == 24
                    || row.Type == 131
                    || row.InterfaceAndOperStatusFlags._bitfield & 2 != 0
                {
                    continue;
                }
                let id = row.InterfaceIndex.to_string();
                let alias = from_wide(&row.Alias);
                let active = row.OperStatus == 1;
                snapshot.networks.push(Device {
                    id: id.clone(),
                    name: format!("{}{}", alias, if active { "" } else { "（断开）" }),
                });
                let rate =
                    self.network
                        .get(&row.InterfaceIndex)
                        .and_then(|&(input, output, prev)| {
                            let seconds = now.duration_since(prev).as_secs_f64();
                            (seconds > 0.0 && row.InOctets >= input && row.OutOctets >= output)
                                .then(|| {
                                    (
                                        (row.InOctets - input) as f64 / seconds,
                                        (row.OutOctets - output) as f64 / seconds,
                                    )
                                })
                        });
                current.insert(row.InterfaceIndex, (row.InOctets, row.OutOctets, now));
                if id == config.network {
                    selected = Some((rate, alias.clone()));
                }
                // Automatic mode sums only connected physical interfaces, excluding VMware/VPN duplication.
                if active && row.InterfaceAndOperStatusFlags._bitfield & 1 != 0 {
                    automatic.push(rate);
                }
            }
            self.network = current;
            let (rates, source) = if config.network.is_empty() {
                (
                    if automatic.is_empty() || automatic.iter().any(Option::is_none) {
                        None
                    } else {
                        Some(
                            automatic
                                .into_iter()
                                .flatten()
                                .fold((0.0, 0.0), |a, b| (a.0 + b.0, a.1 + b.1)),
                        )
                    },
                    "已连接的物理网卡合计".into(),
                )
            } else {
                selected.unwrap_or((None, "所选网卡未连接".into()))
            };
            FreeMibTable(table.cast());
            for (metric, n) in [(Metric::Download, 0), (Metric::Upload, 1)] {
                snapshot.values.insert(
                    metric,
                    rates
                        .map(|r| Reading::rate(if n == 0 { r.0 } else { r.1 }, &source))
                        .unwrap_or_else(|| Reading::missing("等待网卡差分样本，或所选网卡不可用")),
                );
            }
        }
    }
}
pub fn filetime(t: FILETIME) -> u64 {
    ((t.dwHighDateTime as u64) << 32) | t.dwLowDateTime as u64
}
pub fn memory(snapshot: &mut Snapshot) {
    unsafe {
        let mut m = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        if GlobalMemoryStatusEx(&mut m) != 0 {
            snapshot.values.insert(
                Metric::Memory,
                Reading::percent(
                    100.0 * (m.ullTotalPhys - m.ullAvailPhys) as f64 / m.ullTotalPhys as f64,
                    "物理内存；可用包含可立即复用的待机缓存",
                ),
            );
            snapshot.values.insert(
                Metric::Used,
                Reading::gib(
                    m.ullTotalPhys - m.ullAvailPhys,
                    Some(m.ullTotalPhys),
                    "已用物理内存",
                ),
            );
            snapshot.values.insert(
                Metric::Available,
                Reading::gib(
                    m.ullAvailPhys,
                    Some(m.ullTotalPhys),
                    "可用物理内存，包含待机缓存",
                ),
            );
        } else {
            for metric in [Metric::Memory, Metric::Used, Metric::Available] {
                snapshot
                    .values
                    .insert(metric, Reading::missing("物理内存查询失败"));
            }
        }
        let mut p = PERFORMANCE_INFORMATION::default();
        if GetPerformanceInfo(&mut p, std::mem::size_of_val(&p) as u32) != 0 {
            for (metric, pages) in [
                (Metric::Commit, p.CommitTotal),
                (Metric::Paged, p.KernelPaged),
                (Metric::Nonpaged, p.KernelNonpaged),
            ] {
                snapshot.values.insert(
                    metric,
                    Reading::gib(
                        (pages * p.PageSize) as u64,
                        (metric == Metric::Commit).then_some((p.CommitLimit * p.PageSize) as u64),
                        if metric == Metric::Commit {
                            "系统提交内存；包括物理内存和分页文件的承诺，并非实际物理占用"
                        } else {
                            "GetPerformanceInfo：系统内核池页数 × 页大小"
                        },
                    ),
                );
            }
        } else {
            for metric in [Metric::Commit, Metric::Paged, Metric::Nonpaged] {
                snapshot
                    .values
                    .insert(metric, Reading::missing("内核内存查询失败"));
            }
        }
    }
}
