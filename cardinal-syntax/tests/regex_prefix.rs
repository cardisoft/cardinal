mod common;
use common::*;

#[test]
fn regex_prefix_switches_mode() {
    let expr = parse_ok("regex:^Report");
    regex_is(&expr, "^Report");
}

#[test]
fn regex_prefix_trims_whitespace_and_is_case_insensitive() {
    let expr = parse_ok("  ReGeX:  [0-9]{4}   ");
    regex_is(&expr, "[0-9]{4}");
}

#[test]
fn regex_prefix_requires_pattern() {
    let err = parse_err("regex:");
    assert!(err.message.contains("requires a pattern"));
}
