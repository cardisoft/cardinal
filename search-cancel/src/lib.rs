use std::sync::atomic::{AtomicU64, Ordering};

/// How often long-running loops should check whether execution was cancelled.
pub const CANCEL_CHECK_INTERVAL: usize = 0x10000;

/// A global atomic identifies the active search version of Cardinal.
pub static ACTIVE_SEARCH_VERSION: AtomicU64 = AtomicU64::new(0);

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

    pub fn is_cancelled(&self) -> Option<()> {
        if self.version != self.active_version.load(Ordering::Relaxed) {
            None
        } else {
            Some(())
        }
    }

    pub fn is_cancelled_sparse(&self, counter: usize) -> Option<()> {
        if counter % CANCEL_CHECK_INTERVAL == 0 {
            self.is_cancelled()
        } else {
            Some(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_token_is_never_cancelled() {
        let token = CancellationToken::noop();
        assert!(
            token.is_cancelled().is_some(),
            "noop token should never be cancelled"
        );
    }

    #[test]
    fn cancelled_after_version_change() {
        let token_v1 = CancellationToken::new(1);
        assert!(
            token_v1.is_cancelled().is_some(),
            "initial version should be active"
        );

        // Bump the active version, cancelling the older token.
        let _token_v2 = CancellationToken::new(2);
        assert!(token_v1.is_cancelled().is_none());
    }
}
