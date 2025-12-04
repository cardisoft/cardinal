// `elloworl` => Substr("elloworl")
// `/root` => Prefix("root")
// `root/` => Suffix("root")
// `/root/` => Exact("root")
// `/root/bar` => Exact("root"), Prefix("bar")
// `/root/bar/kksk` => Exact("root"), Exact("bar"), Prefix("kksk")
// `foo/bar/kks` => Suffix("foo"), Exact("bar"), Prefix("kks")
// `gaea/lil/bee/` => Suffix("gaea"), Exact("lil"), Exact("bee")
// `bab/bob/` => Suffix("bab"), Exact("bob")
// `/byb/huh/good/` => Exact("byb"), Exact("huh"), Exact("good")
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Segment<'s> {
    Concrete(SegmentConcrete<'s>),
    /// Globstar (`**`) that may span multiple path segments.
    GlobStar,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentConcrete<'s> {
    Substr(&'s str),
    Prefix(&'s str),
    Suffix(&'s str),
    Exact(&'s str),
}

impl SegmentConcrete<'_> {
    pub fn as_value(&self) -> &str {
        match self {
            SegmentConcrete::Substr(s)
            | SegmentConcrete::Prefix(s)
            | SegmentConcrete::Suffix(s)
            | SegmentConcrete::Exact(s) => s,
        }
    }
}

/// Process path-query string into segments.
pub fn query_segmentation(query: &str) -> Vec<Segment<'_>> {
    #[derive(Clone, Copy)]
    enum State {
        Substr,
        Prefix,
        Suffix,
        Exact,
    }
    let left_close = query.starts_with('/');
    let right_close = query.ends_with('/');
    let query = query.trim_start_matches('/').trim_end_matches('/');
    // Filter out ["", "/", "///", ..]
    if query.is_empty() {
        return vec![];
    }
    let segments: Vec<_> = query.split('/').collect();
    // After trimming leading and trailing slashes, if segments contains empty string,
    // it means there are multiple consecutive slashes inserted in the original query.
    // In this case, we should return an empty vector.
    // e.g. "/a//b/" => ["a", "", "b"]
    if segments.contains(&"") {
        return vec![];
    }
    let len = segments.len();
    let states = {
        let mut states: Vec<_> = vec![State::Exact; len];
        assert_ne!(len, 0);
        if len == 1 {
            if !left_close || !right_close {
                if !left_close && !right_close {
                    states[0] = State::Substr;
                } else if !left_close {
                    states[0] = State::Suffix;
                } else if !right_close {
                    states[0] = State::Prefix;
                }
            }
        } else {
            if !left_close {
                states[0] = State::Suffix;
            }
            if !right_close {
                states[len - 1] = State::Prefix;
            }
        }
        states
    };
    states
        .into_iter()
        .zip(segments)
        .map(|(state, segment)| {
            if segment == "**" {
                Segment::GlobStar
            } else {
                let concrete = match state {
                    State::Substr => SegmentConcrete::Substr(segment),
                    State::Prefix => SegmentConcrete::Prefix(segment),
                    State::Suffix => SegmentConcrete::Suffix(segment),
                    State::Exact => SegmentConcrete::Exact(segment),
                };
                Segment::Concrete(concrete)
            }
        })
        .collect()
}

impl<'s> Segment<'s> {
    pub fn substr(value: &'s str) -> Self {
        Segment::Concrete(SegmentConcrete::Substr(value))
    }

    pub fn prefix(value: &'s str) -> Self {
        Segment::Concrete(SegmentConcrete::Prefix(value))
    }

    pub fn suffix(value: &'s str) -> Self {
        Segment::Concrete(SegmentConcrete::Suffix(value))
    }

    pub fn exact(value: &'s str) -> Self {
        Segment::Concrete(SegmentConcrete::Exact(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_segmentation() {
        assert_eq!(
            query_segmentation("elloworl"),
            vec![Segment::substr("elloworl")]
        );
        assert_eq!(query_segmentation("**"), vec![Segment::GlobStar]);
        assert_eq!(query_segmentation("/root"), vec![Segment::prefix("root")]);
        assert_eq!(query_segmentation("root/"), vec![Segment::suffix("root")]);
        assert_eq!(query_segmentation("/root/"), vec![Segment::exact("root")]);
        assert_eq!(
            query_segmentation("/root/bar"),
            vec![Segment::exact("root"), Segment::prefix("bar")]
        );
        assert_eq!(
            query_segmentation("/root/bar/kksk"),
            vec![
                Segment::exact("root"),
                Segment::exact("bar"),
                Segment::prefix("kksk")
            ]
        );
        assert_eq!(
            query_segmentation("foo/bar/kks"),
            vec![
                Segment::suffix("foo"),
                Segment::exact("bar"),
                Segment::prefix("kks")
            ]
        );
        assert_eq!(
            query_segmentation("foo/**/bar"),
            vec![
                Segment::suffix("foo"),
                Segment::GlobStar,
                Segment::prefix("bar")
            ]
        );
        assert_eq!(
            query_segmentation("gaea/lil/bee/"),
            vec![
                Segment::suffix("gaea"),
                Segment::exact("lil"),
                Segment::exact("bee")
            ]
        );
        assert_eq!(
            query_segmentation("bab/bob/"),
            vec![Segment::suffix("bab"), Segment::exact("bob")]
        );
        assert_eq!(
            query_segmentation("/byb/huh/good/"),
            vec![
                Segment::exact("byb"),
                Segment::exact("huh"),
                Segment::exact("good")
            ]
        );
    }

    #[test]
    fn test_query_segmentation_edge_cases() {
        // Empty string
        assert_eq!(query_segmentation(""), vec![]);

        // Single slash
        assert_eq!(query_segmentation("/"), vec![]);

        // Multiple slashes
        assert_eq!(query_segmentation("///"), vec![]);

        // Globstar mixing
        assert_eq!(
            query_segmentation("/**/foo"),
            vec![Segment::GlobStar, Segment::prefix("foo")]
        );
        assert_eq!(
            query_segmentation("foo/**"),
            vec![Segment::suffix("foo"), Segment::GlobStar]
        );

        // Leading and trailing slashes
        assert_eq!(query_segmentation("/a/"), vec![Segment::exact("a")]);

        // Single character
        assert_eq!(query_segmentation("a"), vec![Segment::substr("a")]);

        // Single character with slash
        assert_eq!(query_segmentation("/a"), vec![Segment::prefix("a")]);
        assert_eq!(query_segmentation("a/"), vec![Segment::suffix("a")]);

        // Mixed slashes and empty segments
        assert_eq!(query_segmentation("/a//b/"), vec![]);

        // Long string without slashes
        assert_eq!(
            query_segmentation("thisisaverylongstringwithoutslashes"),
            vec![Segment::substr("thisisaverylongstringwithoutslashes")]
        );

        // Long string with slashes
        assert_eq!(
            query_segmentation("/this/is/a/very/long/string/"),
            vec![
                Segment::exact("this"),
                Segment::exact("is"),
                Segment::exact("a"),
                Segment::exact("very"),
                Segment::exact("long"),
                Segment::exact("string")
            ]
        );

        // Two segments no leading/trailing slash => suffix + prefix
        assert_eq!(
            query_segmentation("foo/bar"),
            vec![Segment::suffix("foo"), Segment::prefix("bar")]
        );
        // Two segments trailing slash => suffix + exact
        assert_eq!(
            query_segmentation("foo/bar/"),
            vec![Segment::suffix("foo"), Segment::exact("bar")]
        );
        // Two segments leading slash => exact + prefix
        assert_eq!(
            query_segmentation("/foo/bar"),
            vec![Segment::exact("foo"), Segment::prefix("bar")]
        );
        // Unicode segments
        assert_eq!(
            query_segmentation("/报告/测试/"),
            vec![Segment::exact("报告"), Segment::exact("测试")]
        );
    }
}
