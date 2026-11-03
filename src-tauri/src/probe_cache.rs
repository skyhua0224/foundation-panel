//! A short-lived cache for the status probes of the panel (VDD driver, virtual mouse, Sunshine version and start time, process memory).
//! Each probe starts a process or asks WMI (0.06 to 1.9 s on the Skyindows VM) and several pages ask for the same one at once; a probe
//! now runs once for all callers that ask while it runs, and its answer is kept for `ttl`. A command that changes what the probe reports
//! calls `invalidating()` first: the answer is dropped when the command starts and again when it ends, whichever way it ends, so a read
//! that runs in between cannot keep the old state. Failures are not kept.

use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// A probe that takes longer than this is written to the log (the pages wait for it).
const SLOW_PROBE: Duration = Duration::from_millis(250);

struct Entry<T> {
    at: Instant,
    epoch: u64,
    value: T,
}

pub struct ProbeCache<T> {
    name: &'static str,
    ttl: Duration,
    epoch: AtomicU64,
    slot: tokio::sync::Mutex<Option<Entry<T>>>,
}

/// Drops the cached answer now and when it goes out of scope.
pub struct Invalidating<'a, T>(&'a ProbeCache<T>);

impl<T> Drop for Invalidating<'_, T> {
    fn drop(&mut self) {
        self.0.invalidate();
    }
}

impl<T> ProbeCache<T> {
    pub const fn new(name: &'static str, ttl: Duration) -> Self {
        Self {
            name,
            ttl,
            epoch: AtomicU64::new(0),
            slot: tokio::sync::Mutex::const_new(None),
        }
    }

    pub fn invalidate(&self) {
        self.epoch.fetch_add(1, Ordering::AcqRel);
    }

    pub fn invalidating(&self) -> Invalidating<'_, T> {
        self.invalidate();
        Invalidating(self)
    }
}

impl<T: Clone> ProbeCache<T> {
    /// The kept answer while it is younger than `ttl` and nothing invalidated it; else `load()`, which runs once for every caller waiting.
    pub async fn get<F, Fut>(&self, load: F) -> Result<T, String>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, String>>,
    {
        let mut slot = self.slot.lock().await;
        let epoch = self.epoch.load(Ordering::Acquire);
        if let Some(entry) = slot.as_ref() {
            if entry.epoch == epoch && entry.at.elapsed() < self.ttl {
                return Ok(entry.value.clone());
            }
        }
        let started = Instant::now();
        let value = load().await?;
        let took = started.elapsed();
        if took >= SLOW_PROBE {
            log::warn!("probe {} took {} ms", self.name, took.as_millis());
        }
        *slot = Some(Entry {
            at: Instant::now(),
            epoch,
            value: value.clone(),
        });
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;

    fn run<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(future)
    }

    #[test]
    fn callers_that_ask_together_share_one_probe_and_a_fresh_answer_is_kept() {
        run(async {
            let cache = Arc::new(ProbeCache::new("test", Duration::from_secs(60)));
            let runs = Arc::new(AtomicUsize::new(0));
            let ask = |cache: Arc<ProbeCache<u32>>, runs: Arc<AtomicUsize>| async move {
                cache
                    .get(|| async {
                        runs.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(20)).await;
                        Ok(7)
                    })
                    .await
            };
            let (a, b, c) = tokio::join!(
                ask(cache.clone(), runs.clone()),
                ask(cache.clone(), runs.clone()),
                ask(cache.clone(), runs.clone())
            );
            assert_eq!((a.unwrap(), b.unwrap(), c.unwrap()), (7, 7, 7));
            assert_eq!(ask(cache, runs.clone()).await.unwrap(), 7);
            assert_eq!(runs.load(Ordering::SeqCst), 1);
        });
    }

    #[test]
    fn an_old_answer_is_probed_again() {
        run(async {
            let cache = ProbeCache::new("test", Duration::from_millis(10));
            assert_eq!(cache.get(|| async { Ok(1) }).await.unwrap(), 1);
            tokio::time::sleep(Duration::from_millis(30)).await;
            assert_eq!(cache.get(|| async { Ok(2) }).await.unwrap(), 2);
        });
    }

    #[test]
    fn a_failure_is_not_kept() {
        run(async {
            let cache = ProbeCache::new("test", Duration::from_secs(60));
            assert!(cache.get(|| async { Err::<u32, _>("no".into()) }).await.is_err());
            assert_eq!(cache.get(|| async { Ok(3) }).await.unwrap(), 3);
        });
    }

    #[test]
    fn a_changing_command_drops_the_answer_and_one_that_read_meanwhile_cannot_keep_it() {
        run(async {
            let cache = ProbeCache::new("test", Duration::from_secs(60));
            assert_eq!(cache.get(|| async { Ok(1) }).await.unwrap(), 1);
            {
                let _changing = cache.invalidating();
                // a read while the command runs sees the new probe ...
                assert_eq!(cache.get(|| async { Ok(2) }).await.unwrap(), 2);
            }
            // ... but the command ended: the answer read during it is not kept
            assert_eq!(cache.get(|| async { Ok(3) }).await.unwrap(), 3);
            assert_eq!(cache.get(|| async { Ok(4) }).await.unwrap(), 3);
        });
    }
}
