use cardinal_syntax::{
    ArgumentKind, ComparisonValue, Expr, Filter, FilterArgument, FilterKind, Query, RangeValue,
    Term,
};
use std::env;

pub(crate) fn expand_query_home_dirs(query: Query) -> Query {
    let Some(home) = home_dir() else { return query };
    expand_query_home_dirs_with_home(query, &home)
}

fn expand_query_home_dirs_with_home(mut query: Query, home: &str) -> Query {
    query.expr = expand_expr(query.expr, home);
    query
}

fn expand_expr(expr: Expr, home: &str) -> Expr {
    match expr {
        Expr::Empty => Expr::Empty,
        Expr::Term(term) => Expr::Term(expand_term(term, home)),
        Expr::Not(inner) => Expr::Not(Box::new(expand_expr(*inner, home))),
        Expr::And(parts) => Expr::And(
            parts
                .into_iter()
                .map(|part| expand_expr(part, home))
                .collect(),
        ),
        Expr::Or(parts) => Expr::Or(
            parts
                .into_iter()
                .map(|part| expand_expr(part, home))
                .collect(),
        ),
    }
}

fn expand_term(term: Term, home: &str) -> Term {
    match term {
        Term::Word(word) => Term::Word(expand_text(word, home)),
        Term::Filter(filter) => Term::Filter(expand_filter(filter, home)),
        // Don't expand when ~ is quoted or in regex
        Term::Phrase(phrase) => Term::Phrase(phrase),
        Term::Regex(pattern) => Term::Regex(pattern),
    }
}

fn expand_filter(mut filter: Filter, home: &str) -> Filter {
    if filter_requires_path(&filter.kind) {
        if let Some(argument) = filter.argument.as_mut() {
            expand_filter_argument(argument, home);
        }
    }
    filter
}

fn filter_requires_path(kind: &FilterKind) -> bool {
    // Only expand filters whose semantics require filesystem-like paths.
    matches!(
        kind,
        FilterKind::Parent | FilterKind::InFolder | FilterKind::NoSubfolders
    )
}

fn expand_filter_argument(argument: &mut FilterArgument, home: &str) {
    let raw = std::mem::take(&mut argument.raw);
    argument.raw = expand_text(raw, home);
    match &mut argument.kind {
        ArgumentKind::Bare | ArgumentKind::Phrase => {}
        ArgumentKind::List(values) => {
            for value in values.iter_mut() {
                if let Some(expanded) = expand_home_prefix(value, home) {
                    *value = expanded;
                }
            }
        }
        ArgumentKind::Range(range) => expand_range(range, home),
        ArgumentKind::Comparison(value) => expand_comparison(value, home),
    }
}

fn expand_range(range: &mut RangeValue, home: &str) {
    if let Some(start) = range.start.as_mut() {
        if let Some(expanded) = expand_home_prefix(start, home) {
            *start = expanded;
        }
    }
    if let Some(end) = range.end.as_mut() {
        if let Some(expanded) = expand_home_prefix(end, home) {
            *end = expanded;
        }
    }
}

fn expand_comparison(value: &mut ComparisonValue, home: &str) {
    if let Some(expanded) = expand_home_prefix(&value.value, home) {
        value.value = expanded;
    }
}

fn expand_text(value: String, home: &str) -> String {
    if let Some(expanded) = expand_home_prefix(&value, home) {
        expanded
    } else {
        value
    }
}

fn expand_home_prefix(value: &str, home: &str) -> Option<String> {
    // Support Unix `~/foo` and Windows-equivalent `~\foo` prefixes while
    // leaving other `~` usages (e.g., `~someone`) untouched.
    if !value.starts_with('~') {
        return None;
    }
    let remainder = &value[1..];
    if remainder.is_empty() {
        return Some(home.to_string());
    }
    let mut chars = remainder.chars();
    match chars.next() {
        Some('/' | '\\') => {
            let mut expanded = String::with_capacity(home.len() + remainder.len());
            expanded.push_str(home);
            expanded.push_str(remainder);
            Some(expanded)
        }
        _ => None,
    }
}

