//! Pure parts of the system load reading (no Windows or Tauri types), so they can be tested anywhere.
//!
//! CPU: `GetSystemTimes` returns idle, kernel and user times in 100 ns ticks; the kernel time includes the idle time
//! (Microsoft, GetSystemTimes remarks), so busy = kernel + user - idle over the interval between two samples.
//! GPU: the PDH counter `\GPU Engine(*)\Utilization Percentage` has one instance per process and engine. Task Manager takes the
//! busiest engine of an adapter as the adapter's load (Microsoft DirectX blog, "GPUs in the Task Manager"), and the sum of the
//! instances of one engine type across processes is that engine's load.

use std::collections::HashMap;

/// One reading of `GetSystemTimes`, in ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuTimes {
    pub idle: u64,
    pub kernel: u64,
    pub user: u64,
}

/// CPU load in percent between two samples; `None` when the counters did not advance or went backwards.
pub fn cpu_percent(prev: CpuTimes, now: CpuTimes) -> Option<f64> {
    let idle = now.idle.checked_sub(prev.idle)?;
    let kernel = now.kernel.checked_sub(prev.kernel)?;
    let user = now.user.checked_sub(prev.user)?;
    let total = kernel.checked_add(user)?;
    if total == 0 {
        return None;
    }
    let busy = total.saturating_sub(idle);
    Some((busy as f64 / total as f64 * 100.0).clamp(0.0, 100.0))
}

/// What a GPU Engine instance name says: `pid_1234_luid_0x00000000_0x0000A1B2_phys_0_eng_0_engtype_3D`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuEngine {
    pub luid: String,
    pub phys: u32,
    /// engine index of the adapter (`eng_N`): the same type can exist several times (e.g. two video encoders)
    pub eng: u32,
    pub engtype: String,
}

pub fn parse_gpu_instance(name: &str) -> Option<GpuEngine> {
    let after = |key: &str| name.split_once(key).map(|(_, rest)| rest);
    let luid_part = after("_luid_")?;
    let mut parts = luid_part.splitn(3, '_');
    let (hi, lo) = (parts.next()?, parts.next()?);
    if !hi.starts_with("0x") || !lo.starts_with("0x") {
        return None;
    }
    let phys_part = after("_phys_")?;
    let phys: u32 = phys_part.split('_').next()?.parse().ok()?;
    let eng: u32 = after("_eng_")?.split('_').next()?.parse().ok()?;
    let engtype = after("_engtype_")?;
    if engtype.is_empty() {
        return None;
    }
    Some(GpuEngine { luid: format!("{hi}_{lo}"), phys, eng, engtype: engtype.to_string() })
}

