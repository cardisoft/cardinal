#![feature(str_from_raw_parts)]
mod cache;
mod file_nodes;
mod highlight;
mod metadata_cache;
mod name_index;
mod persistent;
mod query;
mod query_preprocessor;
mod segment;
mod slab;
mod slab_node;
mod type_and_size;

pub use cache::*;
pub use file_nodes::*;
pub use fswalk::WalkData;
pub use metadata_cache::*;
pub use name_index::*;
pub use persistent::*;
pub use segment::*;
pub use slab::*;
pub use slab_node::*;
pub use type_and_size::*;

#[cfg(test)]
mod tests;
