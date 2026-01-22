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
        Term::Word(word) => Term::Word(expand_text_unquoted(word, home)),
        Term::Filter(filter) => Term::Filter(expand_filter(filter, home)),
        // Don't expand when ~ is quoted or in regex
        Term::Regex(pattern) => Term::Regex(pattern),
    }
}

fn expand_text_unquoted(value: String, home: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut in_quotes = false;
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '"' {
            in_quotes = !in_quotes;
            result.push(ch);
        } else if ch == '~' && !in_quotes {
            // Check if followed by / or \ (or at end of string)
            match chars.peek() {
                Some(&'/') | Some(&'\\') => {
                    result.push_str(home);
                }
                None => {
                    result.push_str(home);
                }
                _ => {
                    result.push(ch);
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

pub(crate) fn strip_query_quotes(mut query: Query) -> Query {
    query.expr = strip_expr_quotes(query.expr);
    query
}

fn strip_expr_quotes(expr: Expr) -> Expr {
    match expr {
        Expr::Empty => Expr::Empty,
        Expr::Term(term) => Expr::Term(strip_term_quotes(term)),
        Expr::Not(inner) => Expr::Not(Box::new(strip_expr_quotes(*inner))),
        Expr::And(parts) => Expr::And(parts.into_iter().map(strip_expr_quotes).collect()),
        Expr::Or(parts) => Expr::Or(parts.into_iter().map(strip_expr_quotes).collect()),
    }
}

fn strip_term_quotes(term: Term) -> Term {
    match term {
        Term::Word(word) => Term::Word(strip_query_quotes_text(&word)),
        Term::Filter(mut filter) => {
            if let Some(arg) = &mut filter.argument {
                arg.raw = strip_query_quotes_text(&arg.raw);
                // Also strip quotes from list values
                if let ArgumentKind::List(values) = &mut arg.kind {
                    *values = values.iter().map(|v| strip_query_quotes_text(v)).collect();
                }
            }
            Term::Filter(filter)
        }
        Term::Regex(pattern) => Term::Regex(pattern),
    }
}

pub(crate) fn strip_query_quotes_text(value: &str) -> String {
    if !value.contains('"') && !value.contains('\\') {
        return value.to_string();
    }

    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek().copied() {
                Some('\\') | Some('"') => {
                    result.push(chars.next().expect("peeked value exists"));
                }
                _ => result.push(ch),
            }
            continue;
        }
        if ch == '"' {
            continue;
        }
        result.push(ch);
    }
    result
}

fn expand_filter(mut filter: Filter, home: &str) -> Filter {
    if filter_requires_path(&filter.kind)
        && let Some(argument) = filter.argument.as_mut()
    {
        expand_filter_argument(argument, home);
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
    if let Some(start) = range.start.as_mut()
        && let Some(expanded) = expand_home_prefix(start, home)
    {
        *start = expanded;
    }
    if let Some(end) = range.end.as_mut()
        && let Some(expanded) = expand_home_prefix(end, home)
    {
        *end = expanded;
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
            Expr::Term(Term::Word(text)) => assert_eq!(text, "\"~/docs\""),
            other => panic!("Unexpected expr: {other:?}"),
        }

        let regex = expand("regex:^~/docs$", "/Users/demo");
        match regex.expr {
            Expr::Term(Term::Regex(pattern)) => assert_eq!(pattern, "^~/docs$"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn expands_only_unquoted_leading_tilde() {
        let query = expand("\"\"~/docs", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"\"/Users/demo/docs"),
            other => panic!("Unexpected expr: {other:?}"),
        }

        let query = expand("\"foo\"~/docs", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"foo\"/Users/demo/docs"),
            other => panic!("Unexpected expr: {other:?}"),
        }

        let query = expand("foo~/docs", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "foo/Users/demo/docs"),
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

    #[test]
    fn expands_tilde_after_multiple_empty_quotes() {
        let query = expand("\"\"\"\"~/docs", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"\"\"\"/Users/demo/docs"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn does_not_expand_tilde_in_quoted_section() {
        let query = expand("\"~/docs\"foo", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"~/docs\"foo"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn expands_tilde_after_closing_quote() {
        let query = expand("\"prefix\"~/docs", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"prefix\"/Users/demo/docs"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn handles_tilde_after_quote_pairs() {
        // Matched quote pairs before tilde
        let query = expand("\"x\"\"y\"~/docs", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"x\"\"y\"/Users/demo/docs"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn handles_multiple_tildes_in_value() {
        let query = expand("~/foo/~/bar", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "/Users/demo/foo//Users/demo/bar"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn handles_tilde_not_at_start() {
        let query = expand("prefix~/docs", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "prefix/Users/demo/docs"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn expands_tilde_with_backslash_on_unix() {
        let query = expand(r"~\docs", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, r"/Users/demo\docs"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn handles_empty_string() {
        let query = expand("", "/Users/demo");
        assert!(matches!(query.expr, Expr::Empty));
    }

    #[test]
    fn handles_tilde_with_special_characters_after() {
        let query = expand("~+docs", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "~+docs"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn expands_and_expression_with_mixed_tilde_usage() {
        let query = expand("~/docs AND \"~/quoted\"", "/Users/demo");
        match query.expr {
            Expr::And(parts) => {
                assert_eq!(parts.len(), 2);
                match &parts[0] {
                    Expr::Term(Term::Word(word)) => assert_eq!(word, "/Users/demo/docs"),
                    other => panic!("Expected expanded word, got {other:?}"),
                }
                match &parts[1] {
                    Expr::Term(Term::Word(word)) => assert_eq!(word, "\"~/quoted\""),
                    other => panic!("Expected quoted word, got {other:?}"),
                }
            }
            other => panic!("Expected AND expression, got {other:?}"),
        }
    }

    #[test]
    fn handles_complex_quoted_patterns() {
        // Quote open-close-open-close before tilde
        let query = expand("\"a\"\"b\"~/docs", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"a\"\"b\"/Users/demo/docs"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn expands_range_with_only_start() {
        let filter = Filter {
            kind: FilterKind::InFolder,
            argument: Some(FilterArgument {
                raw: "~/start..".into(),
                kind: ArgumentKind::Range(RangeValue {
                    start: Some("~/start".into()),
                    end: None,
                    separator: RangeSeparator::Dots,
                }),
            }),
        };
        let filter = expand_filter_term(filter, "/Users/demo");
        let argument = filter.argument.expect("argument");
        match argument.kind {
            ArgumentKind::Range(range) => {
                assert_eq!(range.start.as_deref(), Some("/Users/demo/start"));
                assert_eq!(range.end, None);
            }
            other => panic!("Expected range argument, got {other:?}"),
        }
    }

    #[test]
    fn expands_range_with_only_end() {
        let filter = Filter {
            kind: FilterKind::InFolder,
            argument: Some(FilterArgument {
                raw: "..~/end".into(),
                kind: ArgumentKind::Range(RangeValue {
                    start: None,
                    end: Some("~/end".into()),
                    separator: RangeSeparator::Dots,
                }),
            }),
        };
        let filter = expand_filter_term(filter, "/Users/demo");
        let argument = filter.argument.expect("argument");
        match argument.kind {
            ArgumentKind::Range(range) => {
                assert_eq!(range.start, None);
                assert_eq!(range.end.as_deref(), Some("/Users/demo/end"));
            }
            other => panic!("Expected range argument, got {other:?}"),
        }
    }

    #[test]
    fn handles_list_with_non_tilde_values() {
        let query = expand("parent:~/src;regular;~/lib", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Filter(filter)) => {
                let argument = filter.argument.expect("argument");
                match argument.kind {
                    ArgumentKind::List(values) => {
                        assert_eq!(
                            values,
                            vec![
                                String::from("/Users/demo/src"),
                                String::from("regular"),
                                String::from("/Users/demo/lib"),
                            ]
                        );
                    }
                    other => panic!("Expected list argument, got {other:?}"),
                }
            }
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn handles_tilde_alone_in_list() {
        let query = expand("parent:~;foo", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Filter(filter)) => {
                let argument = filter.argument.expect("argument");
                match argument.kind {
                    ArgumentKind::List(values) => {
                        assert_eq!(
                            values,
                            vec![String::from("/Users/demo"), String::from("foo")]
                        );
                    }
                    other => panic!("Expected list argument, got {other:?}"),
                }
            }
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn preserves_nosubfolders_filter_tilde() {
        let query = expand("nosubfolders:~/work", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Filter(filter)) => {
                assert!(matches!(filter.kind, FilterKind::NoSubfolders));
                let argument = filter.argument.expect("argument");
                assert_eq!(argument.raw, "/Users/demo/work");
            }
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn handles_deeply_nested_boolean_with_tildes() {
        let query = expand("(~/a OR ~/b) AND NOT (~/c OR ~/d)", "/Users/demo");
        // Just verify it parses and expands without panicking
        match query.expr {
            Expr::And(_) => {} // Expected structure
            other => panic!("Expected AND expression, got {other:?}"),
        }
    }

    #[test]
    fn handles_tilde_in_bare_argument() {
        let filter = Filter {
            kind: FilterKind::InFolder,
            argument: Some(FilterArgument {
                raw: "~/docs".into(),
                kind: ArgumentKind::Bare,
            }),
        };
        let filter = expand_filter_term(filter, "/Users/demo");
        assert_eq!(filter.argument.expect("argument").raw, "/Users/demo/docs");
    }

    #[test]
    fn handles_tilde_in_phrase_argument() {
        let filter = Filter {
            kind: FilterKind::InFolder,
            argument: Some(FilterArgument {
                raw: "~/my documents".into(),
                kind: ArgumentKind::Phrase,
            }),
        };
        let filter = expand_filter_term(filter, "/Users/demo");
        assert_eq!(
            filter.argument.expect("argument").raw,
            "/Users/demo/my documents"
        );
    }

    #[test]
    fn strip_quotes_from_simple_word() {
        let query = parse_query("\"hello\"").expect("valid");
        let stripped = strip_query_quotes(query);
        match stripped.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "hello"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn strip_quotes_from_multiple_quoted_sections() {
        let query = parse_query("\"hello\" \"world\"").expect("valid");
        let stripped = strip_query_quotes(query);
        match stripped.expr {
            Expr::And(parts) => {
                assert_eq!(parts.len(), 2);
                match &parts[0] {
                    Expr::Term(Term::Word(word)) => assert_eq!(word, "hello"),
                    other => panic!("Unexpected first term: {other:?}"),
                }
                match &parts[1] {
                    Expr::Term(Term::Word(word)) => assert_eq!(word, "world"),
                    other => panic!("Unexpected second term: {other:?}"),
                }
            }
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn strip_quotes_preserves_filters() {
        let query = parse_query("ext:\"rs\"").expect("valid");
        let stripped = strip_query_quotes(query);
        match stripped.expr {
            Expr::Term(Term::Filter(_)) => {} // Should remain a filter
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn strip_quotes_preserves_regex() {
        let query = parse_query("regex:\"test\"").expect("valid");
        let stripped = strip_query_quotes(query);
        match stripped.expr {
            Expr::Term(Term::Regex(pattern)) => assert_eq!(pattern, "test"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn strip_quotes_from_nested_expressions() {
        let query = parse_query("\"a\" OR NOT \"b\"").expect("valid");
        let stripped = strip_query_quotes(query);
        match stripped.expr {
            Expr::Or(parts) => {
                assert_eq!(parts.len(), 2);
                match &parts[0] {
                    Expr::Term(Term::Word(word)) => assert_eq!(word, "a"),
                    other => panic!("Unexpected left term: {other:?}"),
                }
                match &parts[1] {
                    Expr::Not(inner) => match inner.as_ref() {
                        Expr::Term(Term::Word(word)) => assert_eq!(word, "b"),
                        other => panic!("Unexpected NOT target: {other:?}"),
                    },
                    other => panic!("Unexpected right expr: {other:?}"),
                }
            }
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn strip_quotes_handles_quotes_with_content() {
        let query = parse_query("\"a\"").expect("valid");
        let stripped = strip_query_quotes(query);
        match stripped.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "a"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn strip_quotes_handles_multiple_quotes_in_word() {
        let query = parse_query("\"hello\"\"world\"").expect("valid");
        let stripped = strip_query_quotes(query);
        match stripped.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "helloworld"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn strip_quotes_unescapes_escaped_quotes() {
        let query = parse_query(r#"\"hello\""#).expect("valid");
        let stripped = strip_query_quotes(query);
        match stripped.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"hello\""),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn strip_quotes_unescapes_backslashes() {
        let query = parse_query(r#""C:\\path""#).expect("valid");
        let stripped = strip_query_quotes(query);
        match stripped.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, r"C:\path"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn strip_quotes_unescapes_mixed_content_in_word() {
        let query = parse_query(r#""foo\"bar\"baz""#).expect("valid");
        let stripped = strip_query_quotes(query);
        match stripped.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "foo\"bar\"baz"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn strip_quotes_preserves_non_escape_sequences() {
        let query = parse_query(r#""a\bc""#).expect("valid");
        let stripped = strip_query_quotes(query);
        match stripped.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "a\\bc"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn strip_quotes_unescapes_multiple_backslashes() {
        let query = parse_query(r#""C\\\\path""#).expect("valid");
        let stripped = strip_query_quotes(query);
        match stripped.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "C\\\\path"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn strip_quotes_unescapes_bare_argument() {
        let filter = Filter {
            kind: FilterKind::InFolder,
            argument: Some(FilterArgument {
                raw: r#""C\\Users\\demo""#.into(),
                kind: ArgumentKind::Bare,
            }),
        };
        let query = Query {
            expr: Expr::Term(Term::Filter(filter)),
        };
        let stripped = strip_query_quotes(query);
        match stripped.expr {
            Expr::Term(Term::Filter(filter)) => {
                let arg = filter.argument.expect("argument");
                assert_eq!(arg.raw, "C\\Users\\demo".replace("\\\\", "\\"));
            }
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn strip_quotes_unescapes_phrase_argument() {
        let filter = Filter {
            kind: FilterKind::InFolder,
            argument: Some(FilterArgument {
                raw: r#""C\\Users\\demo Documents""#.into(),
                kind: ArgumentKind::Phrase,
            }),
        };
        let query = Query {
            expr: Expr::Term(Term::Filter(filter)),
        };
        let stripped = strip_query_quotes(query);
        match stripped.expr {
            Expr::Term(Term::Filter(filter)) => {
                let arg = filter.argument.expect("argument");
                assert_eq!(arg.raw, "C\\Users\\demo Documents".replace("\\\\", "\\"));
            }
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn strip_quotes_unescapes_list_argument_values() {
        let filter = Filter {
            kind: FilterKind::InFolder,
            argument: Some(FilterArgument {
                raw: String::new(),
                kind: ArgumentKind::List(vec![r#""C\\path""#.into(), r#""D\\data""#.into()]),
            }),
        };
        let query = Query {
            expr: Expr::Term(Term::Filter(filter)),
        };
        let stripped = strip_query_quotes(query);
        match stripped.expr {
            Expr::Term(Term::Filter(filter)) => {
                let arg = filter.argument.expect("argument");
                match arg.kind {
                    ArgumentKind::List(values) => {
                        assert_eq!(
                            values,
                            vec![
                                "C\\path".replace("\\\\", "\\"),
                                "D\\data".replace("\\\\", "\\"),
                            ]
                        );
                    }
                    other => panic!("Expected list argument, got {other:?}"),
                }
            }
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    // Tests for ~ inside and outside quotes at various positions

    #[test]
    fn tilde_inside_quotes_single_char() {
        // "~" should not expand
        let query = expand("\"~\"", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"~\""),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn tilde_inside_quotes_with_path() {
        // "~/docs" should not expand
        let query = expand("\"~/docs\"", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"~/docs\""),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn tilde_outside_quotes_after() {
        // ~"docs" - tilde at start but followed by quote (not / or \), won't expand
        let query = expand("~\"docs\"", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "~\"docs\""),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn tilde_inside_quotes_after_content() {
        // "prefix~" - tilde inside quotes but not at start should not expand
        let query = expand("\"prefix~\"", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"prefix~\""),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn tilde_inside_quotes_midpath() {
        // "prefix~/docs" - tilde not at word start inside quotes
        let query = expand("\"prefix~/docs\"", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"prefix~/docs\""),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn tilde_surrounded_by_quotes() {
        // prefix"~"suffix - tilde inside quotes with content on both sides
        let query = expand("prefix\"~\"suffix", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "prefix\"~\"suffix"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn quoted_tilde_then_unquoted_suffix() {
        // "~"suffix - quoted tilde followed by unquoted content
        let query = expand("\"~\"suffix", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"~\"suffix"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn quoted_tilde_path_then_unquoted() {
        // "~/docs"rest - quoted tilde path followed by unquoted content
        let query = expand("\"~/docs\"rest", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"~/docs\"rest"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn expanded_tilde_then_quoted_suffix() {
        // ~/docs"suffix" - tilde expands, then quoted content follows
        let query = expand("~/docs\"suffix\"", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "/Users/demo/docs\"suffix\""),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn quoted_tilde_slash_then_unquoted() {
        // "~/"suffix - quoted tilde with slash, then unquoted
        let query = expand("\"~/\"suffix", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"~/\"suffix"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn tilde_between_two_quoted_sections() {
        // "a"~"b" - tilde not followed by / or \, won't expand
        let query = expand("\"a\"~\"b\"", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"a\"~\"b\""),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn tilde_slash_between_quoted_sections() {
        // "a"~/docs"b" - tilde after quotes followed by /, should expand
        let query = expand("\"a\"~/docs\"b\"", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"a\"/Users/demo/docs\"b\""),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn multiple_tildes_quoted_and_unquoted() {
        // ~"test"~/docs - first tilde not followed by /, second tilde (after quotes) should expand
        let query = expand("~\"test\"~/docs", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "~\"test\"/Users/demo/docs"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn quoted_section_with_tilde_not_at_start() {
        // a"~/docs" - regular char, then quoted tilde path
        let query = expand("a\"~/docs\"", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "a\"~/docs\""),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn empty_quotes_tilde_empty_quotes() {
        // ""~"" - tilde not followed by / or \, won't expand
        let query = expand("\"\"~\"\"", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"\"~\"\""),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn tilde_only_after_quote_closes_midword() {
        // "prefix"~ - tilde alone after quotes, should expand to home
        let query = expand("\"prefix\"~", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"prefix\"/Users/demo"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn complex_quote_tilde_interleaving() {
        // "a"~"b"~/c"d" - first unquoted ~ not followed by /, second is, expand second
        let query = expand("\"a\"~\"b\"~/c\"d\"", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"a\"~\"b\"/Users/demo/c\"d\""),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn tilde_with_backslash_in_quotes() {
        // "~\docs" - tilde with backslash inside quotes
        let query = expand("\"~\\docs\"", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"~\\docs\""),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn unmatched_quote_affects_tilde() {
        // Since unmatched quotes cause parse errors, test quote-safe scenarios
        // "a"b~/c - quote closes after 'a', 'b' is unquoted before tilde, should expand
        let query = expand("\"a\"b~/c", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"a\"b/Users/demo/c"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn tilde_slash_at_start_expands() {
        // ~/docs - standard case, should expand
        let query = expand("~/docs", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "/Users/demo/docs"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn tilde_backslash_at_start_expands() {
        // ~\docs - backslash variant, should expand
        let query = expand(r"~\docs", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, r"/Users/demo\docs"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn empty_quotes_then_tilde_slash_expands() {
        // ""~/docs - empty quotes at start, then tilde with slash
        let query = expand("\"\"~/docs", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"\"/Users/demo/docs"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn tilde_alone_at_start_expands() {
        // ~ - just tilde alone
        let query = expand("~", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "/Users/demo"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn quotes_then_tilde_nonslash_no_expand() {
        // ""~x - empty quotes then tilde not followed by / or \
        let query = expand("\"\"~x", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => assert_eq!(word, "\"\"~x"),
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn multiple_expandable_tildes_all_expand() {
        // ~/a/~/b/~/c - all three tildes should expand
        let query = expand("~/a/~/b/~/c", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => {
                assert_eq!(word, "/Users/demo/a//Users/demo/b//Users/demo/c")
            }
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn tildes_mixed_with_quotes_expand_unquoted_only() {
        // ~/a/"~/b"/~/c - first and third expand, second (in quotes) doesn't
        let query = expand("~/a/\"~/b\"/~/c", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => {
                assert_eq!(word, "/Users/demo/a/\"~/b\"//Users/demo/c")
            }
            other => panic!("Unexpected expr: {other:?}"),
        }
    }

    #[test]
    fn multiple_tildes_some_invalid() {
        // ~/a/~x/~/b - first and third expand, second (not followed by / or \) doesn't
        let query = expand("~/a/~x/~/b", "/Users/demo");
        match query.expr {
            Expr::Term(Term::Word(word)) => {
                assert_eq!(word, "/Users/demo/a/~x//Users/demo/b")
            }
            other => panic!("Unexpected expr: {other:?}"),
        }
    }
}
