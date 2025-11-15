#![allow(dead_code)]
//! Shared helpers for `cardinal-syntax` integration tests.

use cardinal_syntax::*;

pub fn parse_ok(input: &str) -> Expr {
    parse_query(input).unwrap().expr
}

pub fn parse_err(input: &str) -> ParseError {
    parse_query(input).unwrap_err()
}

pub fn is_empty(expr: &Expr) -> bool {
    matches!(expr, Expr::Empty)
}

pub fn as_and(expr: &Expr) -> &Vec<Expr> {
    match expr {
        Expr::And(parts) => parts,
        other => panic!("expected And, got: {other:?}"),
    }
}

pub fn as_or(expr: &Expr) -> &Vec<Expr> {
    match expr {
        Expr::Or(parts) => parts,
        other => panic!("expected Or, got: {other:?}"),
    }
}

pub fn as_not(expr: &Expr) -> &Expr {
    match expr {
        Expr::Not(inner) => inner,
        other => panic!("expected Not, got: {other:?}"),
    }
}

pub fn as_term(expr: &Expr) -> &Term {
    match expr {
        Expr::Term(t) => t,
        other => panic!("expected Term, got: {other:?}"),
    }
}

pub fn word_is(expr: &Expr, expected: &str) {
    match as_term(expr) {
        Term::Word(w) => assert_eq!(w, expected),
        other => panic!("expected Word, got: {other:?}"),
    }
}

pub fn phrase_is(expr: &Expr, expected: &str) {
    match as_term(expr) {
        Term::Phrase(w) => assert_eq!(w, expected),
        other => panic!("expected Phrase, got: {other:?}"),
    }
}

pub fn regex_is(expr: &Expr, expected: &str) {
    match as_term(expr) {
        Term::Regex(pat) => assert_eq!(pat, expected),
        other => panic!("expected Regex, got: {other:?}"),
    }
}

pub fn filter_kind(expr: &Expr) -> (&FilterKind, &Option<FilterArgument>) {
    match as_term(expr) {
        Term::Filter(f) => (&f.kind, &f.argument),
        other => panic!("expected Filter, got: {other:?}"),
    }
}

pub fn filter_is_custom(expr: &Expr, name: &str) {
    let (k, _) = filter_kind(expr);
    match k {
        FilterKind::Custom(n) => assert_eq!(n, name),
        other => panic!("expected Custom({name}), got: {other:?}"),
    }
}

pub fn filter_is_kind(expr: &Expr, kind: &FilterKind) {
    let (k, _) = filter_kind(expr);
    assert_eq!(k, kind);
}

pub fn filter_arg_none(expr: &Expr) {
    let (_, arg) = filter_kind(expr);
    assert!(arg.is_none());
}

pub fn filter_arg_raw(expr: &Expr, expected: &str) {
    let (_, arg) = filter_kind(expr);
    let arg = arg.as_ref().expect("missing argument");
    assert_eq!(arg.raw, expected);
}

pub fn filter_arg_is_list(expr: &Expr, expected: &[&str]) {
    let (_, arg) = filter_kind(expr);
    let arg = arg.as_ref().expect("missing argument");
    match &arg.kind {
        ArgumentKind::List(values) => {
            let exp: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
            assert_eq!(*values, exp);
        }
        other => panic!("expected List, got: {other:?}"),
    }
}

pub fn filter_arg_is_range_dots(expr: &Expr, start: Option<&str>, end: Option<&str>) {
    let (_, arg) = filter_kind(expr);
    let arg = arg.as_ref().expect("missing argument");
    match &arg.kind {
        ArgumentKind::Range(range) => {
            assert!(matches!(range.separator, RangeSeparator::Dots));
            assert_eq!(range.start.as_deref(), start);
            assert_eq!(range.end.as_deref(), end);
        }
        other => panic!("expected Range, got: {other:?}"),
    }
}

pub fn filter_arg_is_range_hyphen(expr: &Expr, start: &str, end: &str) {
    let (_, arg) = filter_kind(expr);
    let arg = arg.as_ref().expect("missing argument");
    match &arg.kind {
        ArgumentKind::Range(range) => {
            assert!(matches!(range.separator, RangeSeparator::Hyphen));
            assert_eq!(range.start.as_deref(), Some(start));
            assert_eq!(range.end.as_deref(), Some(end));
        }
        other => panic!("expected Range, got: {other:?}"),
    }
}

pub fn filter_arg_is_comparison(expr: &Expr, op: ComparisonOp, value: &str) {
    let (_, arg) = filter_kind(expr);
    let arg = arg.as_ref().expect("missing argument");
    match &arg.kind {
        ArgumentKind::Comparison(ComparisonValue { op: o, value: v }) => {
            assert_eq!(*o, op);
            assert_eq!(v, value);
        }
        other => panic!("expected Comparison, got: {other:?}"),
    }
}

pub fn assert_keyword_and_gaps(expr: &Expr) {
    // Expect `Expr::And` possibly containing `Expr::Empty` from keyword AND parsing.
    let parts = as_and(expr);
    assert!(parts.iter().any(|e| matches!(e, Expr::Empty)));
}

pub fn assert_keyword_or_gaps(expr: &Expr) {
    let parts = as_or(expr);
    assert!(parts.iter().any(|e| matches!(e, Expr::Empty)));
}
