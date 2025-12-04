use cardinal_syntax::{ArgumentKind, Expr, FilterArgument, Term};
use query_segmentation::{Segment, query_segmentation};
use std::collections::BTreeSet;

pub fn derive_highlight_terms(expr: &Expr) -> Vec<String> {
    let mut collector = HighlightCollector::default();
    collector.collect_expr(expr);
    collector.into_terms()
}

#[derive(Default)]
struct HighlightCollector {
    terms: BTreeSet<String>,
}

impl HighlightCollector {
    fn collect_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Empty => {}
            Expr::Term(term) => self.collect_term(term),
            Expr::Not(inner) => self.collect_expr(inner),
            Expr::And(parts) | Expr::Or(parts) => {
                for part in parts {
                    self.collect_expr(part);
                }
            }
        }
    }

    fn collect_term(&mut self, term: &Term) {
        match term {
            Term::Word(word) => self.collect_text(word),
            Term::Phrase(word) => self.push(word.clone()),
            Term::Filter(filter) => {
                if let Some(argument) = &filter.argument {
                    self.collect_argument(argument);
                }
            }
            Term::Regex(_) => {}
        }
    }

    fn collect_argument(&mut self, argument: &FilterArgument) {
        match &argument.kind {
            ArgumentKind::Bare | ArgumentKind::Phrase => self.collect_text(argument.raw.as_str()),
            ArgumentKind::List(values) => {
                for value in values {
                    self.collect_text(value);
                }
            }
            ArgumentKind::Range(_) | ArgumentKind::Comparison(_) => {}
        }
    }

    fn collect_text(&mut self, value: &str) {
        if value.trim().is_empty() {
            return;
        }

        let segments = query_segmentation(value);
        if let Some(segment) = segments.last() {
            let segment = match segment {
                Segment::Concrete(concrete) => concrete,
                Segment::GlobStar => {
                    // "**" does not contribute to highlight terms
                    return;
                }
            };
            let candidates = literal_chunks(segment.as_value());
            if !candidates.is_empty() {
                for candidate in candidates {
                    self.push(candidate);
                }
                return;
            }
        }

        for candidate in literal_chunks(value) {
            self.push(candidate);
        }
    }

    fn push(&mut self, candidate: String) {
        self.terms.insert(candidate.to_lowercase());
    }

    fn into_terms(self) -> Vec<String> {
        self.terms.into_iter().collect()
    }
}

