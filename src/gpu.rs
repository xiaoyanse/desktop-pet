use crate::{model::{Device, GpuReading, Reading, Snapshot}, config::{Config, Metric}, pdh::Counters};
use std::{collections::HashMap, time::{Duration, Instant}};
use windows::{Win32::{Foundation::LUID, Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE, DXGI_ADAPTER_FLAG_REMOTE}}, Wdk::Graphics::Direct3D::*};

struct Adapter { device: Device, dedicated: u64 }
pub struct Gpus { counters: Counters, adapters: Vec<Adapter>, enumerated: Instant }
impl Gpus {
    pub fn new() -> Self {
        Self { adapters: enumerate(), enumerated: Instant::now(), counters: Counters::new(&[
            ("engine", r"\GPU Engine(*)\Utilization Percentage"),
            ("dedicated", r"\GPU Adapter Memory(*)\Dedicated Usage"),
            ("shared", r"\GPU Adapter Memory(*)\Shared Usage"),
        ]) }
    }
    pub fn sample(&mut self, config: &Config, snapshot: &mut Snapshot) {
        if self.enumerated.elapsed() >= Duration::from_secs(30) {
            self.adapters = enumerate(); self.enumerated = Instant::now();
        }
        snapshot.gpus = self.adapters.iter().map(|a| a.device.clone()).collect();
        self.counters.collect();
        let engines = self.counters.values("engine");
        let dedicated = self.counters.values("dedicated");
        let shared = self.counters.values("shared");
        for adapter in self.adapters.iter().filter(|a| config.gpu.is_empty() || a.device.id == config.gpu) {
            let id = &adapter.device.id;
            let source = format!("{}；GPU 占用采用最繁忙引擎，各进程在同一引擎上的占用相加", adapter.device.name);
            let usage = engines.as_ref().and_then(|rows| busiest_engine(rows, id));
            let mut values = HashMap::new();
            values.insert(Metric::Gpu, usage.map(|v| Reading::percent(v, &source)).unwrap_or_else(|| Reading::missing("GPU 性能计数器不可用或正在预热")));
            for (metric, rows) in [(Metric::Dedicated, &dedicated), (Metric::Shared, &shared)] {
                let bytes = rows.as_ref().and_then(|rows| {
                    let matched: Vec<_> = rows.iter().filter(|(name, _)| name.to_ascii_lowercase().contains(id)).collect();
                    (!matched.is_empty()).then(|| matched.iter().map(|(_, v)| *v).sum::<f64>() as u64)
                });
                let total = (metric == Metric::Dedicated && adapter.dedicated > 512 * 1024 * 1024).then_some(adapter.dedicated);
                values.insert(metric, bytes.map(|v| Reading::gib(v, total, &adapter.device.name)).unwrap_or_else(|| Reading::missing("显存计数器不可用")));
            }
            snapshot.gpu_values.push(GpuReading { device: adapter.device.clone(), values });
        }
        if let Some(first) = snapshot.gpu_values.first() { snapshot.values.extend(first.values.clone()); }
        else {
            for metric in [Metric::Gpu, Metric::Dedicated, Metric::Shared] { snapshot.values.insert(metric, Reading::missing("所选显卡未连接或枚举失败")); }
        }
    }
}
fn enumerate() -> Vec<Adapter> {
        let mut adapters = Vec::new();
        unsafe {
            if let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() {
                for n in 0..32 {
                    let Ok(adapter) = factory.EnumAdapters1(n) else { break; };
                    if let Ok(d) = adapter.GetDesc1() {
                        if d.Flags & (DXGI_ADAPTER_FLAG_SOFTWARE.0 | DXGI_ADAPTER_FLAG_REMOTE.0) as u32 != 0 || !hardware_adapter(d.AdapterLuid) { continue; }
                        let id = format!("luid_0x{:08x}_0x{:08x}", d.AdapterLuid.HighPart as u32, d.AdapterLuid.LowPart);
                        adapters.push(Adapter { device: Device { id, name: crate::native::from_wide(&d.Description) }, dedicated: d.DedicatedVideoMemory as u64 });
                    }
                }
            }
        }
        adapters
}
fn hardware_flags(flags: u32) -> bool {
    // WDDM ADAPTERTYPE: SoftwareDevice=bit 2, IndirectDisplayDevice=bit 6.
    flags & (4 | 64) == 0
}
fn hardware_adapter(luid: LUID) -> bool {
    unsafe {
        let mut open = D3DKMT_OPENADAPTERFROMLUID { AdapterLuid: luid, hAdapter: 0 };
        if D3DKMTOpenAdapterFromLuid(&mut open).0 < 0 { return true; }
        let mut flags = 0u32;
        let mut query = D3DKMT_QUERYADAPTERINFO { hAdapter: open.hAdapter, Type: KMTQAITYPE_ADAPTERTYPE,
            pPrivateDriverData: (&mut flags as *mut u32).cast(), PrivateDriverDataSize: 4 };
        let status = D3DKMTQueryAdapterInfo(&mut query);
        let _ = D3DKMTCloseAdapter(&D3DKMT_CLOSEADAPTER { hAdapter: open.hAdapter });
        status.0 < 0 || hardware_flags(flags)
    }
}
pub fn busiest_engine(rows: &[(String, f64)], id: &str) -> Option<f64> {
    let mut engines = HashMap::<String, f64>::new();
    for (name, value) in rows {
        let name = name.to_ascii_lowercase();
        if !name.contains(id) { continue; }
        if let Some(engine) = name.find("_phys_") { *engines.entry(name[engine..].to_string()).or_default() += value; }
    }
    engines.values().copied().reduce(f64::max).map(|v| v.clamp(0.0, 100.0))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn indirect_and_software_adapters_are_not_physical_gpus() {
        assert!(hardware_flags(3 | 32));
        assert!(hardware_flags(1 | 16));
        assert!(!hardware_flags(3 | 64));
        assert!(!hardware_flags(4));
    }
    #[test]
    fn sum_processes_per_engine_but_do_not_sum_independent_engines() {
        let rows = vec![("pid_1_luid_a_phys_0_eng_0_engtype_3d".into(), 20.0),
            ("pid_2_luid_a_phys_0_eng_0_engtype_3d".into(), 30.0),
            ("pid_1_luid_a_phys_0_eng_1_engtype_copy".into(), 40.0),
            ("pid_1_luid_b_phys_0_eng_0_engtype_3d".into(), 99.0)];
        assert_eq!(busiest_engine(&rows, "luid_a"), Some(50.0));
        assert_eq!(busiest_engine(&rows, "luid_c"), None);
    }
}
