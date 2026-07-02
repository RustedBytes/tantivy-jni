mod document;
mod error;
mod jni_bridge;
mod lifecycle;
mod model;
mod registry;
mod schema;
mod searching;
mod validation;
mod writing;

pub(crate) use error::{NativeError, NativeResult};
pub(crate) use lifecycle::{close_index, open_index};
pub(crate) use searching::search;
pub(crate) use writing::{
    add_documents, commit, commit_and_refresh, delete_all_documents, delete_term, delete_query, refresh, schema_info,
};

pub use jni_bridge::*;

#[cfg(test)]
mod tests;
