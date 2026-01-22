mod common;
use cardinal_syntax::*;
use common::*;

// Tests for backslash escape sequences, focusing on quote escaping

#[test]
fn single_escaped_quote() {
    let expr = parse_ok("\\\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\\\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn double_escaped_quote() {
    let expr = parse_ok("\\\"\\\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\\\"\\\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn escaped_quote_in_phrase() {
    let expr = parse_ok("\"\\\"\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"\\\"\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn phrase_with_escaped_quote_at_start() {
    let expr = parse_ok("\"\\\"hello\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"\\\"hello\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn phrase_with_escaped_quote_at_end() {
    let expr = parse_ok("\"hello\\\"\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"hello\\\"\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn phrase_with_escaped_quote_in_middle() {
    let expr = parse_ok("\"hello\\\"world\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"hello\\\"world\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn phrase_with_multiple_escaped_quotes() {
    let expr = parse_ok("\"a\\\"b\\\"c\\\"d\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"a\\\"b\\\"c\\\"d\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn phrase_with_consecutive_escaped_quotes() {
    let expr = parse_ok("\"\\\"\\\"\\\"\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"\\\"\\\"\\\"\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn phrase_with_backslash_and_escaped_quote() {
    let expr = parse_ok("\"\\\\\\\"\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"\\\\\\\"\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn phrase_with_regular_backslash() {
    let expr = parse_ok("\"C:\\\\Users\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"C:\\\\Users\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn phrase_with_mixed_backslashes() {
    let expr = parse_ok("\"C:\\\\Path\\\"Name\\\"\\\\file.txt\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"C:\\\\Path\\\"Name\\\"\\\\file.txt\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn word_with_escaped_quote() {
    let expr = parse_ok("word\\\"part");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "word\\\"part"),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn word_starting_with_escaped_quote() {
    let expr = parse_ok("\\\"word");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\\\"word"),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn word_ending_with_escaped_quote() {
    let expr = parse_ok("word\\\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "word\\\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn and_expression_with_escaped_quotes() {
    let expr = parse_ok("foo\\\"bar \"baz\\\"qux\" xyz");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 3);
    word_is(&parts[0], "foo\\\"bar");
    word_is(&parts[1], "\"baz\\\"qux\"");
    word_is(&parts[2], "xyz");
}

#[test]
fn or_expression_with_escaped_quotes() {
    let expr = parse_ok("\"a\\\"b\" | \"c\\\"d\" | e\\\"f");
    let parts = as_or(&expr);
    assert_eq!(parts.len(), 3);
    word_is(&parts[0], "\"a\\\"b\"");
    word_is(&parts[1], "\"c\\\"d\"");
    word_is(&parts[2], "e\\\"f");
}

#[test]
fn not_expression_with_escaped_quote() {
    let expr = parse_ok("!\"test\\\"value\"");
    let inner = as_not(&expr);
    word_is(inner, "\"test\\\"value\"");
}

#[test]
fn grouped_expression_with_escaped_quotes() {
    let expr = parse_ok("(\"a\\\"b\" c\\\"d)");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 2);
    word_is(&parts[0], "\"a\\\"b\"");
    word_is(&parts[1], "c\\\"d");
}

#[test]
fn filter_with_escaped_quote_bare() {
    // Without quotes, backslash-quote appears as part of a word, but parser treats it as starting a quote
    // So we need to properly quote it
    let expr = parse_ok("content:\"test\\\"value\"");
    filter_is_kind(&expr, &FilterKind::Content);
    filter_arg_raw(&expr, "\"test\\\"value\"");
}

#[test]
fn filter_with_escaped_quote_in_phrase() {
    let expr = parse_ok("content:\"test\\\"value\"");
    filter_is_kind(&expr, &FilterKind::Content);
    filter_arg_raw(&expr, "\"test\\\"value\"");
}

#[test]
fn filter_with_multiple_escaped_quotes() {
    let expr = parse_ok("parent:\"C:\\\\Users\\\"John\\\"\\\\Documents\"");
    filter_is_kind(&expr, &FilterKind::Parent);
    filter_arg_raw(&expr, "\"C:\\\\Users\\\"John\\\"\\\\Documents\"");
}

#[test]
fn filter_list_with_escaped_quotes() {
    let expr = parse_ok("ext:\"a\\\"b\";\"c\\\"d\"");
    filter_is_kind(&expr, &FilterKind::Ext);
    filter_arg_is_list(&expr, &["\"a\\\"b\"", "\"c\\\"d\""]);
}

#[test]
fn filter_comparison_with_escaped_quote() {
    let expr = parse_ok("parent:>\"path\\\"name\"");
    filter_is_kind(&expr, &FilterKind::Parent);
    filter_arg_is_comparison(&expr, ComparisonOp::Gt, "\"path\\\"name\"");
}

#[test]
fn filter_range_with_escaped_quotes() {
    let expr = parse_ok("parent:\"a\\\"b\"..\"c\\\"d\"");
    filter_is_kind(&expr, &FilterKind::Parent);
    filter_arg_is_range_dots(&expr, Some("\"a\\\"b\""), Some("\"c\\\"d\""));
}

#[test]
fn complex_query_with_escaped_quotes() {
    let expr = parse_ok("\"test\\\"1\" | (folder:\"path\\\"2\" !\"exclude\\\"3\")");
    let parts = as_or(&expr);
    assert_eq!(parts.len(), 2);
    word_is(&parts[0], "\"test\\\"1\"");

    let and_parts = as_and(&parts[1]);
    assert_eq!(and_parts.len(), 2);
    // 优化器重排过滤器到前面
    match &and_parts[0] {
        Expr::Not(inner) => match &**inner {
            Expr::Term(Term::Word(w)) => assert_eq!(w, "\"exclude\\\"3\""),
            other => panic!("unexpected: {other:?}"),
        },
        other => panic!("expected Not, got: {other:?}"),
    }
    filter_is_kind(&and_parts[1], &FilterKind::Folder);
}

#[test]
fn escaped_quote_with_unicode_chars() {
    let expr = parse_ok("\"你好\\\"世界\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"你好\\\"世界\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn escaped_quote_with_emoji() {
    let expr = parse_ok("\"test\\\"😀\\\"value\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"test\\\"😀\\\"value\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn escaped_quote_preserves_in_optimization() {
    let query = parse_ok("folder:\"a\\\"b\" \"c\\\"d\"");
    let optimized = optimize_query(Query { expr: query });
    let parts = as_and(&optimized.expr);
    assert_eq!(parts.len(), 2);
    word_is(&parts[0], "\"c\\\"d\"");
    filter_is_kind(&parts[1], &FilterKind::Folder);
}

#[test]
fn mixed_escape_patterns() {
    let expr = parse_ok("\"a\\b\\\"c\\\\d\\\"e\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"a\\b\\\"c\\\\d\\\"e\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn trailing_backslash_in_quoted_phrase() {
    let expr = parse_ok("\"trailing\\\\\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"trailing\\\\\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn leading_backslash_in_quoted_phrase() {
    let expr = parse_ok("\"\\\\leading\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"\\\\leading\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn only_backslashes_in_phrase() {
    let expr = parse_ok("\"\\\\\\\\\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"\\\\\\\\\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn alternating_backslash_and_escaped_quotes() {
    let expr = parse_ok("\"\\\\\\\"\\\\\\\"\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"\\\\\\\"\\\\\\\"\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn escaped_quote_between_regular_quotes() {
    let expr = parse_ok("\"start\"middle\\\"\"end\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"start\"middle\\\"\"end\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn multiple_filters_with_escaped_quotes() {
    let expr = parse_ok("folder:\"a\\\"b\" parent:\"c\\\"d\" content:\"e\\\"f\"");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 3);
    // 优化器会重排: infolder/parent (priority 0), then other filters
    filter_is_kind(&parts[0], &FilterKind::Parent);
    filter_is_kind(&parts[1], &FilterKind::Folder);
    filter_is_kind(&parts[2], &FilterKind::Content);
}

#[test]
fn escaped_quote_with_wildcard() {
    let expr = parse_ok("\"*.txt\\\"backup\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"*.txt\\\"backup\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn escaped_quote_in_regex() {
    let expr = parse_ok("regex:\"test\\\"[0-9]+\"");
    match expr {
        Expr::Term(Term::Regex(r)) => assert_eq!(r, "test\\\"[0-9]+"),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn escaped_quote_in_mixed_boolean_expression() {
    let expr = parse_ok("(\"a\\\"b\" | c\\\"d) AND !\"e\\\"f\"");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 2);

    let or_parts = as_or(&parts[0]);
    assert_eq!(or_parts.len(), 2);
    word_is(&or_parts[0], "\"a\\\"b\"");
    word_is(&or_parts[1], "c\\\"d");

    let not_inner = as_not(&parts[1]);
    word_is(not_inner, "\"e\\\"f\"");
}

#[test]
fn tag_filter_with_escaped_quotes() {
    let expr = parse_ok("tag:\"Project\\\"A\\\"\"");
    filter_is_kind(&expr, &FilterKind::Tag);
    filter_arg_raw(&expr, "\"Project\\\"A\\\"\"");
}

#[test]
fn infolder_filter_with_escaped_quotes() {
    let expr = parse_ok("infolder:\"/Users\\\"Name\\\"/Documents\"");
    filter_is_kind(&expr, &FilterKind::InFolder);
    filter_arg_raw(&expr, "\"/Users\\\"Name\\\"/Documents\"");
}

#[test]
fn date_filter_with_escaped_quotes() {
    let expr = parse_ok("dm:\"2024\\\"01\"-\"2024\\\"12\"");
    filter_is_kind(&expr, &FilterKind::DateModified);
    filter_arg_is_range_hyphen(&expr, "\"2024\\\"01\"", "\"2024\\\"12\"");
}

#[test]
fn size_filter_with_escaped_quotes() {
    let expr = parse_ok("size:>\"1\\\"GB\"");
    filter_is_kind(&expr, &FilterKind::Size);
    filter_arg_is_comparison(&expr, ComparisonOp::Gt, "\"1\\\"GB\"");
}

#[test]
fn empty_quotes_after_escaped_quote() {
    let expr = parse_ok("\\\"\"\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\\\"\"\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn empty_quotes_before_escaped_quote() {
    let expr = parse_ok("\"\"\\\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"\"\\\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn complex_nested_structure_with_escapes() {
    let expr = parse_ok("<\"a\\\"b\" | (c\\\"d !\"e\\\"f\")> \"g\\\"h\"");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 2);
    word_is(&parts[1], "\"g\\\"h\"");
}

#[test]
fn escaped_quote_at_every_position() {
    let expr = parse_ok("\\\"a \"b\\\"c\\\"d\" e\\\"");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 3);
    word_is(&parts[0], "\\\"a");
    word_is(&parts[1], "\"b\\\"c\\\"d\"");
    word_is(&parts[2], "e\\\"");
}
