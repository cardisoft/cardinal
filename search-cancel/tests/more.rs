use search_cancel::{
    ACTIVE_SCAN_VERSION, ACTIVE_SEARCH_VERSION, CANCEL_CHECK_INTERVAL, CancellationToken,
};
use std::sync::{LazyLock, Mutex, MutexGuard, atomic::Ordering};

static TEST_GUARD: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn lock_and_reset_versions() -> MutexGuard<'static, ()> {
    let guard = TEST_GUARD
        .lock()
        .expect("token test lock should not be poisoned");
    ACTIVE_SEARCH_VERSION.store(0, Ordering::SeqCst);
    ACTIVE_SCAN_VERSION.store(0, Ordering::SeqCst);
    guard
}

#[test]
fn multiple_tokens_cancelled_independently() {
    let _guard = lock_and_reset_versions();

    let t1 = CancellationToken::new(1);
    assert!(t1.is_cancelled().is_some());
    let t2 = CancellationToken::new(2);
    assert!(t1.is_cancelled().is_none());
    assert!(t2.is_cancelled().is_some());
    let t3 = CancellationToken::new(3);
    assert!(t2.is_cancelled().is_none());
    assert!(t3.is_cancelled().is_some());
}

#[test]
fn multiple_scan_tokens_cancelled_independently() {
    let _guard = lock_and_reset_versions();

    let s1 = CancellationToken::new_scan();
    assert!(s1.is_cancelled().is_some());

    let s2 = CancellationToken::new_scan();
    assert!(s1.is_cancelled().is_none());
    assert!(s2.is_cancelled().is_some());

    let s3 = CancellationToken::new_scan();
    assert!(s2.is_cancelled().is_none());
    assert!(s3.is_cancelled().is_some());
}

#[test]
fn search_version_changes_do_not_cancel_scan_token() {
    let _guard = lock_and_reset_versions();

    let scan = CancellationToken::new_scan();
    let _search_v1 = CancellationToken::new(1);
    let _search_v2 = CancellationToken::new(2);
    assert!(
        scan.is_cancelled().is_some(),
        "scan token should not be affected by search version updates"
    );
}

#[test]
fn scan_sparse_checks_only_cancel_at_interval_boundaries() {
    let _guard = lock_and_reset_versions();

    let stale = CancellationToken::new_scan();
    let _latest = CancellationToken::new_scan();

    assert!(
        stale
            .is_cancelled_sparse(CANCEL_CHECK_INTERVAL.saturating_sub(1))
            .is_some(),
        "non-interval checks should skip cancellation check"
    );
    assert!(
        stale.is_cancelled_sparse(CANCEL_CHECK_INTERVAL).is_none(),
        "interval checks should observe stale scan token cancellation"
    );
}

#[test]
fn active_scan_token_sparse_check_always_passes() {
    let _guard = lock_and_reset_versions();

    let active = CancellationToken::new_scan();

    // Non-interval counter: skips check, returns Some.
    assert!(
        active
            .is_cancelled_sparse(CANCEL_CHECK_INTERVAL.saturating_sub(1))
            .is_some(),
        "non-interval sparse check on active token should pass"
    );
    // Interval boundary: actually checks — still Some because token is latest.
    assert!(
        active.is_cancelled_sparse(CANCEL_CHECK_INTERVAL).is_some(),
        "interval sparse check on active token should still pass"
    );
}

#[test]
fn noop_unaffected_by_scan_versions() {
    let _guard = lock_and_reset_versions();

    let noop = CancellationToken::noop();
    let _s1 = CancellationToken::new_scan();
    let _s2 = CancellationToken::new_scan();
    assert!(
        noop.is_cancelled().is_some(),
        "noop token must remain active regardless of scan version bumps"
    );
}

#[test]
fn active_scan_token_survives_many_search_bumps() {
    let _guard = lock_and_reset_versions();

    let scan = CancellationToken::new_scan();
    for v in 1..=10_u64 {
        let _search = CancellationToken::new(v);
    }
    assert!(
        scan.is_cancelled().is_some(),
        "scan token must remain active after many search version changes"
    );
}

#[test]
fn new_scan_cancels_all_previous_scan_tokens() {
    let _guard = lock_and_reset_versions();

    let tokens: Vec<_> = (0..5).map(|_| CancellationToken::new_scan()).collect();
    let latest = tokens.last().unwrap();

    // Only the last token should be active.
    for t in &tokens[..tokens.len() - 1] {
        assert!(
            t.is_cancelled().is_none(),
            "every prior scan token should be cancelled when a newer one is created"
        );
    }
    assert!(
        latest.is_cancelled().is_some(),
        "the most recently created scan token must be active"
    );
}