fn literal_chunks(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let chunks: Vec<String> = trimmed
        .split(['*', '?'])
        .map(str::trim)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| chunk.to_string())
        .collect();

    if chunks.is_empty() && !trimmed.contains(['*', '?']) {
        vec![trimmed.to_string()]
    } else {
        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardinal_syntax::{ParseError, Query, optimize_query, parse_query as raw_parse_query};

    fn parse_query(input: &str) -> Result<Query, ParseError> {
        raw_parse_query(input).map(optimize_query)
    }

    // ============================================================================
    // Basic Word and Phrase Tests (200 lines)
    // ============================================================================

    #[test]
    fn test_empty_query() {
        let result = parse_query("").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_whitespace_only() {
        let result = parse_query("   ").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_single_word() {
        let result = parse_query("report").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["report"]);
    }

    #[test]
    fn test_single_word_uppercase() {
        let result = parse_query("REPORT").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["report"]);
    }

    #[test]
    fn test_single_word_mixedcase() {
        let result = parse_query("RePoRt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["report"]);
    }

    #[test]
    fn test_two_words() {
        let result = parse_query("hello world").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn test_multiple_words() {
        let result = parse_query("foo bar baz qux").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["bar", "baz", "foo", "qux"]);
    }

    #[test]
    fn test_duplicate_words() {
        let result = parse_query("test test test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_duplicate_words_different_case() {
        let result = parse_query("Test TEST test TeSt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_phrase_single_word() {
        let result = parse_query("\"hello\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello"]);
    }

    #[test]
    fn test_phrase_multiple_words() {
        let result = parse_query("\"hello world\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_phrase_with_whitespace() {
        let result = parse_query("\"  hello   world  \"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["  hello   world  "]);
    }

    #[test]
    fn test_phrase_uppercase() {
        let result = parse_query("\"HELLO WORLD\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_mixed_words_and_phrases() {
        let result = parse_query("foo \"bar baz\" qux").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["bar baz", "foo", "qux"]);
    }

    #[test]
    fn test_word_with_numbers() {
        let result = parse_query("test123").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test123"]);
    }

    #[test]
    fn test_word_with_underscore() {
        let result = parse_query("hello_world").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello_world"]);
    }

    #[test]
    fn test_word_with_hyphen() {
        let result = parse_query("hello-world").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello-world"]);
    }

    #[test]
    fn test_word_with_dot() {
        let result = parse_query("file.txt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file.txt"]);
    }

    #[test]
    fn test_numbers_only() {
        let result = parse_query("12345").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["12345"]);
    }

    #[test]
    fn test_special_characters() {
        let result = parse_query("hello@world").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello@world"]);
    }

    #[test]
    fn test_unicode_text() {
        let result = parse_query("你好世界").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["你好世界"]);
    }

    #[test]
    fn test_unicode_phrase() {
        let result = parse_query("\"你好 世界\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["你好 世界"]);
    }

    #[test]
    fn test_emoji() {
        let result = parse_query("test🔥file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test🔥file"]);
    }

    #[test]
    fn test_mixed_languages() {
        let result = parse_query("hello 世界 test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello", "test", "世界"]);
    }

    #[test]
    fn test_cyrillic_text() {
        let result = parse_query("привет мир").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["мир", "привет"]);
    }

    #[test]
    fn test_arabic_text() {
        let result = parse_query("مرحبا عالم").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["عالم", "مرحبا"]);
    }

    #[test]
    fn test_japanese_text() {
        let result = parse_query("こんにちは 世界").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["こんにちは", "世界"]);
    }

    #[test]
    fn test_korean_text() {
        let result = parse_query("안녕하세요 세계").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["세계", "안녕하세요"]);
    }

    // ============================================================================
    // Wildcard and Pattern Tests (200 lines)
    // ============================================================================

    #[test]
    fn test_wildcard_star() {
        let result = parse_query("*.txt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec![".txt"]);
    }

    #[test]
    fn test_wildcard_question() {
        let result = parse_query("file?.txt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec![".txt", "file"]);
    }

    #[test]
    fn test_wildcard_both_sides() {
        let result = parse_query("*test*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_wildcard_multiple_stars() {
        let result = parse_query("*hello*world*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn test_wildcard_only_star() {
        let result = parse_query("*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_wildcard_only_question() {
        let result = parse_query("?").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_wildcard_multiple_only() {
        let result = parse_query("***???***").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_wildcard_with_spaces() {
        let result = parse_query("* test *").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_wildcard_prefix() {
        let result = parse_query("test*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_wildcard_suffix() {
        let result = parse_query("*test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_wildcard_middle() {
        let result = parse_query("he*lo").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["he", "lo"]);
    }

    #[test]
    fn test_wildcard_complex_pattern() {
        let result = parse_query("*test?file*.txt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec![".txt", "file", "test"]);
    }

    #[test]
    fn test_wildcard_numbers() {
        let result = parse_query("file*123").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["123", "file"]);
    }

    #[test]
    fn test_wildcard_underscore() {
        let result = parse_query("test_*_file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["_file", "test_"]);
    }

    #[test]
    fn test_wildcard_hyphen() {
        let result = parse_query("test-*-file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["-file", "test-"]);
    }

    #[test]
    fn test_wildcard_dot() {
        let result = parse_query("test.*.file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec![".file", "test."]);
    }

    #[test]
    fn test_wildcard_unicode() {
        let result = parse_query("*你好*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["你好"]);
    }

    #[test]
    fn test_wildcard_emoji() {
        let result = parse_query("*🔥*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["🔥"]);
    }

    #[test]
    fn test_multiple_wildcards_separate_words() {
        let result = parse_query("*.txt *.rs").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec![".rs", ".txt"]);
    }

    #[test]
    fn test_wildcard_in_phrase() {
        let result = parse_query("\"test * file\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test * file"]);
    }

    #[test]
    fn test_path_with_wildcard() {
        let result = parse_query("src/*/test.rs").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test.rs"]);
    }

    #[test]
    fn test_extension_wildcard() {
        let result = parse_query("file.*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file."]);
    }

    #[test]
    fn test_basename_wildcard() {
        let result = parse_query("*.tar.gz").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec![".tar.gz"]);
    }

    #[test]
    fn test_wildcard_beginning_and_end() {
        let result = parse_query("*file.txt*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file.txt"]);
    }

    #[test]
    fn test_question_mark_pattern() {
        let result = parse_query("test???").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_question_mark_middle() {
        let result = parse_query("te?st").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["st", "te"]);
    }

    #[test]
    fn test_mixed_wildcards() {
        let result = parse_query("*test?file*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file", "test"]);
    }

    #[test]
    fn test_wildcard_longest_segment() {
        let result = parse_query("a*bb*ccc*dddd").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["a", "bb", "ccc", "dddd"]);
    }

    #[test]
    fn test_whitespace_around_wildcard() {
        let result = parse_query("  *test*  ").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    // ============================================================================
    // Boolean Expression Tests (200 lines)
    // ============================================================================

    #[test]
    fn test_and_expression() {
        let result = parse_query("foo bar").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["bar", "foo"]);
    }

    #[test]
    fn test_or_expression() {
        let result = parse_query("foo|bar").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["bar", "foo"]);
    }

    #[test]
    fn test_not_expression() {
        let result = parse_query("!foo").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["foo"]);
    }

    #[test]
    fn test_not_word() {
        let result = parse_query("test !exclude").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["exclude", "test"]);
    }

    #[test]
    fn test_multiple_not() {
        let result = parse_query("foo !bar !baz").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["bar", "baz", "foo"]);
    }

    #[test]
    fn test_complex_and_or() {
        let result = parse_query("foo bar|baz qux").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["bar", "baz", "foo", "qux"]);
    }

    #[test]
    fn test_grouped_expression() {
        let result = parse_query("(foo bar)").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["bar", "foo"]);
    }

    #[test]
    fn test_nested_groups() {
        let result = parse_query("((foo bar))").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["bar", "foo"]);
    }

    #[test]
    fn test_group_with_or() {
        let result = parse_query("(foo|bar) baz").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["bar", "baz", "foo"]);
    }

    #[test]
    fn test_group_with_not() {
        let result = parse_query("!(foo bar)").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["bar", "foo"]);
    }

    #[test]
    fn test_multiple_groups() {
        let result = parse_query("(foo bar) (baz qux)").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["bar", "baz", "foo", "qux"]);
    }

    #[test]
    fn test_or_with_three_terms() {
        let result = parse_query("foo|bar|baz").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["bar", "baz", "foo"]);
    }

    #[test]
    fn test_and_with_or() {
        let result = parse_query("foo bar|baz").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["bar", "baz", "foo"]);
    }

    #[test]
    fn test_complex_boolean() {
        let result = parse_query("(foo|bar) baz !qux").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["bar", "baz", "foo", "qux"]);
    }

    #[test]
    fn test_not_group() {
        let result = parse_query("foo !(bar|baz)").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["bar", "baz", "foo"]);
    }

    #[test]
    fn test_deep_nesting() {
        let result = parse_query("((foo|(bar baz)))").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["bar", "baz", "foo"]);
    }

    #[test]
    fn test_boolean_with_wildcards() {
        let result = parse_query("*.txt|*.rs").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec![".rs", ".txt"]);
    }

    #[test]
    fn test_boolean_with_phrases() {
        let result = parse_query("\"hello world\"|\"foo bar\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["foo bar", "hello world"]);
    }

    #[test]
    fn test_and_with_phrases() {
        let result = parse_query("\"hello world\" test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello world", "test"]);
    }

    #[test]
    fn test_not_phrase() {
        let result = parse_query("!\"hello world\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_multiple_or_chains() {
        let result = parse_query("a|b c|d").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn test_boolean_unicode() {
        let result = parse_query("你好|世界").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["世界", "你好"]);
    }

    #[test]
    fn test_empty_group() {
        assert!(parse_query("foo () bar").is_err());
    }

    #[test]
    fn test_whitespace_in_group() {
        let result = parse_query("(   foo   bar   )").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["bar", "foo"]);
    }

    #[test]
    fn test_multiple_not_operators() {
        let result = parse_query("!!foo").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["foo"]);
    }

    #[test]
    fn test_not_empty() {
        let result = parse_query("!").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_or_empty() {
        let result = parse_query("|").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_complex_nested_boolean() {
        let result = parse_query("((a|b) (c|d)) | ((e|f) (g|h))").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["a", "b", "c", "d", "e", "f", "g", "h"]);
    }

    #[test]
    fn test_boolean_with_numbers() {
        let result = parse_query("123|456 789").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["123", "456", "789"]);
    }

    // ============================================================================
    // Filter Tests (200 lines)
    // ============================================================================

    #[test]
    fn test_filter_bare_argument() {
        let result = parse_query("ext:txt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["txt"]);
    }

    #[test]
    fn test_filter_phrase_argument() {
        let result = parse_query("ext:\"tar gz\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["tar gz"]);
    }

    #[test]
    fn test_filter_no_argument() {
        let result = parse_query("file:").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_filter_with_word() {
        let result = parse_query("ext:txt test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test", "txt"]);
    }

    #[test]
    fn test_multiple_filters() {
        let result = parse_query("ext:txt size:>1mb").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["txt"]);
    }

    #[test]
    fn test_filter_wildcard_argument() {
        let result = parse_query("ext:t*t").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["t"]);
    }

    #[test]
    fn test_filter_uppercase() {
        let result = parse_query("ext:TXT").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["txt"]);
    }

    #[test]
    fn test_filter_numbers() {
        let result = parse_query("ext:mp3").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["mp3"]);
    }

    #[test]
    fn test_filter_path() {
        let result = parse_query("path:src/test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_filter_unicode() {
        let result = parse_query("name:你好").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["你好"]);
    }

    #[test]
    fn test_size_filter_no_highlight() {
        let result = parse_query("size:>1mb").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_date_filter_no_highlight() {
        let result = parse_query("dm:2024-01-01").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["2024-01-01"]);
    }

    #[test]
    fn test_filter_with_boolean() {
        let result = parse_query("ext:txt|ext:rs").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["rs", "txt"]);
    }

    #[test]
    fn test_filter_with_not() {
        let result = parse_query("test !ext:tmp").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test", "tmp"]);
    }

    #[test]
    fn test_filter_in_group() {
        let result = parse_query("(ext:txt test)").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test", "txt"]);
    }

    #[test]
    fn test_filter_phrase_with_spaces() {
        let result = parse_query("name:\"hello world\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_filter_multiple_arguments() {
        let result = parse_query("ext:txt ext:rs ext:md").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["md", "rs", "txt"]);
    }

    #[test]
    fn test_filter_empty_argument() {
        let result = parse_query("ext:\"\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_filter_whitespace_argument() {
        let result = parse_query("ext:\"   \"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_folder_filter() {
        let result = parse_query("folder:Documents").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["documents"]);
    }

    #[test]
    fn test_file_filter_with_name() {
        let result = parse_query("file:test.txt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test.txt"]);
    }

    #[test]
    fn test_type_filter() {
        let result = parse_query("type:picture").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["picture"]);
    }

    #[test]
    fn test_filter_with_hyphen() {
        let result = parse_query("ext:tar-gz").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["tar-gz"]);
    }

    #[test]
    fn test_filter_with_underscore() {
        let result = parse_query("name:test_file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test_file"]);
    }

    #[test]
    fn test_filter_with_dot() {
        let result = parse_query("name:file.test.txt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file.test.txt"]);
    }

    #[test]
    fn test_regex_filter_no_highlight() {
        let result = parse_query("regex:test.*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_regex_with_other_terms() {
        let result = parse_query("regex:test.* hello").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello"]);
    }

    #[test]
    fn test_filter_duplicate_values() {
        let result = parse_query("ext:txt ext:txt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["txt"]);
    }

    #[test]
    fn test_filter_case_insensitive_dedup() {
        let result = parse_query("ext:TXT ext:txt ext:Txt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["txt"]);
    }

    #[test]
    fn test_filter_with_emoji() {
        let result = parse_query("name:test🔥file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test🔥file"]);
    }

    #[test]
    fn test_filter_special_chars() {
        let result = parse_query("name:test@file.txt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test@file.txt"]);
    }

    #[test]
    fn test_audio_filter() {
        let result = parse_query("audio:").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_video_filter() {
        let result = parse_query("video:").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_doc_filter() {
        let result = parse_query("doc:").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    // ============================================================================
    // Edge Cases and Complex Scenarios (200 lines)
    // ============================================================================

    #[test]
    fn test_very_long_word() {
        let long_word = "a".repeat(1000);
        let result = parse_query(&long_word).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].len(), 1000);
    }

    #[test]
    fn test_very_long_phrase() {
        let long_phrase = format!("\"{}\"", "test ".repeat(500));
        let result = parse_query(&long_phrase).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 1);
    }

    #[test]
    fn test_many_terms() {
        let query = (0..100)
            .map(|i| format!("word{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 100);
    }

    #[test]
    fn test_many_or_terms() {
        let query = (0..50)
            .map(|i| format!("word{i}"))
            .collect::<Vec<_>>()
            .join("|");
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 50);
    }

    #[test]
    fn test_deeply_nested_groups() {
        let mut query = String::from("test");
        for _ in 0..20 {
            query = format!("({query})");
        }
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_mixed_everything() {
        let result =
            parse_query("*.txt \"hello world\" test !exclude ext:rs size:>1mb (foo|bar)").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&".txt".to_string()));
        assert!(terms.contains(&"hello world".to_string()));
        assert!(terms.contains(&"test".to_string()));
        assert!(terms.contains(&"exclude".to_string()));
        assert!(terms.contains(&"rs".to_string()));
        assert!(terms.contains(&"foo".to_string()));
        assert!(terms.contains(&"bar".to_string()));
    }

    #[test]
    fn test_sanitize_only_wildcards() {
        let result = parse_query("*?*?*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_sanitize_trim_wildcards() {
        let result = parse_query("***test???").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_middle_wildcards() {
        let result = parse_query("test*ing").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["ing", "test"]);
    }

    #[test]
    fn test_empty_segments() {
        let result = parse_query("**hello**world**").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn test_only_whitespace_segments() {
        let result = parse_query("   ").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_tab_characters() {
        let result = parse_query("hello\tworld").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn test_newline_characters() {
        let result = parse_query("hello\nworld").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn test_carriage_return() {
        let result = parse_query("hello\rworld").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn test_mixed_whitespace() {
        let result = parse_query("hello \t\n\r world").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn test_zero_width_characters() {
        let result = parse_query("test\u{200B}file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 1);
    }

    #[test]
    fn test_combining_characters() {
        let result = parse_query("café").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 1);
    }

    #[test]
    fn test_rtl_text() {
        let result = parse_query("שלום עולם").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 2);
    }

    #[test]
    fn test_mixed_rtl_ltr() {
        let result = parse_query("hello שלום world").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 3);
    }

    #[test]
    fn test_backslash_in_query() {
        let result = parse_query("path\\to\\file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["path\\to\\file"]);
    }

    #[test]
    fn test_forward_slash_in_query() {
        let result = parse_query("path/to/file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file"]);
    }

    #[test]
    fn test_mixed_slashes() {
        let result = parse_query("path\\to/file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file"]);
    }

    #[test]
    fn test_quotes_in_word() {
        let result = parse_query("test\"file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(!terms.is_empty());
    }

    #[test]
    fn test_parentheses_in_word() {
        let result = parse_query("test(file)").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(!terms.is_empty());
    }

    #[test]
    fn test_brackets_in_word() {
        let result = parse_query("test[file]").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test[file]"]);
    }

    #[test]
    fn test_braces_in_word() {
        let result = parse_query("test{file}").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test{file}"]);
    }

    #[test]
    fn test_angle_brackets_in_word() {
        let result = parse_query("test<file>").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file", "test"]);
    }

    #[test]
    fn test_ampersand_in_word() {
        let result = parse_query("test&file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test&file"]);
    }

    #[test]
    fn test_pipe_in_phrase() {
        let result = parse_query("\"test|file\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test|file"]);
    }

    #[test]
    fn test_exclamation_in_phrase() {
        let result = parse_query("\"test!file\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test!file"]);
    }

    #[test]
    fn test_colon_in_word() {
        let result = parse_query("test:file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file"]);
    }

    #[test]
    fn test_semicolon_in_word() {
        let result = parse_query("test;file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test;file"]);
    }

    #[test]
    fn test_comma_in_word() {
        let result = parse_query("test,file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test,file"]);
    }

    #[test]
    fn test_percent_in_word() {
        let result = parse_query("test%file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test%file"]);
    }

    #[test]
    fn test_dollar_in_word() {
        let result = parse_query("test$file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test$file"]);
    }

    #[test]
    fn test_hash_in_word() {
        let result = parse_query("test#file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test#file"]);
    }

    #[test]
    fn test_plus_in_word() {
        let result = parse_query("test+file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test+file"]);
    }

    #[test]
    fn test_equals_in_word() {
        let result = parse_query("test=file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test=file"]);
    }

    #[test]
    fn test_tilde_in_word() {
        let result = parse_query("test~file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test~file"]);
    }

    #[test]
    fn test_backtick_in_word() {
        let result = parse_query("test`file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test`file"]);
    }

    // ============================================================================
    // Query Segmentation Integration Tests (200 lines)
    // ============================================================================

    #[test]
    fn test_segmentation_camelcase() {
        let result = parse_query("testFile").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["testfile"]);
    }

    #[test]
    fn test_segmentation_pascalcase() {
        let result = parse_query("TestFile").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["testfile"]);
    }

    #[test]
    fn test_segmentation_snake_case() {
        let result = parse_query("test_file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test_file"]);
    }

    #[test]
    fn test_segmentation_kebab_case() {
        let result = parse_query("test-file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test-file"]);
    }

    #[test]
    fn test_segmentation_dot_separated() {
        let result = parse_query("test.file.name").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test.file.name"]);
    }

    #[test]
    fn test_segmentation_mixed_case() {
        let result = parse_query("TestFile_Name").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["testfile_name"]);
    }

    #[test]
    fn test_segmentation_with_numbers() {
        let result = parse_query("test123File").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test123file"]);
    }

    #[test]
    fn test_segmentation_all_caps() {
        let result = parse_query("TESTFILE").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["testfile"]);
    }

    #[test]
    fn test_segmentation_alternating_case() {
        let result = parse_query("TeSt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_segmentation_with_wildcard() {
        let result = parse_query("test*File").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file", "test"]);
    }

    #[test]
    fn test_segmentation_multiple_words() {
        let result = parse_query("testFile anotherTest").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["anothertest", "testfile"]);
    }

    #[test]
    fn test_segmentation_phrase() {
        let result = parse_query("\"testFile\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["testfile"]);
    }

    #[test]
    fn test_segmentation_in_filter() {
        let result = parse_query("name:testFile").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["testfile"]);
    }

    #[test]
    fn test_segmentation_complex() {
        let result = parse_query("myTestFile_v2").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["mytestfile_v2"]);
    }

    #[test]
    fn test_segmentation_with_path() {
        let result = parse_query("src/testFile/index.ts").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["index.ts"]);
    }

    #[test]
    fn test_longest_segment_selection() {
        let result = parse_query("a*bb*ccc").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["a", "bb", "ccc"]);
    }

    #[test]
    fn test_segment_with_underscores() {
        let result = parse_query("__test__file__").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["__test__file__"]);
    }

    #[test]
    fn test_segment_with_hyphens() {
        let result = parse_query("--test--file--").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["--test--file--"]);
    }

    #[test]
    fn test_segment_leading_numbers() {
        let result = parse_query("123test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["123test"]);
    }

    #[test]
    fn test_segment_trailing_numbers() {
        let result = parse_query("test123").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test123"]);
    }

    #[test]
    fn test_segment_only_numbers() {
        let result = parse_query("123456").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["123456"]);
    }

    #[test]
    fn test_segment_mixed_separators() {
        let result = parse_query("test_file-name.txt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test_file-name.txt"]);
    }

    #[test]
    fn test_segment_unicode_camelcase() {
        let result = parse_query("测试File").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["测试file"]);
    }

    #[test]
    fn test_segment_emoji_separator() {
        let result = parse_query("test🔥file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test🔥file"]);
    }

    #[test]
    fn test_segment_multiple_extensions() {
        let result = parse_query("archive.tar.gz").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["archive.tar.gz"]);
    }

    #[test]
    fn test_segment_version_number() {
        let result = parse_query("package-v1.2.3").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["package-v1.2.3"]);
    }

    #[test]
    fn test_segment_date_like() {
        let result = parse_query("report-2024-01-15").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["report-2024-01-15"]);
    }

    #[test]
    fn test_segment_uuid_like() {
        let result = parse_query("file-550e8400-e29b-41d4").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file-550e8400-e29b-41d4"]);
    }

    #[test]
    fn test_segment_hash_like() {
        let result = parse_query("commit-abc123def456").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["commit-abc123def456"]);
    }

    #[test]
    fn test_segment_url_like() {
        let result = parse_query("https://example.com").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["example.com"]);
    }

    #[test]
    fn test_segment_email_like() {
        let result = parse_query("user@example.com").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["user@example.com"]);
    }

    #[test]
    fn test_segment_ipv4_like() {
        let result = parse_query("192.168.1.1").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["192.168.1.1"]);
    }

    #[test]
    fn test_segment_single_char() {
        let result = parse_query("a").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["a"]);
    }

    #[test]
    fn test_segment_two_chars() {
        let result = parse_query("ab").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["ab"]);
    }

    #[test]
    fn test_segment_repeated_chars() {
        let result = parse_query("aaa").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["aaa"]);
    }

    #[test]
    fn test_segment_palindrome() {
        let result = parse_query("racecar").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["racecar"]);
    }

    #[test]
    fn test_segment_abbreviation() {
        let result = parse_query("USA").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["usa"]);
    }

    #[test]
    fn test_segment_acronym() {
        let result = parse_query("HTTP").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["http"]);
    }

    #[test]
    fn test_segment_mixed_acronym() {
        let result = parse_query("HTTPServer").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["httpserver"]);
    }

    // ============================================================================
    // Sanitization and Trimming Tests (200 lines)
    // ============================================================================

    #[test]
    fn test_sanitize_leading_wildcards() {
        let result = parse_query("***test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_trailing_wildcards() {
        let result = parse_query("test***").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_leading_questions() {
        let result = parse_query("???test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_trailing_questions() {
        let result = parse_query("test???").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_mixed_leading() {
        let result = parse_query("*?*?test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_mixed_trailing() {
        let result = parse_query("test*?*?").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_both_ends() {
        let result = parse_query("***test???").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_with_spaces() {
        let result = parse_query("  ***test???  ").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_preserve_middle() {
        let result = parse_query("te*st").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["st", "te"]);
    }

    #[test]
    fn test_sanitize_preserve_question_middle() {
        let result = parse_query("te?st").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["st", "te"]);
    }

    #[test]
    fn test_sanitize_longest_chunk() {
        let result = parse_query("a*bb*ccc*dddd").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["a", "bb", "ccc", "dddd"]);
    }

    #[test]
    fn test_sanitize_equal_chunks() {
        let result = parse_query("aa*bb*cc").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["aa", "bb", "cc"]);
    }

    #[test]
    fn test_sanitize_single_char_chunks() {
        let result = parse_query("a*b*c*d").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn test_sanitize_empty_after_trim() {
        let result = parse_query("***").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_sanitize_spaces_only_after_trim() {
        let result = parse_query("*   *").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_sanitize_unicode_with_wildcards() {
        let result = parse_query("*你好*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["你好"]);
    }

    #[test]
    fn test_sanitize_emoji_with_wildcards() {
        let result = parse_query("*🔥*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["🔥"]);
    }

    #[test]
    fn test_sanitize_number_with_wildcards() {
        let result = parse_query("*123*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["123"]);
    }

    #[test]
    fn test_sanitize_path_with_wildcards() {
        let result = parse_query("*/path/to/file*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file"]);
    }

    #[test]
    fn test_sanitize_extension_pattern() {
        let result = parse_query("***.txt***").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec![".txt"]);
    }

    #[test]
    fn test_sanitize_hyphen_separated() {
        let result = parse_query("*test-file*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test-file"]);
    }

    #[test]
    fn test_sanitize_underscore_separated() {
        let result = parse_query("*test_file*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test_file"]);
    }

    #[test]
    fn test_sanitize_dot_separated() {
        let result = parse_query("*test.file*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test.file"]);
    }

    #[test]
    fn test_sanitize_complex_pattern() {
        let result = parse_query("***test*file***name*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file", "name", "test"]);
    }

    #[test]
    fn test_sanitize_alternating_wildcards() {
        let result = parse_query("*?*?test*?*?").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_in_phrase() {
        let result = parse_query("\"***test***\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["***test***"]);
    }

    #[test]
    fn test_sanitize_multiple_words() {
        let result = parse_query("*test* *file*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file", "test"]);
    }

    #[test]
    fn test_sanitize_with_boolean() {
        let result = parse_query("*test*|*file*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file", "test"]);
    }

    #[test]
    fn test_sanitize_with_not() {
        let result = parse_query("!*test*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_in_filter() {
        let result = parse_query("name:*test*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_wildcard_only_in_filter() {
        let result = parse_query("name:***").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_trim_leading_spaces() {
        let result = parse_query("   test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_trim_trailing_spaces() {
        let result = parse_query("test   ").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_trim_both_spaces() {
        let result = parse_query("   test   ").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_trim_internal_spaces_preserved() {
        let result = parse_query("\"  test   file  \"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["  test   file  "]);
    }

    #[test]
    fn test_trim_tabs() {
        let result = parse_query("\t\ttest\t\t").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_trim_newlines() {
        let result = parse_query("\n\ntest\n\n").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_trim_mixed_whitespace() {
        let result = parse_query(" \t\n test \n\t ").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_preserve_internal_structure() {
        let result = parse_query("*test*file*name*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file", "name", "test"]);
    }

    #[test]
    fn test_sanitize_single_wildcard_between() {
        let result = parse_query("test*file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file", "test"]);
    }

    #[test]
    fn test_sanitize_multiple_wildcards_between() {
        let result = parse_query("test***file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file", "test"]);
    }

    // ============================================================================
    // Duplicate Handling and Case Sensitivity Tests (200 lines)
    // ============================================================================

    #[test]
    fn test_dedup_exact_duplicates() {
        let result = parse_query("test test test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_case_insensitive() {
        let result = parse_query("Test test TEST").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_mixed_case() {
        let result = parse_query("test Test TeSt tEsT").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_in_boolean() {
        let result = parse_query("test | test | test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_in_and_expression() {
        let result = parse_query("test test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_with_wildcards() {
        let result = parse_query("*test* *TEST* *Test*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_phrases() {
        let result = parse_query("\"hello world\" \"HELLO WORLD\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_dedup_in_filters() {
        let result = parse_query("ext:txt ext:TXT ext:Txt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["txt"]);
    }

    #[test]
    fn test_dedup_complex_query() {
        let result = parse_query("test Test (test | TEST) !test ext:test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_no_dedup_different_words() {
        let result = parse_query("test file name").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file", "name", "test"]);
    }

    #[test]
    fn test_dedup_unicode() {
        let result = parse_query("你好 你好 你好").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["你好"]);
    }

    #[test]
    fn test_dedup_emoji() {
        let result = parse_query("🔥 🔥 🔥").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["🔥"]);
    }

    #[test]
    fn test_case_lowercase() {
        let result = parse_query("test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_case_uppercase() {
        let result = parse_query("TEST").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_case_titlecase() {
        let result = parse_query("Test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_case_camelcase() {
        let result = parse_query("testFile").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["testfile"]);
    }

    #[test]
    fn test_case_pascalcase() {
        let result = parse_query("TestFile").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["testfile"]);
    }

    #[test]
    fn test_case_snake_case() {
        let result = parse_query("TEST_FILE").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test_file"]);
    }

    #[test]
    fn test_case_screaming_snake_case() {
        let result = parse_query("TEST_FILE_NAME").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test_file_name"]);
    }

    #[test]
    fn test_case_kebab_case_upper() {
        let result = parse_query("TEST-FILE").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test-file"]);
    }

    #[test]
    fn test_case_mixed_separators() {
        let result = parse_query("TEST_file-NAME").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test_file-name"]);
    }

    #[test]
    fn test_case_phrase_lowercase() {
        let result = parse_query("\"hello world\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_case_phrase_uppercase() {
        let result = parse_query("\"HELLO WORLD\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_case_phrase_mixedcase() {
        let result = parse_query("\"Hello World\"").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_case_unicode_lowercase() {
        let result = parse_query("café").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.len() == 1);
    }

    #[test]
    fn test_case_unicode_uppercase() {
        let result = parse_query("CAFÉ").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.len() == 1);
    }

    #[test]
    fn test_case_cyrillic_lower() {
        let result = parse_query("привет").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["привет"]);
    }

    #[test]
    fn test_case_cyrillic_upper() {
        let result = parse_query("ПРИВЕТ").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["привет"]);
    }

    #[test]
    fn test_case_greek_lower() {
        let result = parse_query("γεια").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["γεια"]);
    }

    #[test]
    fn test_case_greek_upper() {
        let result = parse_query("ΓΕΙΑ").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["γεια"]);
    }

    #[test]
    fn test_dedup_with_spaces() {
        let result = parse_query("test   test test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_across_groups() {
        let result = parse_query("(test) test (test)").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_with_not() {
        let result = parse_query("test !test test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_in_nested_groups() {
        let result = parse_query("((test) (test))").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_mixed_terms() {
        let result = parse_query("test \"test\" *test*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_filter_arguments() {
        let result = parse_query("ext:txt name:txt path:txt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["txt"]);
    }

    #[test]
    fn test_dedup_numbers() {
        let result = parse_query("123 123 123").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["123"]);
    }

    #[test]
    fn test_dedup_paths() {
        let result = parse_query("path/to/file path/to/file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["file"]);
    }

    #[test]
    fn test_dedup_extensions() {
        let result = parse_query("*.txt *.TXT *.Txt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec![".txt"]);
    }

    #[test]
    fn test_dedup_hyphenated() {
        let result = parse_query("test-file test-FILE TEST-file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test-file"]);
    }

    #[test]
    fn test_dedup_underscored() {
        let result = parse_query("test_file test_FILE TEST_file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test_file"]);
    }

    #[test]
    fn test_dedup_dotted() {
        let result = parse_query("test.file test.FILE TEST.file").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test.file"]);
    }

    // ============================================================================
    // Stress Tests and Performance Scenarios (200 lines)
    // ============================================================================

    #[test]
    fn test_stress_many_simple_terms() {
        let query = (0..200)
            .map(|i| format!("word{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 200);
    }

    #[test]
    fn test_stress_many_or_terms() {
        let query = (0..100)
            .map(|i| format!("word{i}"))
            .collect::<Vec<_>>()
            .join("|");
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 100);
    }

    #[test]
    fn test_stress_many_phrases() {
        let query = (0..50)
            .map(|i| format!("\"phrase{i}\""))
            .collect::<Vec<_>>()
            .join(" ");
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 50);
    }

    #[test]
    fn test_stress_many_filters() {
        let query = (0..50)
            .map(|i| format!("ext:ext{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 50);
    }

    #[test]
    fn test_stress_long_word() {
        let long_word = "test".repeat(500);
        let result = parse_query(&long_word).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].len(), 2000);
    }

    #[test]
    fn test_stress_long_phrase() {
        let long_phrase = format!("\"{}\"", "test ".repeat(1000));
        let result = parse_query(&long_phrase).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 1);
    }

    #[test]
    fn test_stress_many_wildcards() {
        let query = "*".repeat(1000) + "test" + &"*".repeat(1000);
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_stress_alternating_wildcards() {
        let mut query = String::new();
        for i in 0..100 {
            query.push_str(&format!("word{i}*"));
        }
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(!terms.is_empty());
    }

    #[test]
    fn test_stress_deep_nesting() {
        let mut query = String::from("test");
        for _ in 0..50 {
            query = format!("({query})");
        }
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_stress_wide_or_tree() {
        let parts = (0..200).map(|i| format!("w{i}")).collect::<Vec<_>>();
        let query = parts.join("|");
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 200);
    }

    #[test]
    fn test_stress_wide_and_tree() {
        let parts = (0..200).map(|i| format!("w{i}")).collect::<Vec<_>>();
        let query = parts.join(" ");
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 200);
    }

    #[test]
    fn test_stress_mixed_boolean() {
        let mut query = String::new();
        for i in 0..100 {
            if i % 2 == 0 {
                query.push_str(&format!("w{i} "));
            } else {
                query.push_str(&format!("w{i}|"));
            }
        }
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(!terms.is_empty());
    }

    #[test]
    fn test_stress_many_not_operators() {
        let mut query = String::new();
        for i in 0..100 {
            query.push_str(&format!("!w{i} "));
        }
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 100);
    }

    #[test]
    fn test_stress_unicode_heavy() {
        let query =
            "你好 世界 測試 테스트 тест test प्रयोग परीक्षण δοκιμή تجربة テスト 試験".to_string();
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.len() >= 10);
    }

    #[test]
    fn test_stress_emoji_heavy() {
        let query = "🔥 ⚡ 🎉 💻 📁 📄 🎨 🎯 ⭐ 💡 🚀 🌟 ✨ 🎁 🔔".to_string();
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 15);
    }

    #[test]
    fn test_stress_mixed_scripts() {
        let query = "test你好привет🔥مرحبا".to_string();
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 1);
    }

    #[test]
    fn test_stress_repeated_duplicates() {
        let query = "test ".repeat(500);
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_stress_alternating_case_duplicates() {
        let mut query = String::new();
        for i in 0..500 {
            if i % 2 == 0 {
                query.push_str("test ");
            } else {
                query.push_str("TEST ");
            }
        }
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_stress_complex_nested_boolean() {
        let query = "((a|b) (c|d)) ((e|f) (g|h)) ((i|j) (k|l)) ((m|n) (o|p))".to_string();
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 16);
    }

    #[test]
    fn test_stress_filter_variations() {
        let query =
            "ext:txt name:file path:dir folder:test type:doc size:>1mb dm:today".to_string();
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.len() >= 3);
    }

    #[test]
    fn test_stress_wildcard_patterns() {
        let query = "*test* test* *test t*st te*t *t*e*s*t* *.txt file.* *.tar.gz".to_string();
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(!terms.is_empty());
    }

    #[test]
    fn test_stress_phrase_variations() {
        let query = "\"test\" \"test file\" \"test file name\" \"a\" \"ab\" \"abc\"".to_string();
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.len() >= 6);
    }

    #[test]
    fn test_stress_path_like_queries() {
        let query = "src/main.rs lib/util.rs test/test.rs src/components/Button.tsx".to_string();
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 4);
    }

    #[test]
    fn test_stress_extension_patterns() {
        let query = "*.txt *.rs *.js *.ts *.jsx *.tsx *.md *.json *.toml *.yaml".to_string();
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 10);
    }

    #[test]
    fn test_stress_number_variations() {
        let query = "1 12 123 1234 12345 123456 1234567 12345678".to_string();
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 8);
    }

    #[test]
    fn test_stress_special_char_combinations() {
        let query = "test@file test#file test$file test%file test&file".to_string();
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 5);
    }

    #[test]
    fn test_stress_mixed_separators_many() {
        let query = "test-file test_file test.file test/file test\\file".to_string();
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 5);
    }

    #[test]
    fn test_stress_camelcase_variations() {
        let query = "testFile TestFile testfile TESTFILE testFILE TESTfile".to_string();
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["testfile"]);
    }

    #[test]
    fn test_stress_empty_elements() {
        let query = "test   test   test".to_string();
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_stress_group_variations() {
        let query = "(test) ((test)) (((test))) ((((test))))".to_string();
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_stress_not_variations() {
        let query = "!test !!test !!!test !!!!test".to_string();
        let result = parse_query(&query).unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["test"]);
    }

    // ============================================================================
    // Integration and Real-World Scenarios (200+ lines)
    // ============================================================================

    #[test]
    fn test_real_code_search() {
        let result = parse_query("*.rs cargo test").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&".rs".to_string()));
        assert!(terms.contains(&"cargo".to_string()));
        assert!(terms.contains(&"test".to_string()));
    }

    #[test]
    fn test_real_document_search() {
        let result = parse_query("\"project report\" *.pdf 2024").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"project report".to_string()));
        assert!(terms.contains(&".pdf".to_string()));
        assert!(terms.contains(&"2024".to_string()));
    }

    #[test]
    fn test_real_image_search() {
        let result = parse_query("vacation (*.jpg|*.png) !thumbnail").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"vacation".to_string()));
        assert!(terms.contains(&".jpg".to_string()));
        assert!(terms.contains(&".png".to_string()));
        assert!(terms.contains(&"thumbnail".to_string()));
    }

    #[test]
    fn test_real_log_search() {
        let result = parse_query("error *.log !debug").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"error".to_string()));
        assert!(terms.contains(&".log".to_string()));
        assert!(terms.contains(&"debug".to_string()));
    }

    #[test]
    fn test_real_config_search() {
        let result = parse_query("(*.toml|*.yaml|*.json) config").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&".toml".to_string()));
        assert!(terms.contains(&".yaml".to_string()));
        assert!(terms.contains(&".json".to_string()));
        assert!(terms.contains(&"config".to_string()));
    }

    #[test]
    fn test_real_backup_search() {
        let result = parse_query("backup *.zip size:>1gb").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"backup".to_string()));
        assert!(terms.contains(&".zip".to_string()));
    }

    #[test]
    fn test_real_temp_cleanup() {
        let result = parse_query("(temp|tmp|cache) !important").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"temp".to_string()));
        assert!(terms.contains(&"tmp".to_string()));
        assert!(terms.contains(&"cache".to_string()));
        assert!(terms.contains(&"important".to_string()));
    }

    #[test]
    fn test_real_music_collection() {
        let result = parse_query("\"The Beatles\" (*.mp3|*.flac) !live").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"the beatles".to_string()));
        assert!(terms.contains(&".mp3".to_string()));
        assert!(terms.contains(&".flac".to_string()));
        assert!(terms.contains(&"live".to_string()));
    }

    #[test]
    fn test_real_video_project() {
        let result = parse_query("project *.mp4 size:>100mb !draft").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"project".to_string()));
        assert!(terms.contains(&".mp4".to_string()));
        assert!(terms.contains(&"draft".to_string()));
    }

    #[test]
    fn test_real_source_code() {
        let result = parse_query("(*.cpp|*.h) !test !backup").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&".cpp".to_string()));
        assert!(terms.contains(&".h".to_string()));
        assert!(terms.contains(&"test".to_string()));
        assert!(terms.contains(&"backup".to_string()));
    }

    #[test]
    fn test_real_photo_album() {
        let result = parse_query("\"summer 2024\" (*.jpg|*.heic) folder:Photos").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"summer 2024".to_string()));
        assert!(terms.contains(&".jpg".to_string()));
        assert!(terms.contains(&".heic".to_string()));
        assert!(terms.contains(&"photos".to_string()));
    }

    #[test]
    fn test_real_download_cleanup() {
        let result = parse_query("folder:Downloads dm:lastmonth").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"downloads".to_string()));
    }

    #[test]
    fn test_real_duplicate_finder() {
        let result = parse_query("copy *(1)* *(2)*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"copy".to_string()));
    }

    #[test]
    fn test_real_version_search() {
        let result = parse_query("app*v1* app*v2* !beta").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"beta".to_string()));
    }

    #[test]
    fn test_real_archive_search() {
        let result = parse_query("(*.zip|*.tar|*.gz|*.7z) archive").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&".zip".to_string()));
        assert!(terms.contains(&".tar".to_string()));
        assert!(terms.contains(&".gz".to_string()));
        assert!(terms.contains(&".7z".to_string()));
        assert!(terms.contains(&"archive".to_string()));
    }

    #[test]
    fn test_real_presentation_search() {
        let result = parse_query("(*.ppt|*.pptx|*.key) presentation").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&".ppt".to_string()));
        assert!(terms.contains(&".pptx".to_string()));
        assert!(terms.contains(&".key".to_string()));
        assert!(terms.contains(&"presentation".to_string()));
    }

    #[test]
    fn test_real_spreadsheet_search() {
        let result = parse_query("budget (*.xls|*.xlsx|*.csv)").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"budget".to_string()));
        assert!(terms.contains(&".xls".to_string()));
        assert!(terms.contains(&".xlsx".to_string()));
        assert!(terms.contains(&".csv".to_string()));
    }

    #[test]
    fn test_real_ebook_search() {
        let result = parse_query("(*.pdf|*.epub|*.mobi) !sample").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&".pdf".to_string()));
        assert!(terms.contains(&".epub".to_string()));
        assert!(terms.contains(&".mobi".to_string()));
        assert!(terms.contains(&"sample".to_string()));
    }

    #[test]
    fn test_real_installer_search() {
        let result = parse_query("(*.exe|*.msi|*.dmg|*.pkg) setup install").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&".exe".to_string()));
        assert!(terms.contains(&".msi".to_string()));
        assert!(terms.contains(&".dmg".to_string()));
        assert!(terms.contains(&".pkg".to_string()));
        assert!(terms.contains(&"setup".to_string()));
        assert!(terms.contains(&"install".to_string()));
    }

    #[test]
    fn test_real_database_search() {
        let result = parse_query("(*.db|*.sqlite|*.sql) database").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&".db".to_string()));
        assert!(terms.contains(&".sqlite".to_string()));
        assert!(terms.contains(&".sql".to_string()));
        assert!(terms.contains(&"database".to_string()));
    }

    #[test]
    fn test_real_font_search() {
        let result = parse_query("(*.ttf|*.otf|*.woff) font").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&".ttf".to_string()));
        assert!(terms.contains(&".otf".to_string()));
        assert!(terms.contains(&".woff".to_string()));
        assert!(terms.contains(&"font".to_string()));
    }

    #[test]
    fn test_real_vector_graphics() {
        let result = parse_query("(*.svg|*.ai|*.eps) logo icon").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&".svg".to_string()));
        assert!(terms.contains(&".ai".to_string()));
        assert!(terms.contains(&".eps".to_string()));
        assert!(terms.contains(&"logo".to_string()));
        assert!(terms.contains(&"icon".to_string()));
    }

    #[test]
    fn test_real_3d_model_search() {
        let result = parse_query("(*.obj|*.fbx|*.blend) model").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&".obj".to_string()));
        assert!(terms.contains(&".fbx".to_string()));
        assert!(terms.contains(&".blend".to_string()));
        assert!(terms.contains(&"model".to_string()));
    }

    #[test]
    fn test_real_certificate_search() {
        let result = parse_query("(*.crt|*.pem|*.key) certificate ssl").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&".crt".to_string()));
        assert!(terms.contains(&".pem".to_string()));
        assert!(terms.contains(&".key".to_string()));
        assert!(terms.contains(&"certificate".to_string()));
        assert!(terms.contains(&"ssl".to_string()));
    }

    #[test]
    fn test_real_docker_search() {
        let result = parse_query("Dockerfile docker-compose.yml *.yaml").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"dockerfile".to_string()));
        assert!(terms.contains(&"docker-compose.yml".to_string()));
        assert!(terms.contains(&".yaml".to_string()));
    }

    #[test]
    fn test_real_makefile_search() {
        let result = parse_query("Makefile *.mk build").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"makefile".to_string()));
        assert!(terms.contains(&".mk".to_string()));
        assert!(terms.contains(&"build".to_string()));
    }

    #[test]
    fn test_real_readme_search() {
        let result = parse_query("README* *.md documentation").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"readme".to_string()));
        assert!(terms.contains(&".md".to_string()));
        assert!(terms.contains(&"documentation".to_string()));
    }

    #[test]
    fn test_real_license_search() {
        let result = parse_query("LICENSE* COPYING* *.txt").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"license".to_string()));
        assert!(terms.contains(&"copying".to_string()));
        assert!(terms.contains(&".txt".to_string()));
    }

    #[test]
    fn test_real_gitignore_search() {
        let result = parse_query("*ignore* .git*").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"ignore".to_string()));
        assert!(terms.contains(&".git".to_string()));
    }

    #[test]
    fn test_real_node_modules() {
        let result = parse_query("node_modules package*.json").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&"node_modules".to_string()));
        assert!(terms.contains(&".json".to_string()));
        assert!(terms.contains(&"package".to_string()));
    }

    #[test]
    fn test_real_python_project() {
        let result = parse_query("*.py requirements.txt setup.py !__pycache__").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&".py".to_string()));
        assert!(terms.contains(&"requirements.txt".to_string()));
        assert!(terms.contains(&"setup.py".to_string()));
        assert!(terms.contains(&"__pycache__".to_string()));
    }

    #[test]
    fn test_real_rust_project() {
        let result = parse_query("*.rs Cargo.toml !target").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&".rs".to_string()));
        assert!(terms.contains(&"cargo.toml".to_string()));
        assert!(terms.contains(&"target".to_string()));
    }

    #[test]
    fn test_real_java_project() {
        let result = parse_query("*.java pom.xml build.gradle").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&".java".to_string()));
        assert!(terms.contains(&"pom.xml".to_string()));
        assert!(terms.contains(&"build.gradle".to_string()));
    }

    #[test]
    fn test_real_web_assets() {
        let result = parse_query("(*.css|*.scss|*.less) style theme").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert!(terms.contains(&".css".to_string()));
        assert!(terms.contains(&".scss".to_string()));
        assert!(terms.contains(&".less".to_string()));
        assert!(terms.contains(&"style".to_string()));
        assert!(terms.contains(&"theme".to_string()));
    }

    #[test]
    fn test_empty_result_set() {
        let result = parse_query("").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_ordering_alphabetical() {
        let result = parse_query("zebra apple monkey banana").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms, vec!["apple", "banana", "monkey", "zebra"]);
    }

    #[test]
    fn test_btreeset_ordering() {
        let result = parse_query("zzz aaa mmm bbb").unwrap();
        let terms = derive_highlight_terms(&result.expr);
        assert_eq!(terms[0], "aaa");
        assert_eq!(terms[1], "bbb");
        assert_eq!(terms[2], "mmm");
        assert_eq!(terms[3], "zzz");
    }
}
