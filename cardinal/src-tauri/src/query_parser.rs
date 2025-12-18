use cardinal_syntax::{Expr, FilterKind, Term, parse_query};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ParsedExpr {
    Empty,
    Term { term: ParsedTerm },
    Not { inner: Box<ParsedExpr> },
    And { parts: Vec<ParsedExpr> },
    Or { parts: Vec<ParsedExpr> },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ParsedTerm {
    Word {
        text: String,
    },
    Phrase {
        text: String,
    },
    Regex {
        pattern: String,
    },
    Filter {
        kind: String,
        argument: Option<String>,
    },
}

impl From<Expr> for ParsedExpr {
    fn from(expr: Expr) -> Self {
        match expr {
            Expr::Empty => ParsedExpr::Empty,
            Expr::Term(term) => ParsedExpr::Term { term: term.into() },
            Expr::Not(inner) => ParsedExpr::Not {
                inner: Box::new((*inner).into()),
            },
            Expr::And(parts) => ParsedExpr::And {
                parts: parts.into_iter().map(Into::into).collect(),
            },
            Expr::Or(parts) => ParsedExpr::Or {
                parts: parts.into_iter().map(Into::into).collect(),
            },
        }
    }
}

impl From<Term> for ParsedTerm {
    fn from(term: Term) -> Self {
        match term {
            Term::Word(text) => ParsedTerm::Word { text },
            Term::Phrase(text) => ParsedTerm::Phrase { text },
            Term::Regex(pattern) => ParsedTerm::Regex { pattern },
            Term::Filter(filter) => {
                let kind = filter_kind_to_string(&filter.kind);
                let argument = filter.argument.map(|arg| arg.raw);
                ParsedTerm::Filter { kind, argument }
            }
        }
    }
}

fn filter_kind_to_string(kind: &FilterKind) -> String {
    match kind {
        FilterKind::File => "file".to_string(),
        FilterKind::Folder => "folder".to_string(),
        FilterKind::Ext => "ext".to_string(),
        FilterKind::Type => "type".to_string(),
        FilterKind::Audio => "audio".to_string(),
        FilterKind::Video => "video".to_string(),
        FilterKind::Doc => "doc".to_string(),
        FilterKind::Exe => "exe".to_string(),
        FilterKind::Size => "size".to_string(),
        FilterKind::DateModified => "dm".to_string(),
        FilterKind::DateCreated => "dc".to_string(),
        FilterKind::DateAccessed => "da".to_string(),
        FilterKind::DateRun => "dr".to_string(),
        FilterKind::Parent => "parent".to_string(),
        FilterKind::InFolder => "infolder".to_string(),
        FilterKind::NoSubfolders => "nosubfolders".to_string(),
        FilterKind::Child => "child".to_string(),
        FilterKind::Attribute => "attrib".to_string(),
        FilterKind::AttributeDuplicate => "attribdupe".to_string(),
        FilterKind::DateModifiedDuplicate => "dmdupe".to_string(),
        FilterKind::Duplicate => "dupe".to_string(),
        FilterKind::NamePartDuplicate => "namepartdupe".to_string(),
        FilterKind::SizeDuplicate => "sizedupe".to_string(),
        FilterKind::Artist => "artist".to_string(),
        FilterKind::Album => "album".to_string(),
        FilterKind::Title => "title".to_string(),
        FilterKind::Genre => "genre".to_string(),
        FilterKind::Year => "year".to_string(),
        FilterKind::Track => "track".to_string(),
        FilterKind::Comment => "comment".to_string(),
        FilterKind::Width => "width".to_string(),
        FilterKind::Height => "height".to_string(),
        FilterKind::Dimensions => "dimensions".to_string(),
        FilterKind::Orientation => "orientation".to_string(),
        FilterKind::BitDepth => "bitdepth".to_string(),
        FilterKind::CaseSensitive => "case".to_string(),
        FilterKind::Tag => "tag".to_string(),
        FilterKind::Content => "content".to_string(),
        FilterKind::NoWholeFilename => "nowholefilename".to_string(),
        FilterKind::Custom(name) => name.clone(),
    }
}

#[tauri::command]
pub async fn parse_search_query(query: String) -> Result<ParsedExpr, String> {
    parse_query(&query)
        .map(|parsed| parsed.expr.into())
        .map_err(|e: cardinal_syntax::ParseError| e.to_string())
}
