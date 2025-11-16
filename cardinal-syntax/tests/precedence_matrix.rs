use cardinal_syntax::*;

fn w(s: &str) -> Expr {
    Expr::Term(Term::Word(s.to_string()))
}

fn top_is_and(expr: &Expr) -> (&[Expr], bool) {
    match expr {
        Expr::And(parts) => (parts, true),
        other => (std::slice::from_ref(other), false),
    }
}

#[test]
fn or_has_higher_precedence_than_and() {
    let cases = [
        (
            "a b|c d",
            3,
            vec![
                // a
                |e: &Expr| matches!(e, Expr::Term(Term::Word(s)) if s == "a"),
                // b|c
                |e: &Expr| {
                    matches!(e, Expr::Or(parts) if parts.len() == 2
                        && matches!(&parts[0], Expr::Term(Term::Word(s)) if s == "b")
                        && matches!(&parts[1], Expr::Term(Term::Word(s)) if s == "c")
                    )
                },
                // d
                |e: &Expr| matches!(e, Expr::Term(Term::Word(s)) if s == "d"),
            ],
        ),
        (
            "a|b c|d",
            2,
            vec![
                |e: &Expr| matches!(e, Expr::Or(parts) if parts.len() == 2),
                |e: &Expr| matches!(e, Expr::Or(parts) if parts.len() == 2),
            ],
        ),
        (
            "a OR b AND c",
            2,
            vec![
                |e: &Expr| matches!(e, Expr::Or(parts) if parts.len() == 2),
                |e: &Expr| matches!(e, Expr::Term(Term::Word(s)) if s == "c"),
            ],
        ),
    ];

    for (src, and_len, validators) in cases {
        let q = parse_query(src).unwrap();
        let (parts, is_and) = top_is_and(&q.expr);
        assert!(is_and, "top should be AND for `{}`", src);
        assert_eq!(parts.len(), and_len, "AND arity for `{}`", src);
        for (e, check) in parts.iter().zip(validators.into_iter()) {
            assert!(check(e), "validator failed for query `{}`: {e:?}", src);
        }
    }
}

#[test]
fn not_binds_tighter_than_or_and() {
    let cases = [
        (
            "!a|b c",
            vec![
                |e: &Expr| {
                    matches!(e, Expr::Or(parts)
                        if matches!(&parts[0], Expr::Not(_))
                        && matches!(&parts[1], Expr::Term(Term::Word(s)) if s == "b")
                    )
                },
                |e: &Expr| matches!(e, Expr::Term(Term::Word(s)) if s == "c"),
            ],
        ),
        (
            "NOT a OR b AND NOT c",
            vec![
                |e: &Expr| {
                    matches!(e, Expr::Or(parts)
                        if matches!(&parts[0], Expr::Not(_))
                        && matches!(&parts[1], Expr::Term(Term::Word(s)) if s == "b")
                    )
                },
                |e: &Expr| matches!(e, Expr::Not(_)),
            ],
        ),
        (
            "!!!x | y z",
            vec![
                |e: &Expr| {
                    matches!(e, Expr::Or(parts)
                        if matches!(&parts[0], Expr::Not(inner) if !matches!(&**inner, Expr::Not(_)))
                    )
                },
                |e: &Expr| matches!(e, Expr::Term(Term::Word(s)) if s == "z"),
            ],
        ),
    ];

    for (src, validators) in cases {
        let q = parse_query(src).unwrap();
        let (parts, is_and) = top_is_and(&q.expr);
        assert!(is_and, "expected AND at top for `{}`", src);
        assert_eq!(parts.len(), validators.len());
        for (e, check) in parts.iter().zip(validators.into_iter()) {
            assert!(check(e), "failed validator for `{}`: {e:?}", src);
        }
    }
}

#[test]
fn groups_override_precedence() {
    // (a|b) c should be AND[a|b, c]
    let q = parse_query("(a|b) c").unwrap();
    let Expr::And(parts) = q.expr else {
        panic!("expected AND");
    };
    assert_eq!(parts.len(), 2);
    assert!(matches!(&parts[0], Expr::Or(v) if v.len() == 2));
    assert_eq!(parts[1], w("c"));

    // a (b|c) d => AND[a, b|c, d]
    let q = parse_query("a (b|c) d").unwrap();
    let Expr::And(parts) = q.expr else {
        panic!("expected AND");
    };
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], w("a"));
    assert!(matches!(&parts[1], Expr::Or(v) if v.len() == 2));
    assert_eq!(parts[2], w("d"));

    // <a|b> c|d => AND[ a|b, Or(c,d) ] due to or precedence
    let q = parse_query("<a|b> c|d").unwrap();
    let Expr::And(parts) = q.expr else {
        panic!("expected AND");
    };
    assert_eq!(parts.len(), 2);
    assert!(matches!(&parts[0], Expr::Or(v) if v.len() == 2));
    assert!(matches!(&parts[1], Expr::Or(v) if v.len() == 2));
}

#[test]
fn regex_terms_participate_in_boolean_logic() {
    // Ensure regex: behaves as a normal term in OR/AND contexts
    let q = parse_query("regex:^Rep OR notes ext:txt").unwrap();
    let Expr::And(parts) = q.expr else {
        panic!("expected AND");
    };
    assert_eq!(parts.len(), 2);
    assert!(matches!(&parts[0], Expr::Or(v)
        if matches!(&v[0], Expr::Term(Term::Regex(p)) if p == "^Rep")
        && matches!(&v[1], Expr::Term(Term::Word(s)) if s == "notes")
    ));
    assert!(matches!(&parts[1], Expr::Term(Term::Filter(_))));
}
