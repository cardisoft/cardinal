mod common;
use cardinal_syntax::*;
use common::*;

#[test]
fn list_with_single_item_or_trailing_semicolon_is_bare() {
    let expr = parse_ok("ext:jpg;");
    filter_is_kind(&expr, &FilterKind::Ext);
    // classify_argument should treat this as Bare because only one non-empty item
    let (_, arg) = filter_kind(&expr);
    assert!(matches!(arg.as_ref().unwrap().kind, ArgumentKind::Bare));

    let expr = parse_ok("ext:jpg");
    let (_, arg) = filter_kind(&expr);
    assert!(matches!(arg.as_ref().unwrap().kind, ArgumentKind::Bare));
}

#[test]
fn dotted_range_requires_digits_otherwise_bare() {
    let expr = parse_ok("size:a..b");
    let (_, arg) = filter_kind(&expr);
    assert!(matches!(arg.as_ref().unwrap().kind, ArgumentKind::Bare));

    let expr = parse_ok("size:..10");
    filter_arg_is_range_dots(&expr, None, Some("10"));

    let expr = parse_ok("size:1..");
    filter_arg_is_range_dots(&expr, Some("1"), None);
}

// Spaced ranges are not a single token in Everything syntax; not testing here.
