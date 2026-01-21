mod common;
use cardinal_syntax::*;
use common::*;

#[test]
fn recognized_filter_kinds_without_arguments() {
    let cases = [
        ("file:", FilterKind::File),
        ("folder:", FilterKind::Folder),
        ("audio:", FilterKind::Audio),
        ("video:", FilterKind::Video),
        ("doc:", FilterKind::Doc),
        ("exe:", FilterKind::Exe),
        ("attribdupe:", FilterKind::AttributeDuplicate),
        ("dmdupe:", FilterKind::DateModifiedDuplicate),
        ("dupe:", FilterKind::Duplicate),
        ("namepartdupe:", FilterKind::NamePartDuplicate),
        ("sizedupe:", FilterKind::SizeDuplicate),
        ("nowholefilename:", FilterKind::NoWholeFilename),
    ];

    for (q, kind) in cases {
        let expr = parse_ok(q);
        filter_is_kind(&expr, &kind);
        filter_arg_none(&expr);
    }
}

#[test]
fn custom_filter_name_is_preserved() {
    let expr = parse_ok("proj:");
    filter_is_custom(&expr, "proj");
}

#[test]
fn ext_list_is_semicolon_split() {
    let expr = parse_ok("ext:jpg;png;jpeg");
    filter_is_kind(&expr, &FilterKind::Ext);
    filter_arg_is_list(&expr, &["jpg", "png", "jpeg"]);
}

#[test]
fn ext_trailing_semicolon_is_singleton_list() {
    let expr = parse_ok("ext:jpg;");
    filter_is_kind(&expr, &FilterKind::Ext);
    filter_arg_is_list(&expr, &["jpg"]);
}

#[test]
fn content_filter_has_bare_argument() {
    let expr = parse_ok("content:error");
    filter_is_kind(&expr, &FilterKind::Content);
    filter_arg_raw(&expr, "error");
}

#[test]
fn tag_filter_has_bare_argument() {
    let expr = parse_ok("tag:Project");
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_raw(&expr, "Project");
}

#[test]
fn tag_filter_shorthand_aliases() {
    let expr = parse_ok("t:Project");
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_raw(&expr, "Project");
}

#[test]
fn infolder_filter_shorthand_alias() {
    let expr = parse_ok("in:/Users/demo");
    filter_is_kind(&expr, &FilterKind::InFolder);
    filter_arg_raw(&expr, "/Users/demo");
}

#[test]
fn tag_filter_trailing_semicolon_is_singleton_list() {
    let expr = parse_ok("tag:Orange;");
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_is_list(&expr, &["Orange"]);
}

#[test]
fn phrase_argument_is_detected() {
    let expr = parse_ok("parent:\"/Users/demo\"");
    let (_, arg) = filter_kind(&expr);
    match arg.as_ref().unwrap().kind {
        ArgumentKind::Phrase => {}
        ref other => panic!("expected Phrase, got: {other:?}"),
    }
}

#[test]
fn filter_can_appear_anywhere_in_and_chain() {
    let expr = parse_ok("video: size:>1gb report");
    let parts = as_and(&expr);
    word_is(&parts[0], "report");
    filter_is_kind(&parts[1], &FilterKind::Video);
    filter_arg_none(&parts[1]);
    filter_is_kind(&parts[2], &FilterKind::Size);
    filter_arg_is_comparison(&parts[2], ComparisonOp::Gt, "1gb");
}

#[test]
fn filters_are_moved_to_the_end_of_and_chain() {
    let expr = parse_ok("folder:projects dm:today report dc:thisweek");
    let parts = as_and(&expr);
    word_is(&parts[0], "report");
    filter_is_kind(&parts[1], &FilterKind::Folder);
    filter_arg_raw(&parts[1], "projects");
    filter_is_kind(&parts[2], &FilterKind::DateModified);
    filter_is_kind(&parts[3], &FilterKind::DateCreated);
}

#[test]
fn filters_preserve_relative_order() {
    let expr = parse_ok("foo dc:thisweek bar dm:pastmonth ext:rs");
    let parts = as_and(&expr);
    word_is(&parts[0], "foo");
    word_is(&parts[1], "bar");
    filter_is_kind(&parts[2], &FilterKind::DateCreated);
    filter_is_kind(&parts[3], &FilterKind::DateModified);
    filter_is_kind(&parts[4], &FilterKind::Ext);
}

#[test]
fn filter_with_quoted_argument() {
    let expr = parse_raw("folder:\"My Documents\"");
    filter_is_kind(&expr, &FilterKind::Folder);
    filter_arg_raw(&expr, "\"My Documents\"");
}

#[test]
fn filter_with_quoted_argument_containing_spaces() {
    let expr = parse_raw("parent:\"C:\\Program Files\"");
    filter_is_kind(&expr, &FilterKind::Parent);
    filter_arg_raw(&expr, "\"C:\\Program Files\"");
}

#[test]
fn filter_with_empty_quoted_argument() {
    let expr = parse_raw("content:\"\"");
    filter_is_kind(&expr, &FilterKind::Content);
    filter_arg_raw(&expr, "\"\"");
}

#[test]
fn filter_with_multiple_quoted_segments() {
    let expr = parse_raw("folder:\"part1\"\"part2\"");
    filter_is_kind(&expr, &FilterKind::Folder);
    filter_arg_raw(&expr, "\"part1\"\"part2\"");
}

#[test]
fn filter_with_quoted_special_characters() {
    let expr = parse_raw("content:\"!@#$%^&*()\"");
    filter_is_kind(&expr, &FilterKind::Content);
    filter_arg_raw(&expr, "\"!@#$%^&*()\"");
}

#[test]
fn filter_with_mixed_quoted_and_unquoted() {
    let expr = parse_ok("folder:prefix\"middle\"suffix");
    filter_is_kind(&expr, &FilterKind::Folder);
    filter_arg_raw(&expr, "prefix\"middle\"suffix");
}

#[test]
fn filter_list_with_quoted_values() {
    let expr = parse_raw("ext:\"jpg\";\"png\"");
    filter_is_kind(&expr, &FilterKind::Ext);
    filter_arg_is_list(&expr, &["\"jpg\"", "\"png\""]);
}

#[test]
fn filter_comparison_with_quoted_value() {
    let expr = parse_ok("parent:>\"path\"");
    filter_is_kind(&expr, &FilterKind::Parent);
    filter_arg_is_comparison(&expr, ComparisonOp::Gt, "\"path\"");
}

#[test]
fn filter_range_with_quoted_values() {
    let expr = parse_raw("parent:\"start\"..\"end\"");
    filter_is_kind(&expr, &FilterKind::Parent);
    filter_arg_is_range_dots(&expr, Some("\"start\""), Some("\"end\""));
}

#[test]
fn filter_with_unicode_in_quotes() {
    let expr = parse_raw("content:\"你好世界\"");
    filter_is_kind(&expr, &FilterKind::Content);
    filter_arg_raw(&expr, "\"你好世界\"");
}
