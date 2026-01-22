#![allow(clippy::too_many_lines)]

mod prelude {
    pub(super) use crate::SearchCache;
    pub(super) use fswalk::NodeFileType;
    pub(super) use jiff::Timestamp;
    pub(super) use search_cancel::CancellationToken;
    pub(super) use std::{fs, path::PathBuf};
    pub(super) use tempdir::TempDir;
}

mod support;

mod cache_flow;
mod date_edges;
mod date_keywords;
mod date_volume;
mod integration_filters;
mod query_logic;
mod size_filters;
mod traversal;
mod type_filters;
mod wildcard_star;
