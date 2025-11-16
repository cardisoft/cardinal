use query_segmentation::Segment;
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
    Plain { kind: SegmentKind, needle: String },
    Regex { regex: Regex },
}

impl SegmentMatcher {
    pub(crate) fn matches(&self, candidate: &str) -> bool {
        match self {
            SegmentMatcher::Plain { kind, needle } => match kind {
                SegmentKind::Substr => candidate.contains(needle),
                SegmentKind::Prefix => candidate.starts_with(needle),
                SegmentKind::Suffix => candidate.ends_with(needle),
                SegmentKind::Exact => candidate == needle,
            },
            SegmentMatcher::Regex { regex } => regex.is_match(candidate),
        }
    }
}

pub(crate) fn build_segment_matchers(
    segments: &[Segment<'_>],
    options: SearchOptions,
) -> Result<Vec<SegmentMatcher>, regex::Error> {
    segments
        .iter()
        .map(|segment| {
            let kind = segment_kind(segment);
            let value = segment_value(segment);
            if options.case_insensitive {
                let base = regex::escape(value);
                let pattern = match kind {
                    SegmentKind::Substr => base,
                    SegmentKind::Prefix => format!("^(?:{base})"),
                    SegmentKind::Suffix => format!("(?:{base})$"),
                    SegmentKind::Exact => format!("^(?:{base})$"),
                };
                let mut builder = RegexBuilder::new(&pattern);
                builder.case_insensitive(options.case_insensitive);
                builder.build().map(|regex| SegmentMatcher::Regex { regex })
            } else {
                Ok(SegmentMatcher::Plain {
                    kind,
                    needle: value.to_string(),
                })
            }
        })
        .collect()
}

fn segment_kind(segment: &Segment<'_>) -> SegmentKind {
    match segment {
        Segment::Substr(_) => SegmentKind::Substr,
        Segment::Prefix(_) => SegmentKind::Prefix,
        Segment::Suffix(_) => SegmentKind::Suffix,
        Segment::Exact(_) => SegmentKind::Exact,
    }
}

fn segment_value<'s>(segment: &Segment<'s>) -> &'s str {
    match segment {
        Segment::Substr(value)
        | Segment::Prefix(value)
        | Segment::Suffix(value)
        | Segment::Exact(value) => value,
    }
}
