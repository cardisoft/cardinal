use query_segmentation::{Segment, SegmentConcrete};
use regex::{Regex, RegexBuilder};

#[derive(Debug, Clone, Copy, Default)]
pub struct SearchOptions {
    pub case_insensitive: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum SegmentKind {
    Substr,
    Prefix,
    Suffix,
    Exact,
}

#[derive(Clone, Debug)]
pub(crate) enum SegmentMatcher {
    Concrete(SegmentMatcherConcrete),
    Star,
    GlobStar,
}

#[derive(Clone, Debug)]
pub(crate) enum SegmentMatcherConcrete {
    Plain { kind: SegmentKind, needle: String },
    Regex { regex: Regex },
}

impl SegmentMatcherConcrete {
    pub(crate) fn matches(&self, candidate: &str) -> bool {
        match self {
            SegmentMatcherConcrete::Plain { kind, needle } => match kind {
                SegmentKind::Substr => candidate.contains(needle),
                SegmentKind::Prefix => candidate.starts_with(needle),
                SegmentKind::Suffix => candidate.ends_with(needle),
                SegmentKind::Exact => candidate == needle,
            },
            SegmentMatcherConcrete::Regex { regex } => regex.is_match(candidate),
        }
    }
}

/// Splits a segment value into (has_unescaped_wildcard, literal_text).
///
/// `\*`, `\?` and `\\` denote literal characters (the query preprocessor emits `\*` / `\?` for
/// quoted wildcards such as `"*.rs"` and `\\` for every literal backslash); any other
/// backslash is an ordinary character.
fn unescape_wildcards(value: &str) -> (bool, String) {
    let mut has_wildcard = false;
    let mut literal = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' if matches!(chars.peek(), Some('*') | Some('?') | Some('\\')) => {
                literal.push(chars.next().expect("peeked value exists"));
            }
            '*' | '?' => {
                has_wildcard = true;
                literal.push(ch);
            }
            _ => literal.push(ch),
        }
    }
    (has_wildcard, literal)
}

fn wildcard_to_regex(pattern: &str) -> String {
    let mut regex = String::with_capacity(pattern.len() + 3);
    let mut buf = [0u8; 4];
    regex.push('^');
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' if matches!(chars.peek(), Some('*') | Some('?') | Some('\\')) => {
                // Escaped wildcard or backslash: match the character literally.
                let literal = chars.next().expect("peeked value exists");
                regex.push_str(&regex::escape(literal.encode_utf8(&mut buf)));
            }
            '*' => regex.push_str(".*"),
            '?' => regex.push('.'),
            _ => {
                let encoded = ch.encode_utf8(&mut buf);
                regex.push_str(&regex::escape(encoded));
            }
        }
    }
    regex.push('$');
    regex
}

pub(crate) fn build_segment_matchers(
    segments: &[Segment<'_>],
    options: SearchOptions,
) -> Result<Vec<SegmentMatcher>, regex::Error> {
    segments
        .iter()
        .map(|segment| match segment {
            Segment::GlobStar => Ok(SegmentMatcher::GlobStar),
            Segment::Star => Ok(SegmentMatcher::Star),
            Segment::Concrete(concrete) => build_concrete_segment_matcher(concrete, options),
        })
        .collect()
}

fn build_concrete_segment_matcher(
    segment: &SegmentConcrete<'_>,
    options: SearchOptions,
) -> Result<SegmentMatcher, regex::Error> {
    let kind = segment_kind(segment);
    let value = segment_value(segment);
    // Only unescaped `*` / `?` make this a wildcard segment; `\*` / `\?` come from quoted
    // tokens and must match literally.
    let (is_wildcard, literal) = unescape_wildcards(value);
    if options.case_insensitive || is_wildcard {
        let pattern = if is_wildcard {
            // Wildcard pattern is /exact/ by default, so we don't need to
            // adjust it based on SegmentKind.
            wildcard_to_regex(value)
        } else {
            let base = regex::escape(&literal);
            match kind {
                SegmentKind::Substr => base,
                SegmentKind::Prefix => format!("^(?:{base})"),
                SegmentKind::Suffix => format!("(?:{base})$"),
                SegmentKind::Exact => format!("^(?:{base})$"),
            }
        };
        let mut builder = RegexBuilder::new(&pattern);
        builder.case_insensitive(options.case_insensitive);
        builder
            .build()
            .map(|regex| SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { regex }))
    } else {
        Ok(SegmentMatcher::Concrete(SegmentMatcherConcrete::Plain {
            kind,
            needle: literal,
        }))
    }
}

