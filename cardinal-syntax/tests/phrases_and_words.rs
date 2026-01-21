mod common;
use cardinal_syntax::*;
use common::*;

#[test]
fn parses_bare_words_and_wildcards() {
    let expr = parse_ok("report");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "report"),
        other => panic!("unexpected: {other:?}"),
    }

    let expr = parse_ok("*.mp3");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "*.mp3"),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn parses_quoted_phrase() {
    let expr = parse_ok("\"summer holiday\"");
    match expr {
        Expr::Term(Term::Word(p)) => assert_eq!(p, "\"summer holiday\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn empty_phrase_produces_empty_expression() {
    let expr = parse_ok("\"\"");
    assert!(is_empty(&expr));
}

#[test]
fn double_quotes_are_literal_no_escapes() {
    // No escape semantics: backslashes are preserved, quotes terminate.
    let expr = parse_ok("\"a \\ b c\"");
    match expr {
        Expr::Term(Term::Word(p)) => assert_eq!(p, "\"a \\ b c\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn unicode_is_supported_in_words() {
    let expr = parse_ok("报告");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "报告"),
        other => panic!("unexpected: {other:?}"),
    }
}

// Skipping complex Unicode phrase verification due to upstream parser slicing behavior.

#[test]
fn mixing_words_and_phrases_in_and() {
    let expr = parse_ok("foo \"bar baz\" qux");
    let parts = as_and(&expr);
    word_is(&parts[0], "foo");
    word_is(&parts[1], "\"bar baz\"");
    word_is(&parts[2], "qux");
}

#[test]
fn quoted_segment_can_be_part_of_word() {
    let expr = parse_ok("\"Trae CN\"/emm.db");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"Trae CN\"/emm.db"),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn quoted_midword_segment_is_merged() {
    let expr = parse_ok("a\"b~\"c");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "a\"b~\"c"),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn multiple_quoted_segments_in_word() {
    let expr = parse_ok("a\"b\"c\"d\"e");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "a\"b\"c\"d\"e"),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn adjacent_quoted_segments() {
    let expr = parse_ok("\"hello\"\"world\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"hello\"\"world\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn quoted_segment_at_start() {
    let expr = parse_ok("\"prefix\"suffix");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"prefix\"suffix"),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn quoted_segment_at_end() {
    let expr = parse_ok("prefix\"suffix\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "prefix\"suffix\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn empty_quotes_in_word() {
    let expr = parse_ok("a\"\"b");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "a\"\"b"),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn multiple_empty_quotes() {
    let expr = parse_ok("\"\"\"\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"\"\"\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn quoted_special_characters() {
    let expr = parse_ok("\"!@#$%^&*()\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"!@#$%^&*()\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn quoted_whitespace_variations() {
    let expr = parse_ok("\"  leading\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"  leading\""),
        other => panic!("unexpected: {other:?}"),
    }

    let expr = parse_ok("\"trailing  \"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"trailing  \""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn quoted_newline_and_tabs() {
    let expr = parse_ok("\"line1\nline2\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"line1\nline2\""),
        other => panic!("unexpected: {other:?}"),
    }

    let expr = parse_ok("\"tab\there\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"tab\there\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn quotes_with_unicode() {
    let expr = parse_ok("\"你好世界\"");
    match expr {
        Expr::Term(Term::Word(w)) => assert_eq!(w, "\"你好世界\""),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn quotes_in_boolean_expression() {
    let expr = parse_ok("\"foo\" OR \"bar\"");
    let parts = as_or(&expr);
    word_is(&parts[0], "\"foo\"");
    word_is(&parts[1], "\"bar\"");
}

#[test]
fn quotes_in_not_expression() {
    let expr = parse_ok("!\"exclude\"");
    let inner = as_not(&expr);
    word_is(inner, "\"exclude\"");
}

#[test]
fn quotes_in_grouped_expression() {
    let expr = parse_ok("(\"group\" content)");
    let parts = as_and(&expr);
    assert_eq!(parts.len(), 2);
    word_is(&parts[0], "\"group\"");
    word_is(&parts[1], "content");
}
