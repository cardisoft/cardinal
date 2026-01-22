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
fn filter_with_escaped_quote_in_argument() {
    let expr = parse_raw("content:\"foo\\\"bar\"");
    filter_is_kind(&expr, &FilterKind::Content);
    filter_arg_raw(&expr, "\"foo\\\"bar\"");
}

#[test]
fn filter_with_quoted_argument_containing_spaces() {
    let expr = parse_raw("parent:\"C:\\Program Files\"");
    filter_is_kind(&expr, &FilterKind::Parent);
    filter_arg_raw(&expr, "\"C:\\Program Files\"");
}

#[test]
fn filter_with_empty_quoted_argument() {
    // Empty quotes result in None argument (per user-confirmed semantics)
    let parsed = parse_raw(r#"content:"""#);
    if let Expr::Term(Term::Filter(filter)) = parsed {
        assert!(matches!(filter.kind, FilterKind::Content));
        assert!(filter.argument.is_none());
    } else {
        panic!("Expected filter term");
    }
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
fn filter_list_with_escaped_quotes() {
    let expr = parse_raw("ext:\"jp\\\"g\";\"pn\\\"g\"");
    filter_is_kind(&expr, &FilterKind::Ext);
    filter_arg_is_list(&expr, &["\"jp\\\"g\"", "\"pn\\\"g\""]);
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

#[test]
fn filter_with_multiple_escaped_quotes() {
    let expr = parse_raw("content:\"a\\\"b\\\"c\"");
    filter_is_kind(&expr, &FilterKind::Content);
    filter_arg_raw(&expr, "\"a\\\"b\\\"c\"");
}

#[test]
fn filter_with_escaped_quote_at_start() {
    // Escaped quote at start must be inside a quoted phrase
    let expr = parse_raw("content:\"\\\"value\"");
    filter_is_kind(&expr, &FilterKind::Content);
    filter_arg_raw(&expr, "\"\\\"value\"");
}

#[test]
fn filter_with_escaped_quote_at_end() {
    // Escaped quote at end must be inside a quoted phrase
    let expr = parse_raw("content:\"value\\\"\"");
    filter_is_kind(&expr, &FilterKind::Content);
    filter_arg_raw(&expr, "\"value\\\"\"");
}

#[test]
fn filter_with_backslash_and_escaped_quote() {
    let expr = parse_raw("parent:\"C:\\\\Path\\\"Name\\\"\\\\file\"");
    filter_is_kind(&expr, &FilterKind::Parent);
    filter_arg_raw(&expr, "\"C:\\\\Path\\\"Name\\\"\\\\file\"");
}

#[test]
fn filter_list_with_complex_escapes() {
    let expr = parse_raw("ext:\"a\\\"b\";\"c\\\\d\";\"e\\\"f\\\"g\"");
    filter_is_kind(&expr, &FilterKind::Ext);
    filter_arg_is_list(&expr, &["\"a\\\"b\"", "\"c\\\\d\"", "\"e\\\"f\\\"g\""]);
}

#[test]
fn filter_comparison_with_escaped_quotes() {
    let expr = parse_raw("parent:>\"path\\\"with\\\"quotes\"");
    filter_is_kind(&expr, &FilterKind::Parent);
    filter_arg_is_comparison(&expr, ComparisonOp::Gt, "\"path\\\"with\\\"quotes\"");
}

#[test]
fn filter_range_with_escaped_quotes() {
    let expr = parse_raw("parent:\"start\\\"1\"..\"end\\\"2\"");
    filter_is_kind(&expr, &FilterKind::Parent);
    filter_arg_is_range_dots(&expr, Some("\"start\\\"1\""), Some("\"end\\\"2\""));
}

#[test]
fn filter_with_escaped_quote_in_unicode() {
    let expr = parse_raw("content:\"你好\\\"世界\"");
    filter_is_kind(&expr, &FilterKind::Content);
    filter_arg_raw(&expr, "\"你好\\\"世界\"");
}

#[test]
fn filter_with_only_escaped_quote() {
    // Single escaped quote must be in a quoted phrase
    let expr = parse_raw("content:\"\\\"\"");
    filter_is_kind(&expr, &FilterKind::Content);
    filter_arg_raw(&expr, "\"\\\"\"");
}

#[test]
fn filter_with_consecutive_escaped_quotes() {
    // Multiple escaped quotes in a quoted phrase
    let expr = parse_raw("content:\"\\\"\\\"\\\"\"");
    filter_is_kind(&expr, &FilterKind::Content);
    filter_arg_raw(&expr, "\"\\\"\\\"\\\"\"");
}

#[test]
fn filter_with_mixed_quoted_and_escaped() {
    // Mixed quoted segments with escaped quotes
    let expr = parse_raw("content:\"prefix\\\"\"\"middle\"\"suffix\"");
    filter_is_kind(&expr, &FilterKind::Content);
    filter_arg_raw(&expr, "\"prefix\\\"\"\"middle\"\"suffix\"");
}

#[test]
fn filter_range_with_open_start_and_escaped_end() {
    let expr = parse_raw("parent:..\"end\\\"value\"");
    filter_is_kind(&expr, &FilterKind::Parent);
    filter_arg_is_range_dots(&expr, None, Some("\"end\\\"value\""));
}

#[test]
fn filter_range_with_escaped_start_and_open_end() {
    let expr = parse_raw("parent:\"start\\\"value\"..");
    filter_is_kind(&expr, &FilterKind::Parent);
    filter_arg_is_range_dots(&expr, Some("\"start\\\"value\""), None);
}

#[test]
fn multiple_filters_with_escaped_quotes() {
    let expr = parse_ok("content:\"a\\\"b\" parent:\"c\\\"d\" tag:\"e\\\"f\"");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 3);
    // Optimizer reorders: parent (priority 0), then content, then tag (priority 3)
    filter_is_kind(&parts[0], &FilterKind::Parent);
    filter_is_kind(&parts[1], &FilterKind::Content);
    filter_is_kind(&parts[2], &FilterKind::Tag);
}

#[test]
fn filter_with_special_chars_and_escapes() {
    let expr = parse_raw("content:\"<>|&\\\"!@#$\"");
    filter_is_kind(&expr, &FilterKind::Content);
    filter_arg_raw(&expr, "\"<>|&\\\"!@#$\"");
}

#[test]
fn filter_with_quoted_empty_after_escape() {
    // Escaped quote followed by empty quotes
    let expr = parse_raw("content:\"\\\"\"\"suffix\"");
    filter_is_kind(&expr, &FilterKind::Content);
    filter_arg_raw(&expr, "\"\\\"\"\"suffix\"");
}

#[test]
fn tag_filter_with_escaped_quote_in_list() {
    let expr = parse_raw("tag:\"Red\\\"Orange\";Blue");
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_is_list(&expr, &["\"Red\\\"Orange\"", "Blue"]);
}

#[test]
fn tag_filter_with_escaped_semicolon_separator() {
    // Semicolon outside quotes acts as separator, escaped quote inside quotes
    let expr = parse_raw("tag:\"Red\";\"Blue\\\"Green\"");
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_is_list(&expr, &["\"Red\"", "\"Blue\\\"Green\""]);
}

#[test]
fn tag_filter_with_semicolon_inside_quotes() {
    // Intuitive expectation: semicolon inside quotes should be literal
    let expr = parse_raw("tag:\"Red;\";\"Blue\\\"Green\"");
    filter_is_kind(&expr, &FilterKind::Tag);

    // Expected: semicolon inside "Red;" is part of the value
    // Only semicolon outside quotes should act as separator
    filter_arg_is_list(&expr, &["\"Red;\"", "\"Blue\\\"Green\""]);
}

#[test]
fn tag_filter_semicolon_inside_quotes_with_text() {
    // Intuitive expectation: semicolon inside quotes should be literal
    let expr = parse_raw("tag:\"Red;Orange\";\"Blue\\\"Green\"");
    filter_is_kind(&expr, &FilterKind::Tag);

    // Expected: "Red;Orange" is one value with semicolon as literal character
    filter_arg_is_list(&expr, &["\"Red;Orange\"", "\"Blue\\\"Green\""]);
}

#[test]
fn tag_filter_semicolon_inside_quotes_with_multiple_items() {
    let expr = parse_raw("tag:\"Red;Orange;Yellow\";\"Blue;Green\";Purple");
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_is_list(
        &expr,
        &["\"Red;Orange;Yellow\"", "\"Blue;Green\"", "Purple"],
    );
}

#[test]
fn tag_filter_semicolon_inside_quotes_with_escaped_quote() {
    let expr = parse_raw(r#"tag:"Red;\"Orange\"";Blue"#);
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_is_list(&expr, &["\"Red;\\\"Orange\\\"\"", "Blue"]);
}

#[test]
fn tag_filter_semicolon_inside_quotes_with_escaped_semicolon() {
    let expr = parse_raw(r#"tag:"Red\;Orange";Blue"#);
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_is_list(&expr, &["\"Red\\;Orange\"", "Blue"]);
}

#[test]
fn ext_list_semicolon_inside_quotes_is_literal() {
    let expr = parse_raw("ext:\"tar;gz\";zip");
    filter_is_kind(&expr, &FilterKind::Ext);
    filter_arg_is_list(&expr, &["\"tar;gz\"", "zip"]);
}

#[test]
fn tag_filter_mixed_quoted_and_unquoted_with_semicolons() {
    let expr = parse_raw("tag:\"Red;Orange\";Green;\"Blue;Indigo\"");
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_is_list(&expr, &["\"Red;Orange\"", "Green", "\"Blue;Indigo\""]);
}

// ========== Corner Cases: Semicolons + Escaping + Empty Items ==========

#[test]
fn tag_filter_mixed_quotes_and_bare_three_items() {
    // tag:"a;b";c;"d;e" → ["a;b", "c", "d;e"] (3 items)
    let expr = parse_raw(r#"tag:"a;b";c;"d;e""#);
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_is_list(&expr, &[r#""a;b""#, "c", r#""d;e""#]);
}

#[test]
fn tag_filter_escaped_semicolon_inside_quotes() {
    // tag:"a\;b";c → semicolon inside quotes is literal (backslash preserved)
    let expr = parse_raw(r#"tag:"a\;b";c"#);
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_is_list(&expr, &[r#""a\;b""#, "c"]);
}

#[test]
fn tag_filter_escaped_quote_with_semicolon_inside_quotes() {
    // tag:"a;\"b\"";c → escaped quote doesn't break quote state, semicolon stays literal
    let expr = parse_raw(r#"tag:"a;\"b\"";c"#);
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_is_list(&expr, &[r#""a;\"b\"""#, "c"]);
}

#[test]
fn tag_filter_escaped_quote_before_semicolon_inside_quotes() {
    // tag:"a\";b";c → escaped quote doesn't break quote state, semicolon stays literal
    let expr = parse_raw(r#"tag:"a\";b";c"#);
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_is_list(&expr, &[r#""a\";b""#, "c"]);
}

#[test]
fn tag_filter_backslash_before_semicolon_inside_quotes() {
    // tag:"a\\;b";c → backslash before semicolon, semicolon still literal inside quotes
    let expr = parse_raw(r#"tag:"a\\;b";c"#);
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_is_list(&expr, &[r#""a\\;b""#, "c"]);
}

#[test]
fn tag_filter_backslash_escaped_quote_inside_quotes() {
    // tag:"a\\\"b";c → backslash-escaped quote (quote becomes literal)
    let expr = parse_raw(r#"tag:"a\\\"b";c"#);
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_is_list(&expr, &[r#""a\\\"b""#, "c"]);
}

#[test]
fn tag_filter_consecutive_semicolons_with_spaces() {
    // tag:"a";"b";; → empty items ignored (trailing semicolons)
    // Note: spaces outside quotes break the argument parsing
    let expr = parse_raw(r#"tag:"a";"b";;"#);
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_is_list(&expr, &[r#""a""#, r#""b""#]);
}

#[test]
fn tag_filter_empty_quotes_with_value() {
    // tag:"";a → empty quoted string is preserved as empty item
    let expr = parse_raw(r#"tag:"";a"#);
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_is_list(&expr, &[r#""""#, "a"]);
}

#[test]
fn tag_filter_only_empty_quotes() {
    // tag:"" → empty quoted string, but Filter::argument should be None
    let parsed = parse_raw(r#"tag:"""#);
    if let Expr::Term(Term::Filter(filter)) = parsed {
        assert!(matches!(filter.kind, FilterKind::Tag));
        // Empty quotes should result in None argument (per user confirmation)
        assert!(filter.argument.is_none());
    } else {
        panic!("Expected filter term");
    }
}

#[test]
fn tag_filter_escaped_semicolon_outside_quotes() {
    // tag:a\;b;c → \; outside quotes is literal (does not split)
    let expr = parse_raw(r#"tag:a\;b;c"#);
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_is_list(&expr, &[r#"a\;b"#, "c"]);
}

#[test]
fn ext_filter_semicolon_inside_quotes() {
    // ext:"tar;gz";zip → non-tag filter should have same behavior
    let expr = parse_raw(r#"ext:"tar;gz";zip"#);
    filter_is_kind(&expr, &FilterKind::Ext);
    filter_arg_is_list(&expr, &[r#""tar;gz""#, "zip"]);
}

#[test]
fn custom_filter_semicolon_inside_quotes() {
    // proj:"a;b";c → custom filter should have same behavior
    if let Expr::Term(Term::Filter(filter)) = parse_raw(r#"proj:"a;b";c"#) {
        assert!(matches!(filter.kind, FilterKind::Custom(ref name) if name == "proj"));
        let arg = filter.argument.unwrap();
        match arg.kind {
            ArgumentKind::List(values) => {
                assert_eq!(values, vec![r#""a;b""#, "c"]);
            }
            _ => panic!("Expected list argument"),
        }
    } else {
        panic!("Expected filter term");
    }
}

#[test]
fn tag_filter_complex_mixed_escape_scenarios() {
    // tag:"a\\;b\"c";d\;e;"f;g" → complex mix
    let expr = parse_raw(r#"tag:"a\\;b\"c";d\;e;"f;g""#);
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_is_list(&expr, &[r#""a\\;b\"c""#, r#"d\;e"#, r#""f;g""#]);
}
