use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub(crate) const IDLE_FLUSH_INTERVAL: Duration = Duration::from_secs(5 * 60);
static LAST_SEARCH_AT_MS: AtomicU64 = AtomicU64::new(0);

pub fn note_search_activity() {
    LAST_SEARCH_AT_MS.store(unix_ms_now(), Ordering::Relaxed);
}

pub fn search_idles() -> bool {
    elapsed_since_last_search().is_some_and(|elapsed| elapsed >= IDLE_FLUSH_INTERVAL)
}

fn elapsed_since_last_search() -> Option<Duration> {
    let last = LAST_SEARCH_AT_MS.load(Ordering::Relaxed);
    if last == 0 {
        return None;
    }
    let now_ms = unix_ms_now();
    Some(Duration::from_millis(now_ms.saturating_sub(last)))
}

fn unix_ms_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
