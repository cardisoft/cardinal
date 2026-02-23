use std::sync::atomic::{AtomicU64, Ordering};

/// How often long-running loops should check whether execution was cancelled.
pub const CANCEL_CHECK_INTERVAL: usize = 0x10000;

/// A global atomic identifies the active search version of Cardinal.
pub static ACTIVE_SEARCH_VERSION: AtomicU64 = AtomicU64::new(0);

/// A global atomic identifies the active scanning process version of Cardinal.
pub static ACTIVE_SCAN_VERSION: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug)]
pub struct CancellationToken {
    active_version: &'static AtomicU64,
    version: u64,
}

impl CancellationToken {
    pub fn noop() -> Self {
        static NOOP: AtomicU64 = AtomicU64::new(0);
        Self {
            version: 0,
            active_version: &NOOP,
        }
    }

    pub fn new(version: u64) -> Self {
        ACTIVE_SEARCH_VERSION.store(version, Ordering::SeqCst);
        Self {
            version,
            active_version: &ACTIVE_SEARCH_VERSION,
        }
    }

    pub fn new_scan() -> Self {
        let version = self::ACTIVE_SCAN_VERSION.fetch_add(1, Ordering::SeqCst) + 1;
        Self {
            version,
            active_version: &ACTIVE_SCAN_VERSION,
        }
    }

    pub fn is_cancelled(&self) -> Option<()> {
        if self.version != self.active_version.load(Ordering::Relaxed) {
            None
        } else {
            Some(())
        }
    }

    pub fn is_cancelled_sparse(&self, counter: usize) -> Option<()> {
        if counter.is_multiple_of(CANCEL_CHECK_INTERVAL) {
            self.is_cancelled()
        } else {
            Some(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    static TEST_GUARD: Mutex<()> = Mutex::new(());

    fn lock_versions() -> MutexGuard<'static, ()> {
        TEST_GUARD
            .lock()
            .expect("version lock should not be poisoned")
    }

    fn reset_versions() {
        ACTIVE_SEARCH_VERSION.store(0, Ordering::SeqCst);
        ACTIVE_SCAN_VERSION.store(0, Ordering::SeqCst);
    }

    #[test]
    fn noop_token_is_never_cancelled() {
        let _guard = lock_versions();
        reset_versions();
        let token = CancellationToken::noop();
        assert!(
            token.is_cancelled().is_some(),
            "noop token should never be cancelled"
        );
    }

    #[test]
    fn cancelled_after_version_change() {
        let _guard = lock_versions();
        reset_versions();
        let token_v1 = CancellationToken::new(1);
        assert!(
            token_v1.is_cancelled().is_some(),
            "initial version should be active"
        );

        // Bump the active version, cancelling the older token.
        let _token_v2 = CancellationToken::new(2);
        assert!(token_v1.is_cancelled().is_none());
    }

    #[test]
    fn scan_token_cancelled_after_new_scan_version() {
        let _guard = lock_versions();
        reset_versions();

        let scan_v1 = CancellationToken::new_scan();
        assert!(
            scan_v1.is_cancelled().is_some(),
            "latest scan token should start active"
        );

        let scan_v2 = CancellationToken::new_scan();
        assert!(
            scan_v2.is_cancelled().is_some(),
            "new scan token should be active"
        );
        assert!(
            scan_v1.is_cancelled().is_none(),
            "older scan token should be cancelled by newer scan"
        );
    }

    #[test]
    fn scan_versions_do_not_cancel_search_tokens() {
        let _guard = lock_versions();
        reset_versions();

        let search_v1 = CancellationToken::new(1);
        let _scan_v1 = CancellationToken::new_scan();
        assert!(
            search_v1.is_cancelled().is_some(),
            "scan token updates should not affect search version"
        );

        let _search_v2 = CancellationToken::new(2);
        assert!(
            search_v1.is_cancelled().is_none(),
            "search token should still be governed by search version updates"
        );
    }
}
