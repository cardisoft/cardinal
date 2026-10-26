//! Snapshot scheduling state.
//!
//! All timestamps are `Instant`s. On macOS `Instant` is `CLOCK_UPTIME_RAW`, which does not
//! advance while the machine is asleep, so waking never looks like a long idle period and
//! never brings a checkpoint forward. (The previous wall-clock implementation made the very
//! first flush tick after every wake rewrite the whole cache file.)
//!
//! Two independent things are tracked here:
//! - search activity, which only defines whether the app is *quiet* (an opportunity to write
//!   without delaying a user's searches);
//! - checkpoint scheduling (last write attempt, and how long a due checkpoint has been
//!   waiting for a quiet moment), which bounds how much is written per day.
//!
//! They used to be the same timestamp, reset after every flush, which produced an
//! unconditional full rewrite every five minutes.

use parking_lot::Mutex;
use std::time::{Duration, Instant};

/// The app counts as idle once no search has run for this long; an idle app is always a
/// good moment to write a snapshot.
pub(crate) const IDLE_FLUSH_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Once a checkpoint is already due and waiting, a gap this long between searches is also
/// accepted as a quiet moment, so most checkpoints land in a natural pause instead of being
/// forced at the deferral bound.
pub(crate) const QUIET_GAP: Duration = Duration::from_secs(30);

/// Minimum spacing between two automatic snapshot attempts, whatever the trigger (hide or
/// idle). Each snapshot rewrites the entire cache file (~100 MB for a full-disk index);
/// macOS files a disk-write resource exception for a process that averages roughly 25 KB/s
/// over a day, which a 30-minute spacing can still exceed under continuous change
/// (48 x 100 MB = 4.8 GB/day). Six hours bounds the automatic volume at ~400 MB/day
/// (4.6 KB/s). The cost of a stale snapshot is only extra FSEvents replay after a crash; an
/// orderly exit always writes a dirty index regardless of this interval. If a shorter replay
/// window is preferred, two hours still stays under the threshold (1.2 GB/day, 13.9 KB/s).
pub(crate) const CHECKPOINT_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// Once a checkpoint is due (dirty and the interval has elapsed) it waits this long for a
/// quiet moment (window hidden, idle, or a search gap), then is written anyway so that
/// continuous searching can never starve persistence.
pub(crate) const CHECKPOINT_DEFERRAL: Duration = Duration::from_secs(5 * 60);

static LAST_SEARCH_AT: Mutex<Option<Instant>> = Mutex::new(None);

/// Last snapshot write attempt (success or failure, so a persistently failing disk does not
/// turn into a retry storm). `None` means this process has never attempted one; a freshly
/// walked index therefore becomes due immediately, which is what we want for a baseline
/// that would otherwise take a full-disk walk to recreate.
static LAST_FLUSH_ATTEMPT_AT: Mutex<Option<Instant>> = Mutex::new(None);

/// When the currently due checkpoint started waiting for a quiet moment. Deliberately not
/// reset by searches: new searches must not push the deadline out.
static CHECKPOINT_DUE_SINCE: Mutex<Option<Instant>> = Mutex::new(None);

pub fn note_search_activity() {
    *LAST_SEARCH_AT.lock() = Some(Instant::now());
}

/// True when no search has run for at least [`IDLE_FLUSH_INTERVAL`]. Before the first search
/// of the session this is `false`; the deferral bound in the policy covers that case.
pub fn search_idles() -> bool {
    LAST_SEARCH_AT
        .lock()
        .is_some_and(|last| last.elapsed() >= IDLE_FLUSH_INTERVAL)
}

/// True when no search has run for at least [`QUIET_GAP`] (or ever).
pub fn search_quiet() -> bool {
    LAST_SEARCH_AT
        .lock()
        .is_none_or(|last| last.elapsed() >= QUIET_GAP)
}

/// A snapshot was loaded from disk at startup: it is a valid baseline, so start the
/// checkpoint interval now instead of treating the index as never persisted.
pub fn note_loaded_baseline() {
    *LAST_FLUSH_ATTEMPT_AT.lock() = Some(Instant::now());
}

pub fn note_flush_attempt() {
    *LAST_FLUSH_ATTEMPT_AT.lock() = Some(Instant::now());
    *CHECKPOINT_DUE_SINCE.lock() = None;
}

pub fn since_last_flush_attempt() -> Option<Duration> {
    LAST_FLUSH_ATTEMPT_AT.lock().map(|last| last.elapsed())
}

/// Record that a checkpoint is due but waiting for a quiet moment (idempotent).
pub fn note_checkpoint_due() {
    let mut due_since = CHECKPOINT_DUE_SINCE.lock();
    if due_since.is_none() {
        *due_since = Some(Instant::now());
    }
}

pub fn clear_checkpoint_due() {
    *CHECKPOINT_DUE_SINCE.lock() = None;
}

/// How long the current due checkpoint has been waiting, if one is waiting.
pub fn checkpoint_waiting_for() -> Option<Duration> {
    CHECKPOINT_DUE_SINCE.lock().map(|since| since.elapsed())
}