fn home_dir() -> Option<String> {
    env::var("HOME").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardinal_syntax::{RangeSeparator, Term, parse_query};

    fn expand(input: &str, home: &str) -> Query {
        let parsed = parse_query(input).expect("valid query");
        expand_query_home_dirs_with_home(parsed, home)
    }

    fn expand_filter_term(filter: Filter, home: &str) -> Filter {
        let query = Query {
            expr: Expr::Term(Term::Filter(filter)),
        };
        match expand_query_home_dirs_with_home(query, home).expr {
            Expr::Term(Term::Filter(filter)) => filter,
            other => panic!("Expected filter expr, got {other:?}"),
        }
    }

    #[test]
    fn expands_tilde_in_word_terms() {
        let query = expand("~/code", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "/Users/demo/code"),
            other => panic!("Unexpected expr: {other:?}"),
        };
    }

    #[test]
    fn leaves_regular_terms_untouched() {
        let query = expand("docs", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "docs"),
            other => panic!("Unexpected expr: {other:?}"),
        };
    }

    #[test]
    fn expands_word_with_only_tilde() {
        let query = expand("~", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "/Users/demo"),
            other => panic!("Unexpected expr: {other:?}"),
        };
    }

    #[test]
    fn expands_path_filters() {
        let query = expand("infolder:~/projects", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Filter(filter)) => {
                assert!(matches!(filter.kind, FilterKind::InFolder));
                let argument = filter.argument.expect("argument");
                assert_eq!(argument.raw, "/Users/demo/projects");
            }
            other => panic!("Unexpected expr: {other:?}"),
        };
    }

    #[test]
    fn ignores_non_path_filters() {
        let query = expand("ext:~", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Filter(filter)) => {
                assert!(matches!(filter.kind, FilterKind::Ext));
                let argument = filter.argument.expect("argument");
                assert_eq!(argument.raw, "~");
            }
            other => panic!("Unexpected expr: {other:?}"),
        };
    }

    #[test]
    fn expands_nested_boolean_exprs() {
        let query = expand("~/docs OR NOT parent:~/Downloads", "/Users/demo");
        match query.expr {
            Expr::Or(parts) => {
                assert_eq!(parts.len(), 2);
                match &parts[0] {
                    Expr::Term(Term::Word(word)) => assert_eq!(word, "/Users/demo/docs"),
                    other => panic!("Unexpected left expr: {other:?}"),
                }
                match &parts[1] {
                    Expr::Not(inner) => match inner.as_ref() {
                        Expr::Term(Term::Filter(filter)) => {
                            assert!(matches!(filter.kind, FilterKind::Parent));
                            let argument = filter.argument.clone().expect("argument");
                            assert_eq!(argument.raw, "/Users/demo/Downloads");
                        }
                        other => panic!("Unexpected NOT target: {other:?}"),
                    },
                    other => panic!("Unexpected right expr: {other:?}"),
                }
            }
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn does_not_expand_phrases_or_regexes() {
        let phrase = expand("\"~/docs\"", "/Users/demo");
        match phrase.expr {
            Expr::Term(Term::Phrase(text)) => assert_eq!(text, "~/docs"),
            other => panic!("Unexpected expr: {other:?}"),
        }

        let regex = expand("regex:^~/docs$", "/Users/demo");
        match regex.expr {
            Expr::Term(Term::Regex(pattern)) => assert_eq!(pattern, "^~/docs$"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn expands_list_arguments() {
        let query = expand("parent:~/src;~/lib", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Filter(filter)) => {
                let argument = filter.argument.expect("argument");
                match argument.kind {
                    ArgumentKind::List(values) => {
                        assert_eq!(
                            values,
                            vec![
                                String::from("/Users/demo/src"),
                                String::from("/Users/demo/lib"),
                            ]
                        );
                    }
                    other => panic!("Expected list argument, got {other:?}"),
                }
            }
            other => panic!("Unexpected expr: {other:?}"),
        };
    }

    #[test]
    fn expands_range_arguments() {
        let filter = Filter {
            kind: FilterKind::InFolder,
            argument: Some(FilterArgument {
                raw: "~..~/scratch".into(),
                kind: ArgumentKind::Range(RangeValue {
                    start: Some("~".into()),
                    end: Some("~/scratch".into()),
                    separator: RangeSeparator::Dots,
                }),
            }),
        };
        let filter = expand_filter_term(filter, "/Users/demo");
        let argument = filter.argument.expect("argument");
        match argument.kind {
            ArgumentKind::Range(range) => {
                assert_eq!(range.start.as_deref(), Some("/Users/demo"));
                assert_eq!(range.end.as_deref(), Some("/Users/demo/scratch"));
            }
            other => panic!("Expected range argument, got {other:?}"),
        }
    }

    #[test]
    fn expands_comparison_arguments() {
        let query = expand("parent:>=~/docs", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Filter(filter)) => {
                let argument = filter.argument.expect("argument");
                match argument.kind {
                    ArgumentKind::Comparison(value) => {
                        assert_eq!(value.value, "/Users/demo/docs");
                    }
                    other => panic!("Expected comparison argument, got {other:?}"),
                }
            }
            other => panic!("Unexpected expr: {other:?}"),
        };
    }

    #[test]
    fn expands_windows_style_separators() {
        let query = expand(r"parent:~\\Downloads", r"C:\\Users\\demo");
        match query.expr {
            Expr::Term(Term::Filter(filter)) => {
                let argument = filter.argument.expect("argument");
                assert_eq!(argument.raw, r"C:\\Users\\demo\\Downloads");
            }
            other => panic!("Unexpected expr: {other:?}"),
        };
    }

    #[test]
    fn ignores_named_home_prefixes() {
        let query = expand("parent:~shared/docs", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Filter(filter)) => {
                let argument = filter.argument.expect("argument");
                assert_eq!(argument.raw, "~shared/docs");
            }
            other => panic!("Unexpected expr: {other:?}"),
        };
    }
}
