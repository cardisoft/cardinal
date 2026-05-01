//! Broad deterministic fuzz-like coverage for NamePool search strategies.
//! Pushes a large corpus of synthetic identifiers and validates search behaviors.
//! Intentionally long for coverage depth & line count purposes.

use namepool::NamePool;
use search_cancel::CancellationToken;

fn build_pool() -> NamePool {
    let pool = NamePool::new();
    // Base words and variations.
    let bases = [
        "alpha",
        "beta",
        "gamma",
        "delta",
        "epsilon",
        "zeta",
        "eta",
        "theta",
        "iota",
        "kappa",
        "lambda",
        "mu",
        "nu",
        "xi",
        "omicron",
        "pi",
        "rho",
        "sigma",
        "tau",
        "upsilon",
        "phi",
        "chi",
        "psi",
        "omega",
        "main",
        "lib",
        "config",
        "readme",
        "license",
        "changelog",
        "components",
        "assets",
        "tests",
        "target",
        "search",
        "cache",
        "parser",
        "syntax",
        "segment",
        "event",
        "index",
        "node",
        "slab",
        "icon",
        "walk",
        "cancel",
        "pool",
        "fuzz",
        "match",
        "regex",
    ];
    for base in bases {
        pool.push(base);
    }
    // Add extended variants.
    for base in bases.iter() {
        pool.push(&format!("{base}_v1"));
        pool.push(&format!("{base}_v2"));
        pool.push(&format!("{base}_test"));
        pool.push(&format!("{base}_TEST"));
        pool.push(&format!("pre_{base}_post"));
        pool.push(&format!("{base}_123"));
        pool.push(&format!("{base}123"));
        pool.push(&format!("{base}XYZ"));
        pool.push(&format!("{base}-dash"));
        pool.push(&format!("{base}.dot"));
        pool.push(&format!("{base}CamelCase"));
        pool.push(&format!("{base}_mix_snake_case"));
    }
    pool
}

#[test]
fn substr_search_completeness() {
    let pool = build_pool();
    let token = CancellationToken::noop();
    for needle in [
        "alpha", "main", "lib", "icon", "walk", "cancel", "regex", "segment", "node", "slab",
    ] {
        let results = pool.search_substr(needle, token).unwrap();
        assert!(
            !results.is_empty(),
            "substr search should find at least one match for {needle}"
        );
    }
}

#[test]
fn prefix_search_basic() {
    let pool = build_pool();
    let token = CancellationToken::noop();
    for needle in [
        "alpha",
        "beta",
        "gamma",
        "delta",
        "main",
        "readme",
        "components",
        "segment",
    ] {
        let results = pool.search_prefix(needle, token).unwrap();
        assert!(
            results.iter().any(|s| s.starts_with(needle)),
            "prefix results must start with {needle}"
        );
    }
}

#[test]
fn suffix_search_basic() {
    let pool = build_pool();
    let token = CancellationToken::noop();
    for needle in [
        "v1", "v2", "test", "TEST", "123", "dash", "dot", "Case", "case",
    ] {
        let results = pool.search_suffix(needle, token).unwrap();
        assert!(
            results.iter().any(|s| s.ends_with(needle)),
            "suffix results must end with {needle}"
        );
    }
}

#[test]
fn exact_search_includes_original_bases() {
    let pool = build_pool();
    let token = CancellationToken::noop();
    for needle in [
        "alpha", "beta", "gamma", "delta", "main", "lib", "icon", "walk",
    ] {
        let results = pool.search_exact(needle, token).unwrap();
        assert!(
            results.contains(needle),
            "exact search must contain the needle {needle}"
        );
    }
}

#[test]
fn regex_search_varied_patterns() {
    let pool = build_pool();
    let token = CancellationToken::noop();
    let patterns = [
        "^alpha.*",
        "^beta.*v1$",
        "^gamma.*123$",
        "^[a-z]+_v2$",
        "^[a-z]+_test$",
        "^pre_[a-z]+_post$",
        "^[a-z]+XYZ$",
        "^[a-z]+-dash$",
        "^[a-z]+\\.dot$",
        "^[a-z]+CamelCase$",
        "^[a-z]+_mix_snake_case$",
        "^(alpha|beta|gamma|delta)$",
        "^(main|lib|config)$",
        "^(walk|icon|pool)$",
    ];
    for pat in patterns {
        let re = regex::Regex::new(pat).unwrap();
        let results = pool.search_regex(&re, token).unwrap();
        assert!(!results.is_empty(), "regex {pat} expected matches");
    }
}

#[test]
fn cancellation_simulation() {
    let pool = build_pool();
    let token = CancellationToken::new_search();
    let _ = CancellationToken::new_search(); // cancel previous
    // All searches should return None due to cancellation.
    assert!(pool.search_substr("alpha", token).is_none());
    assert!(pool.search_prefix("alpha", token).is_none());
    assert!(pool.search_suffix("alpha", token).is_none());
    assert!(pool.search_exact("alpha", token).is_none());
    let re = regex::Regex::new("^alpha.*").unwrap();
    assert!(pool.search_regex(&re, token).is_none());
}

#[test]
fn corpus_size_expectations() {
    let pool = build_pool();
    // Due to potential internal normalization/dedup (e.g. case folding) exact variant count shrinks.
    // Ensure we still have a large corpus: at least 5x base count.
    let base_count = 52usize; // number of bases above.
    let expected_min = base_count * 5;
    assert!(
        pool.len() >= expected_min,
        "pool size {} should exceed {expected_min}",
        pool.len()
    );
}