fn segment_kind(segment: &SegmentConcrete<'_>) -> SegmentKind {
    match segment {
        SegmentConcrete::Substr(_) => SegmentKind::Substr,
        SegmentConcrete::Prefix(_) => SegmentKind::Prefix,
        SegmentConcrete::Suffix(_) => SegmentKind::Suffix,
        SegmentConcrete::Exact(_) => SegmentKind::Exact,
    }
}

fn segment_value<'s>(segment: &SegmentConcrete<'s>) -> &'s str {
    match segment {
        SegmentConcrete::Substr(value)
        | SegmentConcrete::Prefix(value)
        | SegmentConcrete::Suffix(value)
        | SegmentConcrete::Exact(value) => value,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SearchOptions, SegmentKind, SegmentMatcher, SegmentMatcherConcrete, build_segment_matchers,
        segment_kind, segment_value, wildcard_to_regex,
    };
    use query_segmentation::{Segment, SegmentConcrete};

    // --- wildcard_to_regex edge cases ---

    #[test]
    fn wildcard_glob_tokens_are_converted() {
        assert_eq!(wildcard_to_regex("foo*bar?baz"), "^foo.*bar.baz$");
    }

    #[test]
    fn wildcard_only_star() {
        assert_eq!(wildcard_to_regex("*"), "^.*$");
    }

    #[test]
    fn wildcard_only_question() {
        assert_eq!(wildcard_to_regex("?"), "^.$");
    }

    #[test]
    fn wildcard_mixed_starts_ends() {
        assert_eq!(wildcard_to_regex("*foo?"), "^.*foo.$");
        assert_eq!(wildcard_to_regex("?foo*"), "^.foo.*$");
    }

    #[test]
    fn wildcard_escapes_regex_characters() {
        assert_eq!(wildcard_to_regex("file.+(1)"), "^file\\.\\+\\(1\\)$");
    }

    #[test]
    fn wildcard_escapes_unicode() {
        // '?' acts as a wildcard (single character), not a literal, so it becomes '.' in the regex.
        assert_eq!(wildcard_to_regex("café*(a?)"), "^café.*\\(a.\\)$");
    }

    #[test]
    fn wildcard_empty_string() {
        assert_eq!(wildcard_to_regex(""), "^$");
    }

    // --- segment_kind mapping ---

    #[test]
    fn segment_kind_mapping() {
        let substr = Segment::substr("abc");
        assert!(matches!(
            segment_kind(expect_concrete(&substr)),
            SegmentKind::Substr
        ));
        let prefix = Segment::prefix("abc");
        assert!(matches!(
            segment_kind(expect_concrete(&prefix)),
            SegmentKind::Prefix
        ));
        let suffix = Segment::suffix("abc");
        assert!(matches!(
            segment_kind(expect_concrete(&suffix)),
            SegmentKind::Suffix
        ));
        let exact = Segment::exact("abc");
        assert!(matches!(
            segment_kind(expect_concrete(&exact)),
            SegmentKind::Exact
        ));
    }

    #[test]
    fn segment_value_extraction() {
        let substr = Segment::substr("abc");
        assert_eq!(segment_value(expect_concrete(&substr)), "abc");
        let prefix = Segment::prefix("def");
        assert_eq!(segment_value(expect_concrete(&prefix)), "def");
        let suffix = Segment::suffix("ghi");
        assert_eq!(segment_value(expect_concrete(&suffix)), "ghi");
        let exact = Segment::exact("jkl");
        assert_eq!(segment_value(expect_concrete(&exact)), "jkl");
    }

    #[test]
    fn globstar_segment_builds_globstar_matcher() {
        let segments = [Segment::GlobStar, Segment::prefix("foo")];
        let opts = SearchOptions::default();
        let matchers = build_segment_matchers(&segments, opts).expect("ok");
        assert!(matches!(matchers[0], SegmentMatcher::GlobStar));
        assert!(matches!(
            matchers[1],
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { .. })
                | SegmentMatcher::Concrete(SegmentMatcherConcrete::Plain { .. })
        ));
    }

    #[test]
    fn star_segment_builds_star_matcher() {
        let segments = [Segment::Star];
        let opts = SearchOptions::default();
        let matchers = build_segment_matchers(&segments, opts).expect("ok");
        assert!(matches!(matchers[0], SegmentMatcher::Star));
    }

    // --- build_segment_matchers (plain, no wildcard, case-sensitive) ---

    #[test]
    fn build_plain_matchers_without_case_insensitive() {
        let segments = [
            Segment::substr("mid"),
            Segment::prefix("pre"),
            Segment::suffix("suf"),
            Segment::exact("exact"),
        ];
        let opts = SearchOptions {
            case_insensitive: false,
        };
        let matchers = build_segment_matchers(&segments, opts).expect("ok");
        assert_eq!(matchers.len(), 4);
        // All should be Plain
        for (m, s) in matchers.iter().zip(segments.iter()) {
            match m {
                SegmentMatcher::Concrete(SegmentMatcherConcrete::Plain { kind, needle }) => {
                    assert_eq!(needle, segment_value(expect_concrete(s)));
                    assert_eq!(*kind as u8, segment_kind(expect_concrete(s)) as u8);
                }
                _ => panic!("Expected Plain matcher"),
            }
        }
    }

    // --- build_segment_matchers with case_insensitive true (regex) ---

    #[test]
    fn build_regex_matchers_case_insensitive() {
        let segments = [
            Segment::substr("mid"),
            Segment::prefix("pre"),
            Segment::suffix("suf"),
            Segment::exact("exact"),
        ];
        let opts = SearchOptions {
            case_insensitive: true,
        };
        let matchers = build_segment_matchers(&segments, opts).expect("ok");
        assert_eq!(matchers.len(), 4);
        let patterns: Vec<_> = matchers
            .iter()
            .map(|m| match m {
                SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { regex }) => {
                    regex.as_str().to_string()
                }
                _ => panic!("Expected Regex matcher"),
            })
            .collect();
        assert_eq!(patterns[0], "mid"); // substr
        assert_eq!(patterns[1], "^(?:pre)"); // prefix
        assert_eq!(patterns[2], "(?:suf)$"); // suffix
        assert_eq!(patterns[3], "^(?:exact)$"); // exact
    }

    // --- wildcard forces regex even when case_sensitive ---

    #[test]
    fn wildcard_forces_regex_exact_anchor() {
        let segments = [Segment::exact("foo*bar?baz")];
        let opts = SearchOptions {
            case_insensitive: false,
        };
        let matchers = build_segment_matchers(&segments, opts).expect("ok");
        assert_eq!(matchers.len(), 1);
        match &matchers[0] {
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { regex }) => {
                assert_eq!(regex.as_str(), "^foo.*bar.baz$");
            }
            _ => panic!("Expected regex for wildcard segment"),
        }
    }

    #[test]
    fn wildcard_case_sensitive() {
        let segments = [Segment::substr("A*B")];
        let opts = SearchOptions {
            case_insensitive: false,
        };
        let matchers = build_segment_matchers(&segments, opts).expect("ok");
        match &matchers[0] {
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { regex }) => {
                assert!(!regex.is_match("aXXb"));
                assert!(regex.is_match("AXXB"));
            }
            _ => panic!("Expected regex matcher"),
        }
    }

    #[test]
    fn wildcard_case_insensitive() {
        let segments = [Segment::substr("A*B")];
        let opts = SearchOptions {
            case_insensitive: true,
        };
        let matchers = build_segment_matchers(&segments, opts).expect("ok");
        match &matchers[0] {
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { regex }) => {
                assert!(regex.is_match("aXXb"));
                assert!(regex.is_match("AXXB"));
            }
            _ => panic!("Expected regex matcher"),
        }
    }

    // --- SegmentMatcher.matches for Plain variants ---

    #[test]
    fn plain_substr_matches() {
        let m = SegmentMatcherConcrete::Plain {
            kind: SegmentKind::Substr,
            needle: "abc".into(),
        };
        assert!(m.matches("zzzabczzz"));
        assert!(!m.matches("abz"));
    }

    #[test]
    fn plain_prefix_matches() {
        let m = SegmentMatcherConcrete::Plain {
            kind: SegmentKind::Prefix,
            needle: "start".into(),
        };
        assert!(m.matches("start_of_line"));
        assert!(!m.matches("line_start"));
    }

    #[test]
    fn plain_suffix_matches() {
        let m = SegmentMatcherConcrete::Plain {
            kind: SegmentKind::Suffix,
            needle: "tail".into(),
        };
        assert!(m.matches("segment_tail"));
        assert!(!m.matches("tail_segment"));
    }

    #[test]
    fn plain_exact_matches() {
        let m = SegmentMatcherConcrete::Plain {
            kind: SegmentKind::Exact,
            needle: "only".into(),
        };
        assert!(m.matches("only"));
        assert!(!m.matches("only1"));
    }

    // --- SegmentMatcher.matches for Regex variants ---

    #[test]
    fn regex_substr_equivalent() {
        let segments = [Segment::substr("abc")];
        let opts = SearchOptions {
            case_insensitive: true,
        };
        let m = build_segment_matchers(&segments, opts).unwrap().remove(0);
        match m {
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { regex }) => {
                assert!(regex.is_match("zzzAbCzzz"));
            }
            _ => panic!("Expected regex matcher"),
        }
    }

    #[test]
    fn regex_prefix_equivalent() {
        let segments = [Segment::prefix("abc")];
        let opts = SearchOptions {
            case_insensitive: true,
        };
        let m = build_segment_matchers(&segments, opts).unwrap().remove(0);
        match m {
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { regex }) => {
                assert!(regex.is_match("AbCzzz"));
                assert!(!regex.is_match("zzzabc"));
            }
            _ => panic!("Expected regex matcher"),
        }
    }

    #[test]
    fn regex_suffix_equivalent() {
        let segments = [Segment::suffix("abc")];
        let opts = SearchOptions {
            case_insensitive: true,
        };
        let m = build_segment_matchers(&segments, opts).unwrap().remove(0);
        match m {
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { regex }) => {
                assert!(regex.is_match("zzzAbC"));
                assert!(!regex.is_match("AbCzzz"));
            }
            _ => panic!("Expected regex matcher"),
        }
    }

    #[test]
    fn regex_exact_equivalent() {
        let segments = [Segment::exact("abc")];
        let opts = SearchOptions {
            case_insensitive: true,
        };
        let m = build_segment_matchers(&segments, opts).unwrap().remove(0);
        match m {
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { regex }) => {
                assert!(regex.is_match("AbC"));
                assert!(!regex.is_match("xabc"));
            }
            _ => panic!("Expected regex matcher"),
        }
    }

    // --- Mixed segments producing both Plain and Regex ---

    #[test]
    fn mixed_segments_plain_and_regex() {
        let segments = [
            Segment::substr("abc"),   // plain
            Segment::prefix("pre"),   // plain
            Segment::suffix("*wild"), // wildcard => regex
            Segment::exact("ex?act"), // wildcard => regex
        ];
        let opts = SearchOptions {
            case_insensitive: false,
        };
        let matchers = build_segment_matchers(&segments, opts).unwrap();
        assert_eq!(matchers.len(), 4);
        assert!(matches!(
            matchers[0],
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Plain { .. })
        ));
        assert!(matches!(
            matchers[1],
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Plain { .. })
        ));
        assert!(matches!(
            matchers[2],
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { .. })
        ));
        assert!(matches!(
            matchers[3],
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { .. })
        ));
    }

    #[test]
    fn mixed_segments_all_regex_when_case_insensitive() {
        let segments = [
            Segment::substr("abc"),
            Segment::prefix("pre"),
            Segment::suffix("suf"),
            Segment::exact("exact"),
        ];
        let opts = SearchOptions {
            case_insensitive: true,
        };
        let matchers = build_segment_matchers(&segments, opts).unwrap();
        for m in matchers {
            assert!(matches!(
                m,
                SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { .. })
            ));
        }
    }

    // --- Wildcard escaping ensures metacharacters are literal ---

    #[test]
    fn wildcard_metacharacters_literal() {
        let segments = [Segment::exact("a+b*(c?)")];
        let opts = SearchOptions {
            case_insensitive: false,
        };
        let matchers = build_segment_matchers(&segments, opts).unwrap();
        match &matchers[0] {
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { regex }) => {
                // '?' is treated as wildcard -> '.'
                assert_eq!(regex.as_str(), "^a\\+b.*\\(c.\\)$");
                assert!(regex.is_match("a+bZZZ(c?)"));
                assert!(!regex.is_match("abZZZ(c?)"));
            }
            _ => panic!("Expected regex"),
        }
    }

    // --- Unicode handling inside regex (no case folding when insensitive=false) ---

    #[test]
    fn unicode_case_sensitive() {
        let segments = [Segment::substr("Café")];
        let opts = SearchOptions {
            case_insensitive: false,
        };
        let matchers = build_segment_matchers(&segments, opts).unwrap();
        match &matchers[0] {
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Plain { needle, .. }) => {
                assert_eq!(needle, "Café");
            }
            _ => panic!("Expected plain matcher"),
        }
    }

    #[test]
    fn unicode_case_insensitive() {
        let segments = [Segment::exact("Café")];
        let opts = SearchOptions {
            case_insensitive: true,
        };
        let matchers = build_segment_matchers(&segments, opts).unwrap();
        match &matchers[0] {
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { regex }) => {
                assert!(regex.is_match("café"));
                // Basic ASCII case fold works; regex crate may not fold é to É on all platforms, so we only check lowercase.
            }
            _ => panic!("Expected regex matcher"),
        }
    }

    // --- Ensure anchoring semantics for prefix/suffix/exact patterns ---

    #[test]
    fn anchoring_prefix_suffix_exact_patterns() {
        let segments = [
            Segment::prefix("pre"),
            Segment::suffix("suf"),
            Segment::exact("all"),
        ];
        let opts = SearchOptions {
            case_insensitive: true,
        };
        let matchers = build_segment_matchers(&segments, opts).unwrap();
        assert_eq!(matchers.len(), 3);
        match &matchers[0] {
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { regex }) => {
                assert!(regex.as_str().starts_with("^(?:"))
            }
            _ => panic!("regex expected"),
        }
        match &matchers[1] {
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { regex }) => {
                assert!(regex.as_str().ends_with(")$"))
            }
            _ => panic!("regex expected"),
        }
        match &matchers[2] {
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { regex }) => {
                assert!(regex.as_str().starts_with("^(?:"));
                assert!(regex.as_str().ends_with(")$"));
            }
            _ => panic!("regex expected"),
        }
    }

    // --- Large pattern capacity (sanity, does not panic) ---

    #[test]
    fn large_pattern_build() {
        let long = "a".repeat(10_000);
        let segments = [Segment::exact(&long)];
        let opts = SearchOptions {
            case_insensitive: true,
        };
        let matchers = build_segment_matchers(&segments, opts).unwrap();
        assert_eq!(matchers.len(), 1);
        match &matchers[0] {
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { regex }) => {
                assert!(regex.as_str().starts_with("^(?:"));
            }
            _ => panic!("Expected regex"),
        }
    }

    // --- Multiple wildcards in one segment ---

    #[test]
    fn multiple_wildcards_match() {
        let segments = [Segment::exact("a*b*c?d")];
        let opts = SearchOptions {
            case_insensitive: false,
        };
        let matchers = build_segment_matchers(&segments, opts).unwrap();
        match &matchers[0] {
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Regex { regex }) => {
                assert!(regex.is_match("aZZbYYcXd"));
                assert!(!regex.is_match("abYcXdX"));
            }
            _ => panic!("Expected regex"),
        }
    }

    // --- Ensure plain Substr acts like contains ---

    #[test]
    fn substr_plain_contains_behavior() {
        let segments = [Segment::substr("mid")];
        let opts = SearchOptions {
            case_insensitive: false,
        };
        let matchers = build_segment_matchers(&segments, opts).unwrap();
        match &matchers[0] {
            SegmentMatcher::Concrete(SegmentMatcherConcrete::Plain { needle, .. }) => {
                assert_eq!(needle, "mid");
                assert!(
                    SegmentMatcherConcrete::Plain {
                        kind: SegmentKind::Substr,
                        needle: needle.clone()
                    }
                    .matches("xxmidxx")
                );
            }
            _ => panic!("Expected plain matcher"),
        }
    }

    // --- Escaped wildcards (from quoted tokens) are literal ---

    #[test]
    fn escaped_wildcards_are_literal() {
        use super::unescape_wildcards;
        assert_eq!(unescape_wildcards("\\*.rs"), (false, "*.rs".to_string()));
        assert_eq!(unescape_wildcards("a\\?b"), (false, "a?b".to_string()));
        assert_eq!(unescape_wildcards("*.rs"), (true, "*.rs".to_string()));
        // Mixed: one literal star, one real wildcard.
        assert_eq!(unescape_wildcards("\\*x*"), (true, "*x*".to_string()));
        // Other backslashes are ordinary characters.
        assert_eq!(unescape_wildcards("a\\b"), (false, "a\\b".to_string()));
        // `\\` is a literal backslash; a wildcard after it is still a wildcard
        // (the preprocessor emits this for a user-typed `foo\*.txt`).
        assert_eq!(
            unescape_wildcards("foo\\\\*.txt"),
            (true, "foo\\*.txt".to_string())
        );
        let regex = regex::Regex::new(&super::wildcard_to_regex("foo\\\\*.txt")).unwrap();
        assert!(regex.is_match("foo\\report.txt"));
        assert!(!regex.is_match("foo*.txt"));
        assert!(!regex.is_match("foo_x.txt"));
    }

    #[test]
    fn quoted_star_matches_literally_in_plain_and_regex_modes() {
        // Case-sensitive: plain substring matcher with the unescaped needle.
        let segments = [Segment::substr("\\*.rs")];
        let matchers = build_segment_matchers(
            &segments,
            SearchOptions {
                case_insensitive: false,
            },
        )
        .unwrap();
        match &matchers[0] {
            SegmentMatcher::Concrete(matcher @ SegmentMatcherConcrete::Plain { needle, .. }) => {
                assert_eq!(needle, "*.rs");
                assert!(matcher.matches("weird*.rs"));
                assert!(!matcher.matches("main.rs"));
            }
            other => panic!("Expected plain matcher, got {other:?}"),
        }

        // Case-insensitive: regex matcher, star still literal.
        let matchers = build_segment_matchers(
            &segments,
            SearchOptions {
                case_insensitive: true,
            },
        )
        .unwrap();
        match &matchers[0] {
            SegmentMatcher::Concrete(matcher @ SegmentMatcherConcrete::Regex { .. }) => {
                assert!(matcher.matches("WEIRD*.RS"));
                assert!(!matcher.matches("main.rs"));
            }
            other => panic!("Expected regex matcher, got {other:?}"),
        }

        // Mixed: literal star followed by a real wildcard.
        let regex = super::wildcard_to_regex("\\*x*");
        let regex = regex::Regex::new(&regex).unwrap();
        assert!(regex.is_match("*xanything"));
        assert!(!regex.is_match("yxanything"));
    }

    fn expect_concrete<'a>(segment: &'a Segment<'a>) -> &'a SegmentConcrete<'a> {
        match segment {
            Segment::Concrete(concrete) => concrete,
            Segment::GlobStar => panic!("expected concrete segment"),
            Segment::Star => panic!("expected concrete segment"),
        }
    }
}
