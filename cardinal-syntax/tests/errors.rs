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
