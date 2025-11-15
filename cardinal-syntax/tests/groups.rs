mod common;
use common::*;

#[test]
fn angle_bracket_group_with_or() {
    let expr = parse_ok("<D: | E:> *.mp3");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 2);

    let g = &parts[0];
    let or_parts = as_or(g);
    assert_eq!(or_parts.len(), 2);
    filter_is_custom(&or_parts[0], "D");
    filter_arg_none(&or_parts[0]);
    filter_is_custom(&or_parts[1], "E");
    filter_arg_none(&or_parts[1]);

    word_is(&parts[1], "*.mp3");
}

#[test]
fn parentheses_group_with_and_inside() {
    let expr = parse_ok("(foo bar) baz");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 2);
    let inner = &parts[0];
    let gparts = as_and(inner);
    word_is(&gparts[0], "foo");
    word_is(&gparts[1], "bar");
    word_is(&parts[1], "baz");
}

#[test]
fn nested_groups_or_in_angle_and_and_in_parens() {
    let expr = parse_ok("(foo <bar|baz>) qux");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 2);

    let left = &parts[0];
    let lp = as_and(left);
    assert_eq!(lp.len(), 2);
    word_is(&lp[0], "foo");

    let region = &lp[1];
    let region_parts = as_or(region);
    word_is(&region_parts[0], "bar");
    word_is(&region_parts[1], "baz");

    word_is(&parts[1], "qux");
}

#[test]
fn reports_unmatched_closing() {
    let err = parse_err(")foo");
    assert!(err.message.contains("unexpected closing"));
}

#[test]
fn reports_unmatched_opening() {
    let err = parse_err("<foo bar");
    assert!(err.message.contains("expected '>'"));
}
