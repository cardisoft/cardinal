mod common;
use common::*;

#[test]
fn unexpected_trailing_characters() {
    let err = parse_err("foo)");
    assert!(err.message.contains("unexpected closing"));
}

#[test]
fn missing_quote_is_reported() {
    let err = parse_err("\"unterminated");
    assert!(err.message.contains("missing closing quote"));
}

#[test]
fn colon_is_treated_as_word_not_error() {
    let expr = parse_ok(":");
    match expr {
        cardinal_syntax::Expr::Term(cardinal_syntax::Term::Word(w)) => assert_eq!(w, ":"),
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn error_positions_are_byte_offsets() {
    let err = parse_err("<a b");
    assert!(err.position > 0);
}

#[test]
fn unclosed_quote_at_start() {
    let err = parse_err("\"unclosed");
    assert!(err.message.contains("missing closing quote"));
}

#[test]
fn unclosed_quote_in_middle() {
    let err = parse_err("foo \"unclosed bar");
    assert!(err.message.contains("missing closing quote"));
}

#[test]
fn unclosed_quote_with_special_chars() {
    let err = parse_err("\"test!@#$");
    assert!(err.message.contains("missing closing quote"));
}

#[test]
fn unclosed_quote_in_filter() {
    let err = parse_err("ext:\"txt");
    assert!(err.message.contains("missing closing quote"));
}

#[test]
fn unclosed_quote_after_boolean() {
    let err = parse_err("foo OR \"bar");
    assert!(err.message.contains("missing closing quote"));
}

#[test]
fn unclosed_quote_in_group() {
    let err = parse_err("(\"foo bar)");
    assert!(err.message.contains("missing closing quote"));
}

#[test]
fn unclosed_quote_with_unicode() {
    let err = parse_err("\"你好");
    assert!(err.message.contains("missing closing quote"));
}
