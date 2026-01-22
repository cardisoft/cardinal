use crate::query_preprocessor::strip_query_quotes_text;
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
            ArgumentKind::Bare => self.collect_text(argument.raw.as_str()),
            ArgumentKind::Phrase => self.collect_literal_text(argument.raw.as_str()),
            ArgumentKind::List(values) => {
                for value in values {
                    if value.contains('"') {
                        self.collect_literal_text(value);
                    } else {
                        self.collect_text(value);
                    }
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
                // Return last segment only(which is the filename part in a path)
                return;
            }
        }

        for candidate in literal_chunks(value) {
            self.push(candidate);
        }
    }

    fn collect_literal_text(&mut self, value: &str) {
        let unquoted = strip_query_quotes_text(value);
        let unquoted = unquoted.trim();
        if unquoted.is_empty() {
            return;
        }
        self.push(unquoted.to_string());
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
    use crate::query_preprocessor::{expand_query_home_dirs, strip_query_quotes};
    use cardinal_syntax::{ParseError, parse_query as raw_parse_query};

    /// Helper for highlight tests: mirrors production order
    /// (highlight is derived BEFORE quote stripping, preserving literal markers)
    fn parse_and_highlight(input: &str) -> Result<Vec<String>, ParseError> {
        raw_parse_query(input)
            .map(expand_query_home_dirs)
            .map(strip_query_quotes)
            .map(|expanded| {
                // Derive highlights while quotes are still present (literal marker)
                derive_highlight_terms(&expanded.expr)
            })
    }

    /// Helper for error-checking tests that just need to parse without highlighting
    fn parse_for_error_check(input: &str) -> Result<(), ParseError> {
        raw_parse_query(input).map(|_| ())
    }

    // ============================================================================
    // Basic Word and Phrase Tests (200 lines)
    // ============================================================================

    #[test]
    fn test_empty_query() {
        let terms = parse_and_highlight("").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_whitespace_only() {
        let terms = parse_and_highlight("   ").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_single_word() {
        let terms = parse_and_highlight("report").unwrap();
        assert_eq!(terms, vec!["report"]);
    }

    #[test]
    fn test_single_word_uppercase() {
        let terms = parse_and_highlight("REPORT").unwrap();
        assert_eq!(terms, vec!["report"]);
    }

    #[test]
    fn test_single_word_mixedcase() {
        let terms = parse_and_highlight("RePoRt").unwrap();
        assert_eq!(terms, vec!["report"]);
    }

    #[test]
    fn test_two_words() {
        let terms = parse_and_highlight("hello world").unwrap();
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn test_multiple_words() {
        let terms = parse_and_highlight("foo bar baz qux").unwrap();
        assert_eq!(terms, vec!["bar", "baz", "foo", "qux"]);
    }

    #[test]
    fn test_duplicate_words() {
        let terms = parse_and_highlight("test test test").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_duplicate_words_different_case() {
        let terms = parse_and_highlight("Test TEST test TeSt").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_phrase_single_word() {
        let terms = parse_and_highlight("\"hello\"").unwrap();
        assert_eq!(terms, vec!["hello"]);
    }

    #[test]
    fn test_phrase_with_escaped_quotes() {
        let terms = parse_and_highlight(r#"\"hello\""#).unwrap();
        assert_eq!(terms, vec![r#""hello""#]);
    }

    #[test]
    fn test_phrase_with_escaped_backslash() {
        let terms = parse_and_highlight(r#""C:\\path""#).unwrap();
        assert_eq!(terms, vec![r"c:\path"]);
    }

    #[test]
    fn test_word_with_escaped_quotes() {
        let terms = parse_and_highlight(r#"foo\"bar\"baz"#).unwrap();
        assert_eq!(terms, vec![r#"foo"bar"baz"#]);
    }

    #[test]
    fn test_word_with_escaped_quotes_and_backslashes() {
        let terms = parse_and_highlight(r#"\"C:\\path\""#).unwrap();
        assert_eq!(terms, vec![r#""c:\path""#]);
    }

    #[test]
    fn test_phrase_multiple_words() {
        let terms = parse_and_highlight("\"hello world\"").unwrap();
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_phrase_with_whitespace() {
        let terms = parse_and_highlight("\"  hello   world  \"").unwrap();
        assert_eq!(terms, vec!["hello   world"]);
    }

    #[test]
    fn test_phrase_uppercase() {
        let terms = parse_and_highlight("\"HELLO WORLD\"").unwrap();
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_mixed_words_and_phrases() {
        let terms = parse_and_highlight("foo \"bar baz\" qux").unwrap();
        assert_eq!(terms, vec!["bar baz", "foo", "qux"]);
    }

    #[test]
    fn test_word_with_numbers() {
        let terms = parse_and_highlight("test123").unwrap();
        assert_eq!(terms, vec!["test123"]);
    }

    #[test]
    fn test_word_with_underscore() {
        let terms = parse_and_highlight("hello_world").unwrap();
        assert_eq!(terms, vec!["hello_world"]);
    }

    #[test]
    fn test_word_with_hyphen() {
        let terms = parse_and_highlight("hello-world").unwrap();
        assert_eq!(terms, vec!["hello-world"]);
    }

    #[test]
    fn test_word_with_dot() {
        let terms = parse_and_highlight("file.txt").unwrap();
        assert_eq!(terms, vec!["file.txt"]);
    }

    #[test]
    fn test_numbers_only() {
        let terms = parse_and_highlight("12345").unwrap();
        assert_eq!(terms, vec!["12345"]);
    }

    #[test]
    fn test_special_characters() {
        let terms = parse_and_highlight("hello@world").unwrap();
        assert_eq!(terms, vec!["hello@world"]);
    }

    #[test]
    fn test_unicode_text() {
        let terms = parse_and_highlight("你好世界").unwrap();
        assert_eq!(terms, vec!["你好世界"]);
    }

    #[test]
    fn test_unicode_phrase() {
        let terms = parse_and_highlight("\"你好 世界\"").unwrap();
        assert_eq!(terms, vec!["你好 世界"]);
    }

    #[test]
    fn test_emoji() {
        let terms = parse_and_highlight("test🔥file").unwrap();
        assert_eq!(terms, vec!["test🔥file"]);
    }

    #[test]
    fn test_mixed_languages() {
        let terms = parse_and_highlight("hello 世界 test").unwrap();
        assert_eq!(terms, vec!["hello", "test", "世界"]);
    }

    #[test]
    fn test_cyrillic_text() {
        let terms = parse_and_highlight("привет мир").unwrap();
        assert_eq!(terms, vec!["мир", "привет"]);
    }

    #[test]
    fn test_arabic_text() {
        let terms = parse_and_highlight("مرحبا عالم").unwrap();
        assert_eq!(terms, vec!["عالم", "مرحبا"]);
    }

    #[test]
    fn test_japanese_text() {
        let terms = parse_and_highlight("こんにちは 世界").unwrap();
        assert_eq!(terms, vec!["こんにちは", "世界"]);
    }

    #[test]
    fn test_korean_text() {
        let terms = parse_and_highlight("안녕하세요 세계").unwrap();
        assert_eq!(terms, vec!["세계", "안녕하세요"]);
    }

    // ============================================================================
    // Wildcard and Pattern Tests (200 lines)
    // ============================================================================

    #[test]
    fn test_wildcard_star() {
        let terms = parse_and_highlight("*.txt").unwrap();
        assert_eq!(terms, vec![".txt"]);
    }

    #[test]
    fn test_wildcard_question() {
        let terms = parse_and_highlight("file?.txt").unwrap();
        assert_eq!(terms, vec![".txt", "file"]);
    }

    #[test]
    fn test_wildcard_both_sides() {
        let terms = parse_and_highlight("*test*").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_wildcard_multiple_stars() {
        let terms = parse_and_highlight("*hello*world*").unwrap();
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn test_wildcard_only_star() {
        let terms = parse_and_highlight("*").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_wildcard_only_question() {
        let terms = parse_and_highlight("?").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_wildcard_multiple_only() {
        let terms = parse_and_highlight("***???***").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_wildcard_with_spaces() {
        let terms = parse_and_highlight("* test *").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_wildcard_prefix() {
        let terms = parse_and_highlight("test*").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_wildcard_suffix() {
        let terms = parse_and_highlight("*test").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_wildcard_middle() {
        let terms = parse_and_highlight("he*lo").unwrap();
        assert_eq!(terms, vec!["he", "lo"]);
    }

    #[test]
    fn test_wildcard_complex_pattern() {
        let terms = parse_and_highlight("*test?file*.txt").unwrap();
        assert_eq!(terms, vec![".txt", "file", "test"]);
    }

    #[test]
    fn test_wildcard_numbers() {
        let terms = parse_and_highlight("file*123").unwrap();
        assert_eq!(terms, vec!["123", "file"]);
    }

    #[test]
    fn test_wildcard_underscore() {
        let terms = parse_and_highlight("test_*_file").unwrap();
        assert_eq!(terms, vec!["_file", "test_"]);
    }

    #[test]
    fn test_wildcard_hyphen() {
        let terms = parse_and_highlight("test-*-file").unwrap();
        assert_eq!(terms, vec!["-file", "test-"]);
    }

    #[test]
    fn test_wildcard_dot() {
        let terms = parse_and_highlight("test.*.file").unwrap();
        assert_eq!(terms, vec![".file", "test."]);
    }

    #[test]
    fn test_wildcard_unicode() {
        let terms = parse_and_highlight("*你好*").unwrap();
        assert_eq!(terms, vec!["你好"]);
    }

    #[test]
    fn test_wildcard_emoji() {
        let terms = parse_and_highlight("*🔥*").unwrap();
        assert_eq!(terms, vec!["🔥"]);
    }

    #[test]
    fn test_multiple_wildcards_separate_words() {
        let terms = parse_and_highlight("*.txt *.rs").unwrap();
        assert_eq!(terms, vec![".rs", ".txt"]);
    }

    #[test]
    fn test_wildcard_in_phrase() {
        let terms = parse_and_highlight("\"test * file\"").unwrap();
        assert_eq!(terms, vec!["file", "test"]);
    }

    #[test]
    fn test_home_expansion_in_highlights() {
        let Ok(_) = std::env::var("HOME") else {
            return;
        };
        let terms = parse_and_highlight("~").unwrap();

        assert_ne!(terms, vec!["~"]);
        assert!(terms.iter().all(|term| !term.contains('~')));
    }

    #[test]
    fn test_path_with_wildcard() {
        let terms = parse_and_highlight("src/*/test.rs").unwrap();
        assert_eq!(terms, vec!["test.rs"]);
    }

    #[test]
    fn test_extension_wildcard() {
        let terms = parse_and_highlight("file.*").unwrap();
        assert_eq!(terms, vec!["file."]);
    }

    #[test]
    fn test_basename_wildcard() {
        let terms = parse_and_highlight("*.tar.gz").unwrap();
        assert_eq!(terms, vec![".tar.gz"]);
    }

    #[test]
    fn test_wildcard_beginning_and_end() {
        let terms = parse_and_highlight("*file.txt*").unwrap();
        assert_eq!(terms, vec!["file.txt"]);
    }

    #[test]
    fn test_question_mark_pattern() {
        let terms = parse_and_highlight("test???").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_question_mark_middle() {
        let terms = parse_and_highlight("te?st").unwrap();
        assert_eq!(terms, vec!["st", "te"]);
    }

    #[test]
    fn test_mixed_wildcards() {
        let terms = parse_and_highlight("*test?file*").unwrap();
        assert_eq!(terms, vec!["file", "test"]);
    }

    #[test]
    fn test_wildcard_longest_segment() {
        let terms = parse_and_highlight("a*bb*ccc*dddd").unwrap();
        assert_eq!(terms, vec!["a", "bb", "ccc", "dddd"]);
    }

    #[test]
    fn test_whitespace_around_wildcard() {
        let terms = parse_and_highlight("  *test*  ").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    // ============================================================================
    // Boolean Expression Tests (200 lines)
    // ============================================================================

    #[test]
    fn test_and_expression() {
        let terms = parse_and_highlight("foo bar").unwrap();
        assert_eq!(terms, vec!["bar", "foo"]);
    }

    #[test]
    fn test_or_expression() {
        let terms = parse_and_highlight("foo|bar").unwrap();
        assert_eq!(terms, vec!["bar", "foo"]);
    }

    #[test]
    fn test_not_expression() {
        let terms = parse_and_highlight("!foo").unwrap();
        assert_eq!(terms, vec!["foo"]);
    }

    #[test]
    fn test_not_word() {
        let terms = parse_and_highlight("test !exclude").unwrap();
        assert_eq!(terms, vec!["exclude", "test"]);
    }

    #[test]
    fn test_multiple_not() {
        let terms = parse_and_highlight("foo !bar !baz").unwrap();
        assert_eq!(terms, vec!["bar", "baz", "foo"]);
    }

    #[test]
    fn test_complex_and_or() {
        let terms = parse_and_highlight("foo bar|baz qux").unwrap();
        assert_eq!(terms, vec!["bar", "baz", "foo", "qux"]);
    }

    #[test]
    fn test_grouped_expression() {
        let terms = parse_and_highlight("(foo bar)").unwrap();
        assert_eq!(terms, vec!["bar", "foo"]);
    }

    #[test]
    fn test_nested_groups() {
        let terms = parse_and_highlight("((foo bar))").unwrap();
        assert_eq!(terms, vec!["bar", "foo"]);
    }

    #[test]
    fn test_group_with_or() {
        let terms = parse_and_highlight("(foo|bar) baz").unwrap();
        assert_eq!(terms, vec!["bar", "baz", "foo"]);
    }

    #[test]
    fn test_group_with_not() {
        let terms = parse_and_highlight("!(foo bar)").unwrap();
        assert_eq!(terms, vec!["bar", "foo"]);
    }

    #[test]
    fn test_multiple_groups() {
        let terms = parse_and_highlight("(foo bar) (baz qux)").unwrap();
        assert_eq!(terms, vec!["bar", "baz", "foo", "qux"]);
    }

    #[test]
    fn test_or_with_three_terms() {
        let terms = parse_and_highlight("foo|bar|baz").unwrap();
        assert_eq!(terms, vec!["bar", "baz", "foo"]);
    }

    #[test]
    fn test_and_with_or() {
        let terms = parse_and_highlight("foo bar|baz").unwrap();
        assert_eq!(terms, vec!["bar", "baz", "foo"]);
    }

    #[test]
    fn test_complex_boolean() {
        let terms = parse_and_highlight("(foo|bar) baz !qux").unwrap();
        assert_eq!(terms, vec!["bar", "baz", "foo", "qux"]);
    }

    #[test]
    fn test_not_group() {
        let terms = parse_and_highlight("foo !(bar|baz)").unwrap();
        assert_eq!(terms, vec!["bar", "baz", "foo"]);
    }

    #[test]
    fn test_deep_nesting() {
        let terms = parse_and_highlight("((foo|(bar baz)))").unwrap();
        assert_eq!(terms, vec!["bar", "baz", "foo"]);
    }

    #[test]
    fn test_boolean_with_wildcards() {
        let terms = parse_and_highlight("*.txt|*.rs").unwrap();
        assert_eq!(terms, vec![".rs", ".txt"]);
    }

    #[test]
    fn test_boolean_with_phrases() {
        let terms = parse_and_highlight("\"hello world\"|\"foo bar\"").unwrap();
        assert_eq!(terms, vec!["foo bar", "hello world"]);
    }

    #[test]
    fn test_and_with_phrases() {
        let terms = parse_and_highlight("\"hello world\" test").unwrap();
        assert_eq!(terms, vec!["hello world", "test"]);
    }

    #[test]
    fn test_not_phrase() {
        let terms = parse_and_highlight("!\"hello world\"").unwrap();
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_multiple_or_chains() {
        let terms = parse_and_highlight("a|b c|d").unwrap();
        assert_eq!(terms, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn test_boolean_unicode() {
        let terms = parse_and_highlight("你好|世界").unwrap();
        assert_eq!(terms, vec!["世界", "你好"]);
    }

    #[test]
    fn test_empty_group() {
        assert!(parse_for_error_check("foo () bar").is_err());
    }

    #[test]
    fn test_whitespace_in_group() {
        let terms = parse_and_highlight("(   foo   bar   )").unwrap();
        assert_eq!(terms, vec!["bar", "foo"]);
    }

    #[test]
    fn test_multiple_not_operators() {
        let terms = parse_and_highlight("!!foo").unwrap();
        assert_eq!(terms, vec!["foo"]);
    }

    #[test]
    fn test_not_empty() {
        let terms = parse_and_highlight("!").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_or_empty() {
        let terms = parse_and_highlight("|").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_complex_nested_boolean() {
        let terms = parse_and_highlight("((a|b) (c|d)) | ((e|f) (g|h))").unwrap();
        assert_eq!(terms, vec!["a", "b", "c", "d", "e", "f", "g", "h"]);
    }

    #[test]
    fn test_boolean_with_numbers() {
        let terms = parse_and_highlight("123|456 789").unwrap();
        assert_eq!(terms, vec!["123", "456", "789"]);
    }

    // ============================================================================
    // Filter Tests (200 lines)
    // ============================================================================

    #[test]
    fn test_filter_bare_argument() {
        let terms = parse_and_highlight("ext:txt").unwrap();
        assert_eq!(terms, vec!["txt"]);
    }

    #[test]
    fn test_filter_phrase_argument() {
        let terms = parse_and_highlight("ext:\"tar gz\"").unwrap();
        assert_eq!(terms, vec!["tar gz"]);
    }

    #[test]
    fn test_filter_no_argument() {
        let terms = parse_and_highlight("file:").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_filter_with_word() {
        let terms = parse_and_highlight("ext:txt test").unwrap();
        assert_eq!(terms, vec!["test", "txt"]);
    }

    #[test]
    fn test_multiple_filters() {
        let terms = parse_and_highlight("ext:txt size:>1mb").unwrap();
        assert_eq!(terms, vec!["txt"]);
    }

    #[test]
    fn test_filter_wildcard_argument() {
        let terms = parse_and_highlight("ext:t*t").unwrap();
        assert_eq!(terms, vec!["t"]);
    }

    #[test]
    fn test_filter_uppercase() {
        let terms = parse_and_highlight("ext:TXT").unwrap();
        assert_eq!(terms, vec!["txt"]);
    }

    #[test]
    fn test_filter_numbers() {
        let terms = parse_and_highlight("ext:mp3").unwrap();
        assert_eq!(terms, vec!["mp3"]);
    }

    #[test]
    fn test_filter_path() {
        let terms = parse_and_highlight("path:src/test").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_filter_unicode() {
        let terms = parse_and_highlight("name:你好").unwrap();
        assert_eq!(terms, vec!["你好"]);
    }

    #[test]
    fn test_size_filter_no_highlight() {
        let terms = parse_and_highlight("size:>1mb").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_date_filter_no_highlight() {
        let terms = parse_and_highlight("dm:2024-01-01").unwrap();
        assert_eq!(terms, vec!["2024-01-01"]);
    }

    #[test]
    fn test_filter_with_boolean() {
        let terms = parse_and_highlight("ext:txt|ext:rs").unwrap();
        assert_eq!(terms, vec!["rs", "txt"]);
    }

    #[test]
    fn test_filter_with_not() {
        let terms = parse_and_highlight("test !ext:tmp").unwrap();
        assert_eq!(terms, vec!["test", "tmp"]);
    }

    #[test]
    fn test_filter_in_group() {
        let terms = parse_and_highlight("(ext:txt test)").unwrap();
        assert_eq!(terms, vec!["test", "txt"]);
    }

    #[test]
    fn test_filter_phrase_with_spaces() {
        let terms = parse_and_highlight("name:\"hello world\"").unwrap();
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_filter_multiple_arguments() {
        let terms = parse_and_highlight("ext:txt ext:rs ext:md").unwrap();
        assert_eq!(terms, vec!["md", "rs", "txt"]);
    }

    #[test]
    fn test_filter_empty_argument() {
        let terms = parse_and_highlight("ext:\"\"").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_filter_whitespace_argument() {
        let terms = parse_and_highlight("ext:\"   \"").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_folder_filter() {
        let terms = parse_and_highlight("folder:Documents").unwrap();
        assert_eq!(terms, vec!["documents"]);
    }

    #[test]
    fn test_file_filter_with_name() {
        let terms = parse_and_highlight("file:test.txt").unwrap();
        assert_eq!(terms, vec!["test.txt"]);
    }

    #[test]
    fn test_type_filter() {
        let terms = parse_and_highlight("type:picture").unwrap();
        assert_eq!(terms, vec!["picture"]);
    }

    #[test]
    fn test_filter_with_hyphen() {
        let terms = parse_and_highlight("ext:tar-gz").unwrap();
        assert_eq!(terms, vec!["tar-gz"]);
    }

    #[test]
    fn test_filter_with_underscore() {
        let terms = parse_and_highlight("name:test_file").unwrap();
        assert_eq!(terms, vec!["test_file"]);
    }

    #[test]
    fn test_filter_with_dot() {
        let terms = parse_and_highlight("name:file.test.txt").unwrap();
        assert_eq!(terms, vec!["file.test.txt"]);
    }

    #[test]
    fn test_regex_filter_no_highlight() {
        let terms = parse_and_highlight("regex:test.*").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_regex_with_other_terms() {
        let terms = parse_and_highlight("regex:test.* hello").unwrap();
        assert_eq!(terms, vec!["hello"]);
    }

    #[test]
    fn test_filter_duplicate_values() {
        let terms = parse_and_highlight("ext:txt ext:txt").unwrap();
        assert_eq!(terms, vec!["txt"]);
    }

    #[test]
    fn test_filter_case_insensitive_dedup() {
        let terms = parse_and_highlight("ext:TXT ext:txt ext:Txt").unwrap();
        assert_eq!(terms, vec!["txt"]);
    }

    #[test]
    fn test_filter_with_emoji() {
        let terms = parse_and_highlight("name:test🔥file").unwrap();
        assert_eq!(terms, vec!["test🔥file"]);
    }

    #[test]
    fn test_filter_special_chars() {
        let terms = parse_and_highlight("name:test@file.txt").unwrap();
        assert_eq!(terms, vec!["test@file.txt"]);
    }

    #[test]
    fn test_audio_filter() {
        let terms = parse_and_highlight("audio:").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_video_filter() {
        let terms = parse_and_highlight("video:").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_doc_filter() {
        let terms = parse_and_highlight("doc:").unwrap();
        assert_eq!(terms.len(), 0);
    }

    // ============================================================================
    // Edge Cases and Complex Scenarios (200 lines)
    // ============================================================================

    #[test]
    fn test_very_long_word() {
        let long_word = "a".repeat(1000);
        let terms = parse_and_highlight(&long_word).unwrap();
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].len(), 1000);
    }

    #[test]
    fn test_very_long_phrase() {
        let long_phrase = format!("\"{}\"", "test ".repeat(500));
        let terms = parse_and_highlight(&long_phrase).unwrap();
        assert_eq!(terms.len(), 1);
    }

    #[test]
    fn test_many_terms() {
        let query = (0..100)
            .map(|i| format!("word{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms.len(), 100);
    }

    #[test]
    fn test_many_or_terms() {
        let query = (0..50)
            .map(|i| format!("word{i}"))
            .collect::<Vec<_>>()
            .join("|");
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms.len(), 50);
    }

    #[test]
    fn test_deeply_nested_groups() {
        let mut query = String::from("test");
        for _ in 0..20 {
            query = format!("({query})");
        }
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_mixed_everything() {
        let terms =
            parse_and_highlight("*.txt \"hello world\" test !exclude ext:rs size:>1mb (foo|bar)")
                .unwrap();

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
        let terms = parse_and_highlight("*?*?*").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_sanitize_trim_wildcards() {
        let terms = parse_and_highlight("***test???").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_middle_wildcards() {
        let terms = parse_and_highlight("test*ing").unwrap();
        assert_eq!(terms, vec!["ing", "test"]);
    }

    #[test]
    fn test_empty_segments() {
        let terms = parse_and_highlight("**hello**world**").unwrap();
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn test_only_whitespace_segments() {
        let terms = parse_and_highlight("   ").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_tab_characters() {
        let terms = parse_and_highlight("hello\tworld").unwrap();
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn test_newline_characters() {
        let terms = parse_and_highlight("hello\nworld").unwrap();
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn test_carriage_return() {
        let terms = parse_and_highlight("hello\rworld").unwrap();
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn test_mixed_whitespace() {
        let terms = parse_and_highlight("hello \t\n\r world").unwrap();
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn test_zero_width_characters() {
        let terms = parse_and_highlight("test\u{200B}file").unwrap();
        assert_eq!(terms.len(), 1);
    }

    #[test]
    fn test_combining_characters() {
        let terms = parse_and_highlight("café").unwrap();
        assert_eq!(terms.len(), 1);
    }

    #[test]
    fn test_rtl_text() {
        let terms = parse_and_highlight("שלום עולם").unwrap();
        assert_eq!(terms.len(), 2);
    }

    #[test]
    fn test_mixed_rtl_ltr() {
        let terms = parse_and_highlight("hello שלום world").unwrap();
        assert_eq!(terms.len(), 3);
    }

    #[test]
    fn test_backslash_in_query() {
        let terms = parse_and_highlight("path\\to\\file").unwrap();
        assert_eq!(terms, vec!["path\\to\\file"]);
    }

    #[test]
    fn test_forward_slash_in_query() {
        let terms = parse_and_highlight("path/to/file").unwrap();
        assert_eq!(terms, vec!["file"]);
    }

    #[test]
    fn test_mixed_slashes() {
        let terms = parse_and_highlight("path\\to/file").unwrap();
        assert_eq!(terms, vec!["file"]);
    }

    #[test]
    fn test_quotes_in_word() {
        let terms = parse_and_highlight("\"Trae CN\"/emm.db").unwrap();

        assert!(!terms.is_empty());
    }

    #[test]
    fn test_quotes_in_word_adjacent() {
        assert!(parse_for_error_check("test\"file").is_err());
    }

    #[test]
    fn test_quotes_in_word_adjacent_balanced() {
        let terms = parse_and_highlight("test\"file\"").unwrap();

        assert!(!terms.is_empty());
    }

    #[test]
    fn test_parentheses_in_word() {
        let terms = parse_and_highlight("test(file)").unwrap();

        assert!(!terms.is_empty());
    }

    #[test]
    fn test_brackets_in_word() {
        let terms = parse_and_highlight("test[file]").unwrap();
        assert_eq!(terms, vec!["test[file]"]);
    }

    #[test]
    fn test_braces_in_word() {
        let terms = parse_and_highlight("test{file}").unwrap();
        assert_eq!(terms, vec!["test{file}"]);
    }

    #[test]
    fn test_angle_brackets_in_word() {
        let terms = parse_and_highlight("test<file>").unwrap();
        assert_eq!(terms, vec!["file", "test"]);
    }

    #[test]
    fn test_ampersand_in_word() {
        let terms = parse_and_highlight("test&file").unwrap();
        assert_eq!(terms, vec!["test&file"]);
    }

    #[test]
    fn test_pipe_in_phrase() {
        let terms = parse_and_highlight("\"test|file\"").unwrap();
        assert_eq!(terms, vec!["test|file"]);
    }

    #[test]
    fn test_exclamation_in_phrase() {
        let terms = parse_and_highlight("\"test!file\"").unwrap();
        assert_eq!(terms, vec!["test!file"]);
    }

    #[test]
    fn test_colon_in_word() {
        let terms = parse_and_highlight("test:file").unwrap();
        assert_eq!(terms, vec!["file"]);
    }

    #[test]
    fn test_semicolon_in_word() {
        let terms = parse_and_highlight("test;file").unwrap();
        assert_eq!(terms, vec!["test;file"]);
    }

    #[test]
    fn test_comma_in_word() {
        let terms = parse_and_highlight("test,file").unwrap();
        assert_eq!(terms, vec!["test,file"]);
    }

    #[test]
    fn test_percent_in_word() {
        let terms = parse_and_highlight("test%file").unwrap();
        assert_eq!(terms, vec!["test%file"]);
    }

    #[test]
    fn test_dollar_in_word() {
        let terms = parse_and_highlight("test$file").unwrap();
        assert_eq!(terms, vec!["test$file"]);
    }

    #[test]
    fn test_hash_in_word() {
        let terms = parse_and_highlight("test#file").unwrap();
        assert_eq!(terms, vec!["test#file"]);
    }

    #[test]
    fn test_plus_in_word() {
        let terms = parse_and_highlight("test+file").unwrap();
        assert_eq!(terms, vec!["test+file"]);
    }

    #[test]
    fn test_equals_in_word() {
        let terms = parse_and_highlight("test=file").unwrap();
        assert_eq!(terms, vec!["test=file"]);
    }

    #[test]
    fn test_tilde_in_word() {
        let terms = parse_and_highlight("test~file").unwrap();
        assert_eq!(terms, vec!["test~file"]);
    }

    #[test]
    fn test_backtick_in_word() {
        let terms = parse_and_highlight("test`file").unwrap();
        assert_eq!(terms, vec!["test`file"]);
    }

    // ============================================================================
    // Query Segmentation Integration Tests (200 lines)
    // ============================================================================

    #[test]
    fn test_segmentation_camelcase() {
        let terms = parse_and_highlight("testFile").unwrap();
        assert_eq!(terms, vec!["testfile"]);
    }

    #[test]
    fn test_segmentation_pascalcase() {
        let terms = parse_and_highlight("TestFile").unwrap();
        assert_eq!(terms, vec!["testfile"]);
    }

    #[test]
    fn test_segmentation_snake_case() {
        let terms = parse_and_highlight("test_file").unwrap();
        assert_eq!(terms, vec!["test_file"]);
    }

    #[test]
    fn test_segmentation_kebab_case() {
        let terms = parse_and_highlight("test-file").unwrap();
        assert_eq!(terms, vec!["test-file"]);
    }

    #[test]
    fn test_segmentation_dot_separated() {
        let terms = parse_and_highlight("test.file.name").unwrap();
        assert_eq!(terms, vec!["test.file.name"]);
    }

    #[test]
    fn test_segmentation_mixed_case() {
        let terms = parse_and_highlight("TestFile_Name").unwrap();
        assert_eq!(terms, vec!["testfile_name"]);
    }

    #[test]
    fn test_segmentation_with_numbers() {
        let terms = parse_and_highlight("test123File").unwrap();
        assert_eq!(terms, vec!["test123file"]);
    }

    #[test]
    fn test_segmentation_all_caps() {
        let terms = parse_and_highlight("TESTFILE").unwrap();
        assert_eq!(terms, vec!["testfile"]);
    }

    #[test]
    fn test_segmentation_alternating_case() {
        let terms = parse_and_highlight("TeSt").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_segmentation_with_wildcard() {
        let terms = parse_and_highlight("test*File").unwrap();
        assert_eq!(terms, vec!["file", "test"]);
    }

    #[test]
    fn test_segmentation_multiple_words() {
        let terms = parse_and_highlight("testFile anotherTest").unwrap();
        assert_eq!(terms, vec!["anothertest", "testfile"]);
    }

    #[test]
    fn test_segmentation_phrase() {
        let terms = parse_and_highlight("\"testFile\"").unwrap();
        assert_eq!(terms, vec!["testfile"]);
    }

    #[test]
    fn test_segmentation_in_filter() {
        let terms = parse_and_highlight("name:testFile").unwrap();
        assert_eq!(terms, vec!["testfile"]);
    }

    #[test]
    fn test_segmentation_complex() {
        let terms = parse_and_highlight("myTestFile_v2").unwrap();
        assert_eq!(terms, vec!["mytestfile_v2"]);
    }

    #[test]
    fn test_segmentation_with_path() {
        let terms = parse_and_highlight("src/testFile/index.ts").unwrap();
        assert_eq!(terms, vec!["index.ts"]);
    }

    #[test]
    fn test_longest_segment_selection() {
        let terms = parse_and_highlight("a*bb*ccc").unwrap();
        assert_eq!(terms, vec!["a", "bb", "ccc"]);
    }

    #[test]
    fn test_segment_with_underscores() {
        let terms = parse_and_highlight("__test__file__").unwrap();
        assert_eq!(terms, vec!["__test__file__"]);
    }

    #[test]
    fn test_segment_with_hyphens() {
        let terms = parse_and_highlight("--test--file--").unwrap();
        assert_eq!(terms, vec!["--test--file--"]);
    }

    #[test]
    fn test_segment_leading_numbers() {
        let terms = parse_and_highlight("123test").unwrap();
        assert_eq!(terms, vec!["123test"]);
    }

    #[test]
    fn test_segment_trailing_numbers() {
        let terms = parse_and_highlight("test123").unwrap();
        assert_eq!(terms, vec!["test123"]);
    }

    #[test]
    fn test_segment_only_numbers() {
        let terms = parse_and_highlight("123456").unwrap();
        assert_eq!(terms, vec!["123456"]);
    }

    #[test]
    fn test_segment_mixed_separators() {
        let terms = parse_and_highlight("test_file-name.txt").unwrap();
        assert_eq!(terms, vec!["test_file-name.txt"]);
    }

    #[test]
    fn test_segment_unicode_camelcase() {
        let terms = parse_and_highlight("测试File").unwrap();
        assert_eq!(terms, vec!["测试file"]);
    }

    #[test]
    fn test_segment_emoji_separator() {
        let terms = parse_and_highlight("test🔥file").unwrap();
        assert_eq!(terms, vec!["test🔥file"]);
    }

    #[test]
    fn test_segment_multiple_extensions() {
        let terms = parse_and_highlight("archive.tar.gz").unwrap();
        assert_eq!(terms, vec!["archive.tar.gz"]);
    }

    #[test]
    fn test_segment_version_number() {
        let terms = parse_and_highlight("package-v1.2.3").unwrap();
        assert_eq!(terms, vec!["package-v1.2.3"]);
    }

    #[test]
    fn test_segment_date_like() {
        let terms = parse_and_highlight("report-2024-01-15").unwrap();
        assert_eq!(terms, vec!["report-2024-01-15"]);
    }

    #[test]
    fn test_segment_uuid_like() {
        let terms = parse_and_highlight("file-550e8400-e29b-41d4").unwrap();
        assert_eq!(terms, vec!["file-550e8400-e29b-41d4"]);
    }

    #[test]
    fn test_segment_hash_like() {
        let terms = parse_and_highlight("commit-abc123def456").unwrap();
        assert_eq!(terms, vec!["commit-abc123def456"]);
    }

    #[test]
    fn test_segment_url_like() {
        let terms = parse_and_highlight("https://example.com").unwrap();
        assert_eq!(terms, vec!["example.com"]);
    }

    #[test]
    fn test_segment_email_like() {
        let terms = parse_and_highlight("user@example.com").unwrap();
        assert_eq!(terms, vec!["user@example.com"]);
    }

    #[test]
    fn test_segment_ipv4_like() {
        let terms = parse_and_highlight("192.168.1.1").unwrap();
        assert_eq!(terms, vec!["192.168.1.1"]);
    }

    #[test]
    fn test_segment_single_char() {
        let terms = parse_and_highlight("a").unwrap();
        assert_eq!(terms, vec!["a"]);
    }

    #[test]
    fn test_segment_two_chars() {
        let terms = parse_and_highlight("ab").unwrap();
        assert_eq!(terms, vec!["ab"]);
    }

    #[test]
    fn test_segment_repeated_chars() {
        let terms = parse_and_highlight("aaa").unwrap();
        assert_eq!(terms, vec!["aaa"]);
    }

    #[test]
    fn test_segment_palindrome() {
        let terms = parse_and_highlight("racecar").unwrap();
        assert_eq!(terms, vec!["racecar"]);
    }

    #[test]
    fn test_segment_abbreviation() {
        let terms = parse_and_highlight("USA").unwrap();
        assert_eq!(terms, vec!["usa"]);
    }

    #[test]
    fn test_segment_acronym() {
        let terms = parse_and_highlight("HTTP").unwrap();
        assert_eq!(terms, vec!["http"]);
    }

    #[test]
    fn test_segment_mixed_acronym() {
        let terms = parse_and_highlight("HTTPServer").unwrap();
        assert_eq!(terms, vec!["httpserver"]);
    }

    // ============================================================================
    // Sanitization and Trimming Tests (200 lines)
    // ============================================================================

    #[test]
    fn test_sanitize_leading_wildcards() {
        let terms = parse_and_highlight("***test").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_trailing_wildcards() {
        let terms = parse_and_highlight("test***").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_leading_questions() {
        let terms = parse_and_highlight("???test").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_trailing_questions() {
        let terms = parse_and_highlight("test???").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_mixed_leading() {
        let terms = parse_and_highlight("*?*?test").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_mixed_trailing() {
        let terms = parse_and_highlight("test*?*?").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_both_ends() {
        let terms = parse_and_highlight("***test???").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_with_spaces() {
        let terms = parse_and_highlight("  ***test???  ").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_preserve_middle() {
        let terms = parse_and_highlight("te*st").unwrap();
        assert_eq!(terms, vec!["st", "te"]);
    }

    #[test]
    fn test_sanitize_preserve_question_middle() {
        let terms = parse_and_highlight("te?st").unwrap();
        assert_eq!(terms, vec!["st", "te"]);
    }

    #[test]
    fn test_sanitize_longest_chunk() {
        let terms = parse_and_highlight("a*bb*ccc*dddd").unwrap();
        assert_eq!(terms, vec!["a", "bb", "ccc", "dddd"]);
    }

    #[test]
    fn test_sanitize_equal_chunks() {
        let terms = parse_and_highlight("aa*bb*cc").unwrap();
        assert_eq!(terms, vec!["aa", "bb", "cc"]);
    }

    #[test]
    fn test_sanitize_single_char_chunks() {
        let terms = parse_and_highlight("a*b*c*d").unwrap();
        assert_eq!(terms, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn test_sanitize_empty_after_trim() {
        let terms = parse_and_highlight("***").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_sanitize_spaces_only_after_trim() {
        let terms = parse_and_highlight("*   *").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_sanitize_unicode_with_wildcards() {
        let terms = parse_and_highlight("*你好*").unwrap();
        assert_eq!(terms, vec!["你好"]);
    }

    #[test]
    fn test_sanitize_emoji_with_wildcards() {
        let terms = parse_and_highlight("*🔥*").unwrap();
        assert_eq!(terms, vec!["🔥"]);
    }

    #[test]
    fn test_sanitize_number_with_wildcards() {
        let terms = parse_and_highlight("*123*").unwrap();
        assert_eq!(terms, vec!["123"]);
    }

    #[test]
    fn test_sanitize_path_with_wildcards() {
        let terms = parse_and_highlight("*/path/to/file*").unwrap();
        assert_eq!(terms, vec!["file"]);
    }

    #[test]
    fn test_sanitize_extension_pattern() {
        let terms = parse_and_highlight("***.txt***").unwrap();
        assert_eq!(terms, vec![".txt"]);
    }

    #[test]
    fn test_sanitize_hyphen_separated() {
        let terms = parse_and_highlight("*test-file*").unwrap();
        assert_eq!(terms, vec!["test-file"]);
    }

    #[test]
    fn test_sanitize_underscore_separated() {
        let terms = parse_and_highlight("*test_file*").unwrap();
        assert_eq!(terms, vec!["test_file"]);
    }

    #[test]
    fn test_sanitize_dot_separated() {
        let terms = parse_and_highlight("*test.file*").unwrap();
        assert_eq!(terms, vec!["test.file"]);
    }

    #[test]
    fn test_sanitize_complex_pattern() {
        let terms = parse_and_highlight("***test*file***name*").unwrap();
        assert_eq!(terms, vec!["file", "name", "test"]);
    }

    #[test]
    fn test_sanitize_alternating_wildcards() {
        let terms = parse_and_highlight("*?*?test*?*?").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_in_phrase() {
        let terms = parse_and_highlight("\"***test***\"").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_multiple_words() {
        let terms = parse_and_highlight("*test* *file*").unwrap();
        assert_eq!(terms, vec!["file", "test"]);
    }

    #[test]
    fn test_sanitize_with_boolean() {
        let terms = parse_and_highlight("*test*|*file*").unwrap();
        assert_eq!(terms, vec!["file", "test"]);
    }

    #[test]
    fn test_sanitize_with_not() {
        let terms = parse_and_highlight("!*test*").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_in_filter() {
        let terms = parse_and_highlight("name:*test*").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_wildcard_only_in_filter() {
        let terms = parse_and_highlight("name:***").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_trim_leading_spaces() {
        let terms = parse_and_highlight("   test").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_trim_trailing_spaces() {
        let terms = parse_and_highlight("test   ").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_trim_both_spaces() {
        let terms = parse_and_highlight("   test   ").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_trim_internal_spaces_preserved() {
        let terms = parse_and_highlight("\"  test   file  \"").unwrap();
        assert_eq!(terms, vec!["test   file"]);
    }

    // ============================================================================
    // Quote Handling Tests
    // ============================================================================

    #[test]
    fn test_quoted_phrase_strips_quotes() {
        let terms = parse_and_highlight("\"hello world\"").unwrap();
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_quoted_single_word() {
        let terms = parse_and_highlight("\"test\"").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_multiple_quoted_phrases() {
        let terms = parse_and_highlight("\"hello world\" \"foo bar\"").unwrap();
        assert_eq!(terms, vec!["foo bar", "hello world"]);
    }

    #[test]
    fn test_quoted_with_wildcards_extracts_segments() {
        let terms = parse_and_highlight("\"test * file\"").unwrap();
        assert_eq!(terms, vec!["file", "test"]);
    }

    #[test]
    fn test_quoted_empty() {
        let terms = parse_and_highlight("\"\"").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_quoted_whitespace_only() {
        let terms = parse_and_highlight("\"   \"").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_quoted_with_leading_trailing_spaces() {
        let terms = parse_and_highlight("\"  hello  \"").unwrap();
        assert_eq!(terms, vec!["hello"]);
    }

    #[test]
    fn test_filter_quoted_argument() {
        let terms = parse_and_highlight("ext:\"tar gz\"").unwrap();
        assert_eq!(terms, vec!["tar gz"]);
    }

    #[test]
    fn test_filter_quoted_with_wildcards() {
        let terms = parse_and_highlight("name:\"*test*\"").unwrap();

        // Quoted argument is treated as phrase, wildcards are processed
        assert_eq!(terms, vec!["*test*"]);
    }

    #[test]
    fn test_filter_quoted_empty() {
        let terms = parse_and_highlight("ext:\"\"").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_quoted_phrase_argument_in_filter() {
        let terms = parse_and_highlight("content:\"hello world\"").unwrap();
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_word_with_embedded_quotes() {
        let terms = parse_and_highlight("test\"file\"name").unwrap();
        assert_eq!(terms, vec!["testfilename"]);
    }

    #[test]
    fn test_adjacent_quoted_sections() {
        let terms = parse_and_highlight("\"hello\"\"world\"").unwrap();
        assert_eq!(terms, vec!["helloworld"]);
    }

    #[test]
    fn test_quoted_prefix_unquoted_suffix() {
        let terms = parse_and_highlight("\"prefix\"suffix").unwrap();
        assert_eq!(terms, vec!["prefixsuffix"]);
    }

    #[test]
    fn test_unquoted_prefix_quoted_suffix() {
        let terms = parse_and_highlight("prefix\"suffix\"").unwrap();
        assert_eq!(terms, vec!["prefixsuffix"]);
    }

    #[test]
    fn test_quoted_middle_segment() {
        let terms = parse_and_highlight("pre\"middle\"fix").unwrap();
        assert_eq!(terms, vec!["premiddlefix"]);
    }

    #[test]
    fn test_multiple_empty_quotes_in_word() {
        let terms = parse_and_highlight("test\"\"\"\"file").unwrap();
        assert_eq!(terms, vec!["testfile"]);
    }

    #[test]
    fn test_quoted_unicode_phrase() {
        let terms = parse_and_highlight("\"你好 世界\"").unwrap();
        assert_eq!(terms, vec!["你好 世界"]);
    }

    #[test]
    fn test_quoted_emoji_phrase() {
        let terms = parse_and_highlight("\"test 🔥 file\"").unwrap();
        assert_eq!(terms, vec!["test 🔥 file"]);
    }

    #[test]
    fn test_quoted_special_chars_preserved() {
        let terms = parse_and_highlight("\"!@#$%^&*()\"").unwrap();
        assert_eq!(terms, vec!["!@#$%^&", "()"]);
    }

    #[test]
    fn test_quoted_path_like() {
        let terms = parse_and_highlight("\"path/to/file\"").unwrap();
        assert_eq!(terms, vec!["file"]);
    }

    #[test]
    fn test_quoted_with_backslashes() {
        let terms = parse_and_highlight("\"path\\to\\file\"").unwrap();
        assert_eq!(terms, vec!["path\\to\\file"]);
    }

    #[test]
    fn test_collect_literal_text_strips_quotes() {
        let terms = parse_and_highlight("\"literal text\"").unwrap();

        assert!(!terms.iter().any(|t| t.contains('"')));
    }

    #[test]
    fn test_strip_quotes_from_filter_phrase() {
        let terms = parse_and_highlight("parent:\"/Users/demo\"").unwrap();
        assert_eq!(terms, vec!["/users/demo"]);
    }

    #[test]
    fn test_filter_phrase_with_escaped_backslashes() {
        let terms = parse_and_highlight(r#"parent:"C:\\Users\\Demo""#).unwrap();
        assert_eq!(terms, vec![r"c:\users\demo"]);
    }

    #[test]
    fn test_quoted_comparison_in_filter() {
        let terms = parse_and_highlight("name:>\"test\"").unwrap();

        // Comparison values are not highlighted
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_quoted_range_in_filter() {
        let terms = parse_and_highlight("name:\"a\"..\"z\"").unwrap();

        // Range values are not highlighted
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_filter_list_with_quoted_items() {
        let terms = parse_and_highlight("ext:\"jpg\";\"png\";\"gif\"").unwrap();

        // After stripping quotes from the list items
        assert_eq!(terms, vec!["gif", "jpg", "png"]);
    }

    #[test]
    fn test_filter_list_with_escaped_backslashes() {
        let terms = parse_and_highlight(r#"ext:"J\\PG";"P\\NG""#).unwrap();
        assert_eq!(terms, vec![r"j\pg", r"p\ng"]);
    }

    #[test]
    fn test_trim_tabs() {
        let terms = parse_and_highlight("\t\ttest\t\t").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_trim_newlines() {
        let terms = parse_and_highlight("\n\ntest\n\n").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_trim_mixed_whitespace() {
        let terms = parse_and_highlight(" \t\n test \n\t ").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_sanitize_preserve_internal_structure() {
        let terms = parse_and_highlight("*test*file*name*").unwrap();
        assert_eq!(terms, vec!["file", "name", "test"]);
    }

    #[test]
    fn test_sanitize_single_wildcard_between() {
        let terms = parse_and_highlight("test*file").unwrap();
        assert_eq!(terms, vec!["file", "test"]);
    }

    #[test]
    fn test_sanitize_multiple_wildcards_between() {
        let terms = parse_and_highlight("test***file").unwrap();
        assert_eq!(terms, vec!["file", "test"]);
    }

    // ============================================================================
    // Duplicate Handling and Case Sensitivity Tests (200 lines)
    // ============================================================================

    #[test]
    fn test_dedup_exact_duplicates() {
        let terms = parse_and_highlight("test test test").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_case_insensitive() {
        let terms = parse_and_highlight("Test test TEST").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_mixed_case() {
        let terms = parse_and_highlight("test Test TeSt tEsT").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_in_boolean() {
        let terms = parse_and_highlight("test | test | test").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_in_and_expression() {
        let terms = parse_and_highlight("test test").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_with_wildcards() {
        let terms = parse_and_highlight("*test* *TEST* *Test*").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_phrases() {
        let terms = parse_and_highlight("\"hello world\" \"HELLO WORLD\"").unwrap();
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_dedup_in_filters() {
        let terms = parse_and_highlight("ext:txt ext:TXT ext:Txt").unwrap();
        assert_eq!(terms, vec!["txt"]);
    }

    #[test]
    fn test_dedup_complex_query() {
        let terms = parse_and_highlight("test Test (test | TEST) !test ext:test").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_no_dedup_different_words() {
        let terms = parse_and_highlight("test file name").unwrap();
        assert_eq!(terms, vec!["file", "name", "test"]);
    }

    #[test]
    fn test_dedup_unicode() {
        let terms = parse_and_highlight("你好 你好 你好").unwrap();
        assert_eq!(terms, vec!["你好"]);
    }

    #[test]
    fn test_dedup_emoji() {
        let terms = parse_and_highlight("🔥 🔥 🔥").unwrap();
        assert_eq!(terms, vec!["🔥"]);
    }

    #[test]
    fn test_case_lowercase() {
        let terms = parse_and_highlight("test").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_case_uppercase() {
        let terms = parse_and_highlight("TEST").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_case_titlecase() {
        let terms = parse_and_highlight("Test").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_case_camelcase() {
        let terms = parse_and_highlight("testFile").unwrap();
        assert_eq!(terms, vec!["testfile"]);
    }

    #[test]
    fn test_case_pascalcase() {
        let terms = parse_and_highlight("TestFile").unwrap();
        assert_eq!(terms, vec!["testfile"]);
    }

    #[test]
    fn test_case_snake_case() {
        let terms = parse_and_highlight("TEST_FILE").unwrap();
        assert_eq!(terms, vec!["test_file"]);
    }

    #[test]
    fn test_case_screaming_snake_case() {
        let terms = parse_and_highlight("TEST_FILE_NAME").unwrap();
        assert_eq!(terms, vec!["test_file_name"]);
    }

    #[test]
    fn test_case_kebab_case_upper() {
        let terms = parse_and_highlight("TEST-FILE").unwrap();
        assert_eq!(terms, vec!["test-file"]);
    }

    #[test]
    fn test_case_mixed_separators() {
        let terms = parse_and_highlight("TEST_file-NAME").unwrap();
        assert_eq!(terms, vec!["test_file-name"]);
    }

    #[test]
    fn test_case_phrase_lowercase() {
        let terms = parse_and_highlight("\"hello world\"").unwrap();
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_case_phrase_uppercase() {
        let terms = parse_and_highlight("\"HELLO WORLD\"").unwrap();
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_case_phrase_mixedcase() {
        let terms = parse_and_highlight("\"Hello World\"").unwrap();
        assert_eq!(terms, vec!["hello world"]);
    }

    #[test]
    fn test_case_unicode_lowercase() {
        let terms = parse_and_highlight("café").unwrap();

        assert!(terms.len() == 1);
    }

    #[test]
    fn test_case_unicode_uppercase() {
        let terms = parse_and_highlight("CAFÉ").unwrap();

        assert!(terms.len() == 1);
    }

    #[test]
    fn test_case_cyrillic_lower() {
        let terms = parse_and_highlight("привет").unwrap();
        assert_eq!(terms, vec!["привет"]);
    }

    #[test]
    fn test_case_cyrillic_upper() {
        let terms = parse_and_highlight("ПРИВЕТ").unwrap();
        assert_eq!(terms, vec!["привет"]);
    }

    #[test]
    fn test_case_greek_lower() {
        let terms = parse_and_highlight("γεια").unwrap();
        assert_eq!(terms, vec!["γεια"]);
    }

    #[test]
    fn test_case_greek_upper() {
        let terms = parse_and_highlight("ΓΕΙΑ").unwrap();
        assert_eq!(terms, vec!["γεια"]);
    }

    #[test]
    fn test_dedup_with_spaces() {
        let terms = parse_and_highlight("test   test test").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_across_groups() {
        let terms = parse_and_highlight("(test) test (test)").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_with_not() {
        let terms = parse_and_highlight("test !test test").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_in_nested_groups() {
        let terms = parse_and_highlight("((test) (test))").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_mixed_terms() {
        let terms = parse_and_highlight("test \"test\" *test*").unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_dedup_filter_arguments() {
        let terms = parse_and_highlight("ext:txt name:txt path:txt").unwrap();
        assert_eq!(terms, vec!["txt"]);
    }

    #[test]
    fn test_dedup_numbers() {
        let terms = parse_and_highlight("123 123 123").unwrap();
        assert_eq!(terms, vec!["123"]);
    }

    #[test]
    fn test_dedup_paths() {
        let terms = parse_and_highlight("path/to/file path/to/file").unwrap();
        assert_eq!(terms, vec!["file"]);
    }

    #[test]
    fn test_dedup_extensions() {
        let terms = parse_and_highlight("*.txt *.TXT *.Txt").unwrap();
        assert_eq!(terms, vec![".txt"]);
    }

    #[test]
    fn test_dedup_hyphenated() {
        let terms = parse_and_highlight("test-file test-FILE TEST-file").unwrap();
        assert_eq!(terms, vec!["test-file"]);
    }

    #[test]
    fn test_dedup_underscored() {
        let terms = parse_and_highlight("test_file test_FILE TEST_file").unwrap();
        assert_eq!(terms, vec!["test_file"]);
    }

    #[test]
    fn test_dedup_dotted() {
        let terms = parse_and_highlight("test.file test.FILE TEST.file").unwrap();
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
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms.len(), 200);
    }

    #[test]
    fn test_stress_many_or_terms() {
        let query = (0..100)
            .map(|i| format!("word{i}"))
            .collect::<Vec<_>>()
            .join("|");
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms.len(), 100);
    }

    #[test]
    fn test_stress_many_phrases() {
        let query = (0..50)
            .map(|i| format!("\"phrase{i}\""))
            .collect::<Vec<_>>()
            .join(" ");
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms.len(), 50);
    }

    #[test]
    fn test_stress_many_filters() {
        let query = (0..50)
            .map(|i| format!("ext:ext{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms.len(), 50);
    }

    #[test]
    fn test_stress_long_word() {
        let long_word = "test".repeat(500);
        let terms = parse_and_highlight(&long_word).unwrap();
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].len(), 2000);
    }

    #[test]
    fn test_stress_long_phrase() {
        let long_phrase = format!("\"{}\"", "test ".repeat(1000));
        let terms = parse_and_highlight(&long_phrase).unwrap();
        assert_eq!(terms.len(), 1);
    }

    #[test]
    fn test_stress_many_wildcards() {
        let query = "*".repeat(1000) + "test" + &"*".repeat(1000);
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_stress_alternating_wildcards() {
        let mut query = String::new();
        for i in 0..100 {
            query.push_str(&format!("word{i}*"));
        }
        let terms = parse_and_highlight(&query).unwrap();

        assert!(!terms.is_empty());
    }

    #[test]
    fn test_stress_deep_nesting() {
        let mut query = String::from("test");
        for _ in 0..50 {
            query = format!("({query})");
        }
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_stress_wide_or_tree() {
        let parts = (0..200).map(|i| format!("w{i}")).collect::<Vec<_>>();
        let query = parts.join("|");
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms.len(), 200);
    }

    #[test]
    fn test_stress_wide_and_tree() {
        let parts = (0..200).map(|i| format!("w{i}")).collect::<Vec<_>>();
        let query = parts.join(" ");
        let terms = parse_and_highlight(&query).unwrap();
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
        let terms = parse_and_highlight(&query).unwrap();

        assert!(!terms.is_empty());
    }

    #[test]
    fn test_stress_many_not_operators() {
        let mut query = String::new();
        for i in 0..100 {
            query.push_str(&format!("!w{i} "));
        }
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms.len(), 100);
    }

    #[test]
    fn test_stress_unicode_heavy() {
        let query =
            "你好 世界 測試 테스트 тест test प्रयोग परीक्षण δοκιμή تجربة テスト 試験".to_string();
        let terms = parse_and_highlight(&query).unwrap();

        assert!(terms.len() >= 10);
    }

    #[test]
    fn test_stress_emoji_heavy() {
        let query = "🔥 ⚡ 🎉 💻 📁 📄 🎨 🎯 ⭐ 💡 🚀 🌟 ✨ 🎁 🔔".to_string();
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms.len(), 15);
    }

    #[test]
    fn test_stress_mixed_scripts() {
        let query = "test你好привет🔥مرحبا".to_string();
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms.len(), 1);
    }

    #[test]
    fn test_stress_repeated_duplicates() {
        let query = "test ".repeat(500);
        let terms = parse_and_highlight(&query).unwrap();
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
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_stress_complex_nested_boolean() {
        let query = "((a|b) (c|d)) ((e|f) (g|h)) ((i|j) (k|l)) ((m|n) (o|p))".to_string();
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms.len(), 16);
    }

    #[test]
    fn test_stress_filter_variations() {
        let query =
            "ext:txt name:file path:dir folder:test type:doc size:>1mb dm:today".to_string();
        let terms = parse_and_highlight(&query).unwrap();

        assert!(terms.len() >= 3);
    }

    #[test]
    fn test_stress_wildcard_patterns() {
        let query = "*test* test* *test t*st te*t *t*e*s*t* *.txt file.* *.tar.gz".to_string();
        let terms = parse_and_highlight(&query).unwrap();

        assert!(!terms.is_empty());
    }

    #[test]
    fn test_stress_phrase_variations() {
        let query = "\"test\" \"test file\" \"test file name\" \"a\" \"ab\" \"abc\"".to_string();
        let terms = parse_and_highlight(&query).unwrap();

        assert!(terms.len() >= 6);
    }

    #[test]
    fn test_stress_path_like_queries() {
        let query = "src/main.rs lib/util.rs test/test.rs src/components/Button.tsx".to_string();
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms.len(), 4);
    }

    #[test]
    fn test_stress_extension_patterns() {
        let query = "*.txt *.rs *.js *.ts *.jsx *.tsx *.md *.json *.toml *.yaml".to_string();
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms.len(), 10);
    }

    #[test]
    fn test_stress_number_variations() {
        let query = "1 12 123 1234 12345 123456 1234567 12345678".to_string();
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms.len(), 8);
    }

    #[test]
    fn test_stress_special_char_combinations() {
        let query = "test@file test#file test$file test%file test&file".to_string();
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms.len(), 5);
    }

    #[test]
    fn test_stress_mixed_separators_many() {
        let query = "test-file test_file test.file test/file test\\file".to_string();
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms.len(), 5);
    }

    #[test]
    fn test_stress_camelcase_variations() {
        let query = "testFile TestFile testfile TESTFILE testFILE TESTfile".to_string();
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms, vec!["testfile"]);
    }

    #[test]
    fn test_stress_empty_elements() {
        let query = "test   test   test".to_string();
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_stress_group_variations() {
        let query = "(test) ((test)) (((test))) ((((test))))".to_string();
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    #[test]
    fn test_stress_not_variations() {
        let query = "!test !!test !!!test !!!!test".to_string();
        let terms = parse_and_highlight(&query).unwrap();
        assert_eq!(terms, vec!["test"]);
    }

    // ============================================================================
    // Integration and Real-World Scenarios (200+ lines)
    // ============================================================================

    #[test]
    fn test_real_code_search() {
        let terms = parse_and_highlight("*.rs cargo test").unwrap();

        assert!(terms.contains(&".rs".to_string()));
        assert!(terms.contains(&"cargo".to_string()));
        assert!(terms.contains(&"test".to_string()));
    }

    #[test]
    fn test_real_document_search() {
        let terms = parse_and_highlight("\"project report\" *.pdf 2024").unwrap();

        assert!(terms.contains(&"project report".to_string()));
        assert!(terms.contains(&".pdf".to_string()));
        assert!(terms.contains(&"2024".to_string()));
    }

    #[test]
    fn test_real_image_search() {
        let terms = parse_and_highlight("vacation (*.jpg|*.png) !thumbnail").unwrap();

        assert!(terms.contains(&"vacation".to_string()));
        assert!(terms.contains(&".jpg".to_string()));
        assert!(terms.contains(&".png".to_string()));
        assert!(terms.contains(&"thumbnail".to_string()));
    }

    #[test]
    fn test_real_log_search() {
        let terms = parse_and_highlight("error *.log !debug").unwrap();

        assert!(terms.contains(&"error".to_string()));
        assert!(terms.contains(&".log".to_string()));
        assert!(terms.contains(&"debug".to_string()));
    }

    #[test]
    fn test_real_config_search() {
        let terms = parse_and_highlight("(*.toml|*.yaml|*.json) config").unwrap();

        assert!(terms.contains(&".toml".to_string()));
        assert!(terms.contains(&".yaml".to_string()));
        assert!(terms.contains(&".json".to_string()));
        assert!(terms.contains(&"config".to_string()));
    }

    #[test]
    fn test_real_backup_search() {
        let terms = parse_and_highlight("backup *.zip size:>1gb").unwrap();

        assert!(terms.contains(&"backup".to_string()));
        assert!(terms.contains(&".zip".to_string()));
    }

    #[test]
    fn test_real_temp_cleanup() {
        let terms = parse_and_highlight("(temp|tmp|cache) !important").unwrap();

        assert!(terms.contains(&"temp".to_string()));
        assert!(terms.contains(&"tmp".to_string()));
        assert!(terms.contains(&"cache".to_string()));
        assert!(terms.contains(&"important".to_string()));
    }

    #[test]
    fn test_real_music_collection() {
        let terms = parse_and_highlight("\"The Beatles\" (*.mp3|*.flac) !live").unwrap();

        assert!(terms.contains(&"the beatles".to_string()));
        assert!(terms.contains(&".mp3".to_string()));
        assert!(terms.contains(&".flac".to_string()));
        assert!(terms.contains(&"live".to_string()));
    }

    #[test]
    fn test_real_video_project() {
        let terms = parse_and_highlight("project *.mp4 size:>100mb !draft").unwrap();

        assert!(terms.contains(&"project".to_string()));
        assert!(terms.contains(&".mp4".to_string()));
        assert!(terms.contains(&"draft".to_string()));
    }

    #[test]
    fn test_real_source_code() {
        let terms = parse_and_highlight("(*.cpp|*.h) !test !backup").unwrap();

        assert!(terms.contains(&".cpp".to_string()));
        assert!(terms.contains(&".h".to_string()));
        assert!(terms.contains(&"test".to_string()));
        assert!(terms.contains(&"backup".to_string()));
    }

    #[test]
    fn test_real_photo_album() {
        let terms = parse_and_highlight("\"summer 2024\" (*.jpg|*.heic) folder:Photos").unwrap();

        assert!(terms.contains(&"summer 2024".to_string()));
        assert!(terms.contains(&".jpg".to_string()));
        assert!(terms.contains(&".heic".to_string()));
        assert!(terms.contains(&"photos".to_string()));
    }

    #[test]
    fn test_real_download_cleanup() {
        let terms = parse_and_highlight("folder:Downloads dm:lastmonth").unwrap();

        assert!(terms.contains(&"downloads".to_string()));
    }

    #[test]
    fn test_real_duplicate_finder() {
        let terms = parse_and_highlight("copy *(1)* *(2)*").unwrap();

        assert!(terms.contains(&"copy".to_string()));
    }

    #[test]
    fn test_real_version_search() {
        let terms = parse_and_highlight("app*v1* app*v2* !beta").unwrap();

        assert!(terms.contains(&"beta".to_string()));
    }

    #[test]
    fn test_real_archive_search() {
        let terms = parse_and_highlight("(*.zip|*.tar|*.gz|*.7z) archive").unwrap();

        assert!(terms.contains(&".zip".to_string()));
        assert!(terms.contains(&".tar".to_string()));
        assert!(terms.contains(&".gz".to_string()));
        assert!(terms.contains(&".7z".to_string()));
        assert!(terms.contains(&"archive".to_string()));
    }

    #[test]
    fn test_real_presentation_search() {
        let terms = parse_and_highlight("(*.ppt|*.pptx|*.key) presentation").unwrap();

        assert!(terms.contains(&".ppt".to_string()));
        assert!(terms.contains(&".pptx".to_string()));
        assert!(terms.contains(&".key".to_string()));
        assert!(terms.contains(&"presentation".to_string()));
    }

    #[test]
    fn test_real_spreadsheet_search() {
        let terms = parse_and_highlight("budget (*.xls|*.xlsx|*.csv)").unwrap();

        assert!(terms.contains(&"budget".to_string()));
        assert!(terms.contains(&".xls".to_string()));
        assert!(terms.contains(&".xlsx".to_string()));
        assert!(terms.contains(&".csv".to_string()));
    }

    #[test]
    fn test_real_ebook_search() {
        let terms = parse_and_highlight("(*.pdf|*.epub|*.mobi) !sample").unwrap();

        assert!(terms.contains(&".pdf".to_string()));
        assert!(terms.contains(&".epub".to_string()));
        assert!(terms.contains(&".mobi".to_string()));
        assert!(terms.contains(&"sample".to_string()));
    }

    #[test]
    fn test_real_installer_search() {
        let terms = parse_and_highlight("(*.exe|*.msi|*.dmg|*.pkg) setup install").unwrap();

        assert!(terms.contains(&".exe".to_string()));
        assert!(terms.contains(&".msi".to_string()));
        assert!(terms.contains(&".dmg".to_string()));
        assert!(terms.contains(&".pkg".to_string()));
        assert!(terms.contains(&"setup".to_string()));
        assert!(terms.contains(&"install".to_string()));
    }

    #[test]
    fn test_real_database_search() {
        let terms = parse_and_highlight("(*.db|*.sqlite|*.sql) database").unwrap();

        assert!(terms.contains(&".db".to_string()));
        assert!(terms.contains(&".sqlite".to_string()));
        assert!(terms.contains(&".sql".to_string()));
        assert!(terms.contains(&"database".to_string()));
    }

    #[test]
    fn test_real_font_search() {
        let terms = parse_and_highlight("(*.ttf|*.otf|*.woff) font").unwrap();

        assert!(terms.contains(&".ttf".to_string()));
        assert!(terms.contains(&".otf".to_string()));
        assert!(terms.contains(&".woff".to_string()));
        assert!(terms.contains(&"font".to_string()));
    }

    #[test]
    fn test_real_vector_graphics() {
        let terms = parse_and_highlight("(*.svg|*.ai|*.eps) logo icon").unwrap();

        assert!(terms.contains(&".svg".to_string()));
        assert!(terms.contains(&".ai".to_string()));
        assert!(terms.contains(&".eps".to_string()));
        assert!(terms.contains(&"logo".to_string()));
        assert!(terms.contains(&"icon".to_string()));
    }

    #[test]
    fn test_real_3d_model_search() {
        let terms = parse_and_highlight("(*.obj|*.fbx|*.blend) model").unwrap();

        assert!(terms.contains(&".obj".to_string()));
        assert!(terms.contains(&".fbx".to_string()));
        assert!(terms.contains(&".blend".to_string()));
        assert!(terms.contains(&"model".to_string()));
    }

    #[test]
    fn test_real_certificate_search() {
        let terms = parse_and_highlight("(*.crt|*.pem|*.key) certificate ssl").unwrap();

        assert!(terms.contains(&".crt".to_string()));
        assert!(terms.contains(&".pem".to_string()));
        assert!(terms.contains(&".key".to_string()));
        assert!(terms.contains(&"certificate".to_string()));
        assert!(terms.contains(&"ssl".to_string()));
    }

    #[test]
    fn test_real_docker_search() {
        let terms = parse_and_highlight("Dockerfile docker-compose.yml *.yaml").unwrap();

        assert!(terms.contains(&"dockerfile".to_string()));
        assert!(terms.contains(&"docker-compose.yml".to_string()));
        assert!(terms.contains(&".yaml".to_string()));
    }

    #[test]
    fn test_real_makefile_search() {
        let terms = parse_and_highlight("Makefile *.mk build").unwrap();

        assert!(terms.contains(&"makefile".to_string()));
        assert!(terms.contains(&".mk".to_string()));
        assert!(terms.contains(&"build".to_string()));
    }

    #[test]
    fn test_real_readme_search() {
        let terms = parse_and_highlight("README* *.md documentation").unwrap();

        assert!(terms.contains(&"readme".to_string()));
        assert!(terms.contains(&".md".to_string()));
        assert!(terms.contains(&"documentation".to_string()));
    }

    #[test]
    fn test_real_license_search() {
        let terms = parse_and_highlight("LICENSE* COPYING* *.txt").unwrap();

        assert!(terms.contains(&"license".to_string()));
        assert!(terms.contains(&"copying".to_string()));
        assert!(terms.contains(&".txt".to_string()));
    }

    #[test]
    fn test_real_gitignore_search() {
        let terms = parse_and_highlight("*ignore* .git*").unwrap();

        assert!(terms.contains(&"ignore".to_string()));
        assert!(terms.contains(&".git".to_string()));
    }

    #[test]
    fn test_real_node_modules() {
        let terms = parse_and_highlight("node_modules package*.json").unwrap();

        assert!(terms.contains(&"node_modules".to_string()));
        assert!(terms.contains(&".json".to_string()));
        assert!(terms.contains(&"package".to_string()));
    }

    #[test]
    fn test_real_python_project() {
        let terms = parse_and_highlight("*.py requirements.txt setup.py !__pycache__").unwrap();

        assert!(terms.contains(&".py".to_string()));
        assert!(terms.contains(&"requirements.txt".to_string()));
        assert!(terms.contains(&"setup.py".to_string()));
        assert!(terms.contains(&"__pycache__".to_string()));
    }

    #[test]
    fn test_real_rust_project() {
        let terms = parse_and_highlight("*.rs Cargo.toml !target").unwrap();

        assert!(terms.contains(&".rs".to_string()));
        assert!(terms.contains(&"cargo.toml".to_string()));
        assert!(terms.contains(&"target".to_string()));
    }

    #[test]
    fn test_real_java_project() {
        let terms = parse_and_highlight("*.java pom.xml build.gradle").unwrap();

        assert!(terms.contains(&".java".to_string()));
        assert!(terms.contains(&"pom.xml".to_string()));
        assert!(terms.contains(&"build.gradle".to_string()));
    }

    #[test]
    fn test_real_web_assets() {
        let terms = parse_and_highlight("(*.css|*.scss|*.less) style theme").unwrap();

        assert!(terms.contains(&".css".to_string()));
        assert!(terms.contains(&".scss".to_string()));
        assert!(terms.contains(&".less".to_string()));
        assert!(terms.contains(&"style".to_string()));
        assert!(terms.contains(&"theme".to_string()));
    }

    #[test]
    fn test_empty_result_set() {
        let terms = parse_and_highlight("").unwrap();
        assert_eq!(terms.len(), 0);
    }

    #[test]
    fn test_ordering_alphabetical() {
        let terms = parse_and_highlight("zebra apple monkey banana").unwrap();
        assert_eq!(terms, vec!["apple", "banana", "monkey", "zebra"]);
    }

    #[test]
    fn test_btreeset_ordering() {
        let terms = parse_and_highlight("zzz aaa mmm bbb").unwrap();
        assert_eq!(terms[0], "aaa");
        assert_eq!(terms[1], "bbb");
        assert_eq!(terms[2], "mmm");
        assert_eq!(terms[3], "zzz");
    }
}
