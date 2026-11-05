//! CPU and GPU load of this PC for the overview tiles, from native Windows counters only (no third-party program).
//!
//! CPU: `GetSystemTimes` (kernel32). GPU: the PDH counter `\GPU Engine(*)\Utilization Percentage` (the data Task Manager
//! shows, WDDM 2.0 drivers and newer). Both are rates, so the first sample has no value; the reading is kept between calls.

use serde::Serialize;

use crate::system_load_core::{aggregate_gpu, cpu_percent, CpuTimes};

#[derive(Debug, Clone, Default, Serialize)]
pub struct SystemLoad {
    /// busy share of all processors in percent, `None` until two samples exist
    pub cpu_percent: Option<f64>,
    /// load of the busiest GPU engine of the busiest adapter in percent, `None` when the counters are not there
    pub gpu_percent: Option<f64>,
}

/// Read the current CPU and GPU load (cheap: the last reading is reused when called again within 250 ms).
#[tauri::command]
pub async fn system_load() -> Result<SystemLoad, String> {
    tokio::task::spawn_blocking(sample).await.map_err(|e| e.to_string())
}

#[cfg(not(target_os = "windows"))]
fn sample() -> SystemLoad {
    SystemLoad::default()
}

#[cfg(target_os = "windows")]
fn sample() -> SystemLoad {
    use std::sync::Mutex;
    static SAMPLER: Mutex<Option<win::Sampler>> = Mutex::new(None);
    let mut guard = match SAMPLER.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    let sampler = guard.get_or_insert_with(win::Sampler::new);
    sampler.sample()
}

#[cfg(target_os = "windows")]
mod win {
    use std::time::{Duration, Instant};

    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
        PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER,
        PDH_HQUERY, PDH_MORE_DATA,
    };
    use windows::Win32::System::Threading::GetSystemTimes;

    use super::{aggregate_gpu, cpu_percent, CpuTimes, SystemLoad};

    /// PDH_FMT_NOCAP100: values above 100 (many processes on one engine) are not reset to 100; the sum is clamped later.
    const PDH_FMT_NOCAP100: u32 = 0x0000_8000;
    const MIN_INTERVAL: Duration = Duration::from_millis(250);

    pub struct Sampler {
        cpu_prev: Option<CpuTimes>,
        last: Option<Instant>,
        cpu: Option<f64>,
        gpu: Option<f64>,
        query: PDH_HQUERY,
        counter: PDH_HCOUNTER,
        pdh: bool,
    }

    // The PDH handles are only used behind the mutex in `sample()`.
    unsafe impl Send for Sampler {}

    impl Sampler {
        pub fn new() -> Self {
            let mut query = PDH_HQUERY(std::ptr::null_mut());
            let mut counter = PDH_HCOUNTER(std::ptr::null_mut());
            let mut pdh = false;
            unsafe {
                if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) == 0 {
                    pdh = PdhAddEnglishCounterW(query, w!("\\GPU Engine(*)\\Utilization Percentage"), 0, &mut counter) == 0;
                    if !pdh {
                        PdhCloseQuery(query);
                        query = PDH_HQUERY(std::ptr::null_mut());
                    }
                }
            }
            Self { cpu_prev: None, last: None, cpu: None, gpu: None, query, counter, pdh }
        }

        pub fn sample(&mut self) -> SystemLoad {
            if self.last.map_or(false, |t| t.elapsed() < MIN_INTERVAL) {
                return SystemLoad { cpu_percent: self.cpu, gpu_percent: self.gpu };
            }
            self.last = Some(Instant::now());
            if let Some(now) = read_cpu_times() {
                if let Some(prev) = self.cpu_prev {
                    self.cpu = cpu_percent(prev, now).or(self.cpu);
                }
                self.cpu_prev = Some(now);
            }
            if self.pdh {
                self.gpu = self.read_gpu();
            }
            SystemLoad { cpu_percent: self.cpu, gpu_percent: self.gpu }
        }

        /// Two collections are needed before a rate counter has a value; until then (or when the counter set is missing,
        /// as on drivers older than WDDM 2.0) this returns `None`.
        fn read_gpu(&mut self) -> Option<f64> {
            // SAFETY: the query and counter handles are valid while `pdh` is set; the item buffer is sized by the first call.
            unsafe {
                if PdhCollectQueryData(self.query) != 0 {
                    return None;
                }
                let format = PDH_FMT(PDH_FMT_DOUBLE.0 | PDH_FMT_NOCAP100);
                let mut size: u32 = 0;
                let mut count: u32 = 0;
                if PdhGetFormattedCounterArrayW(self.counter, format, &mut size, &mut count, None) != PDH_MORE_DATA || size == 0 {
                    return None;
                }
                // 8-byte aligned storage for the item array and the instance names that follow it
                let mut storage: Vec<u64> = vec![0; (size as usize + 7) / 8];
                let items = storage.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
                if PdhGetFormattedCounterArrayW(self.counter, format, &mut size, &mut count, Some(items)) != 0 {
                    return None;
                }
                let list = std::slice::from_raw_parts(items, count as usize);
                let mut readings: Vec<(String, f64)> = Vec::with_capacity(list.len());
                for item in list {
                    let status = item.FmtValue.CStatus;
                    if status != PDH_CSTATUS_VALID_DATA && status != PDH_CSTATUS_NEW_DATA {
                        continue;
                    }
                    if let Ok(name) = item.szName.to_string() {
                        readings.push((name, item.FmtValue.Anonymous.doubleValue));
                    }
                }
                aggregate_gpu(&readings)
            }
        }
    }

    impl Drop for Sampler {
        fn drop(&mut self) {
            if self.pdh {
                unsafe { PdhCloseQuery(self.query) };
            }
        }
    }

    fn read_cpu_times() -> Option<CpuTimes> {
        let (mut idle, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default());
        unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.ok()?;
        let ticks = |f: FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
        Some(CpuTimes { idle: ticks(idle), kernel: ticks(kernel), user: ticks(user) })
    }
}