/// Load of the busiest adapter: per adapter the busiest engine (the instances of one engine summed over processes), clamped to 0..100.
pub fn aggregate_gpu(items: &[(String, f64)]) -> Option<f64> {
    let mut engines: HashMap<(String, u32, u32, String), f64> = HashMap::new();
    for (name, value) in items {
        if !value.is_finite() || *value < 0.0 {
            continue;
        }
        if let Some(e) = parse_gpu_instance(name) {
            *engines.entry((e.luid, e.phys, e.eng, e.engtype)).or_insert(0.0) += value;
        }
    }
    let mut adapters: HashMap<(String, u32), f64> = HashMap::new();
    for ((luid, phys, _, _), sum) in engines {
        let slot = adapters.entry((luid, phys)).or_insert(0.0);
        if sum > *slot {
            *slot = sum;
        }
    }
    adapters.values().copied().fold(None, |best: Option<f64>, v| Some(best.map_or(v, |b| b.max(v)))).map(|v| v.clamp(0.0, 100.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(idle: u64, kernel: u64, user: u64) -> CpuTimes {
        CpuTimes { idle, kernel, user }
    }

    #[test]
    fn cpu_load_counts_kernel_time_minus_idle() {
        // 1000 ticks passed: kernel 600 (of which idle 500), user 400 -> busy 500
        assert_eq!(cpu_percent(t(0, 0, 0), t(500, 600, 400)), Some(50.0));
        assert_eq!(cpu_percent(t(10, 20, 30), t(10, 120, 30)), Some(100.0));
        assert_eq!(cpu_percent(t(0, 0, 0), t(1000, 1000, 0)), Some(0.0));
    }

    #[test]
    fn cpu_load_rejects_a_standing_or_backwards_counter() {
        assert_eq!(cpu_percent(t(5, 5, 5), t(5, 5, 5)), None);
        assert_eq!(cpu_percent(t(10, 10, 10), t(5, 20, 20)), None);
        // idle larger than the interval (sampling skew) clamps to zero instead of going negative
        assert_eq!(cpu_percent(t(0, 0, 0), t(120, 100, 0)), Some(0.0));
    }

    #[test]
    fn instance_names_are_parsed() {
        let e = parse_gpu_instance("pid_1234_luid_0x00000000_0x0000A1B2_phys_0_eng_0_engtype_3D").unwrap();
        assert_eq!(e, GpuEngine { luid: "0x00000000_0x0000A1B2".into(), phys: 0, eng: 0, engtype: "3D".into() });
        let v = parse_gpu_instance("pid_9_luid_0x00000000_0x0000A1B2_phys_1_eng_5_engtype_VideoEncode").unwrap();
        assert_eq!((v.phys, v.eng, v.engtype.as_str()), (1, 5, "VideoEncode"));
        for bad in ["", "pid_1", "pid_1_luid_0x0_0x1_phys_x_eng_0_engtype_3D", "pid_1_luid_1_2_phys_0_eng_0_engtype_3D", "pid_1_luid_0x0_0x1_phys_0_eng_0", "pid_1_luid_0x0_0x1_phys_0_eng_x_engtype_3D"] {
            assert!(parse_gpu_instance(bad).is_none(), "{bad}");
        }
    }

    fn inst(luid: &str, phys: u32, engtype: &str, pid: u32) -> String {
        format!("pid_{pid}_luid_0x00000000_{luid}_phys_{phys}_eng_0_engtype_{engtype}")
    }

    #[test]
    fn processes_add_up_per_engine_and_the_busiest_engine_wins() {
        let items = vec![
            (inst("0x0000000A", 0, "3D", 1), 30.0),
            (inst("0x0000000A", 0, "3D", 2), 25.0),
            (inst("0x0000000A", 0, "VideoEncode", 3), 40.0),
            (inst("0x0000000A", 0, "Copy", 3), 5.0),
        ];
        assert_eq!(aggregate_gpu(&items), Some(55.0));
    }

    #[test]
    fn engines_of_one_type_are_not_added_together() {
        // two video encoders at 40 % each are two busy engines, not one engine at 80 %
        let a = "pid_1_luid_0x00000000_0x0000000A_phys_0_eng_3_engtype_VideoEncode".to_string();
        let b = "pid_1_luid_0x00000000_0x0000000A_phys_0_eng_4_engtype_VideoEncode".to_string();
        assert_eq!(aggregate_gpu(&[(a, 40.0), (b, 40.0)]), Some(40.0));
    }

    #[test]
    fn the_busiest_adapter_is_reported_and_values_are_clamped() {
        let items = vec![
            (inst("0x0000000A", 0, "3D", 1), 10.0),
            (inst("0x0000000B", 0, "3D", 1), 70.0),
            (inst("0x0000000B", 0, "Compute", 2), 90.0),
            (inst("0x0000000C", 0, "3D", 1), 130.0),
        ];
        assert_eq!(aggregate_gpu(&items), Some(100.0));
        assert_eq!(aggregate_gpu(&items[..3]), Some(90.0));
    }

    #[test]
    fn nothing_usable_gives_nothing() {
        assert_eq!(aggregate_gpu(&[]), None);
        assert_eq!(aggregate_gpu(&[("garbage".into(), 50.0)]), None);
        assert_eq!(aggregate_gpu(&[(inst("0x1", 0, "3D", 1), f64::NAN)]), None);
    }
}
