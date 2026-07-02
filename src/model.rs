use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tantivy::schema::{Field, Schema, TantivyDocument};
use tantivy::{Index, IndexReader, IndexWriter};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FieldKind {
    Text,
    String,
    I64,
    U64,
    F64,
    Bool,
    Bytes,
    Date,
    Json,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SchemaRequest {
    pub(crate) fields: Vec<FieldRequest>,
    #[serde(default, rename = "defaultSearchFields")]
    pub(crate) default_search_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FieldRequest {
    pub(crate) name: String,
    #[serde(rename = "type")]
    pub(crate) kind: FieldKind,
    #[serde(default = "default_true")]
    pub(crate) stored: bool,
    #[serde(default = "default_true")]
    pub(crate) indexed: bool,
    #[serde(default)]
    pub(crate) fast: bool,
    #[serde(default)]
    pub(crate) tokenizer: Option<TokenizerMode>,
    #[serde(default)]
    pub(crate) experimental: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpenOptions {
    #[serde(default = "default_true")]
    pub(crate) create: bool,
    #[serde(default = "default_writer_threads", rename = "writerThreads")]
    pub(crate) writer_threads: usize,
    #[serde(default = "default_writer_memory_bytes", rename = "writerMemoryBytes")]
    pub(crate) writer_memory_bytes: usize,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DocumentBatch {
    pub(crate) documents: Vec<DocumentRequest>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DocumentRequest {
    pub(crate) fields: HashMap<String, Vec<FieldValueRequest>>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct FieldValueRequest {
    #[serde(rename = "type")]
    pub(crate) kind: FieldKind,
    pub(crate) value: JsonValue,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeleteRequest {
    #[serde(rename = "type")]
    pub(crate) kind: FieldKind,
    pub(crate) value: JsonValue,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SearchRequest {
    pub(crate) query: String,
    #[serde(default = "default_limit")]
    pub(crate) limit: usize,
    #[serde(default)]
    pub(crate) offset: usize,
    #[serde(default, rename = "defaultFields")]
    pub(crate) default_fields: Vec<String>,
    #[serde(default, rename = "selectedFields")]
    pub(crate) selected_fields: Vec<String>,
    #[serde(default)]
    pub(crate) sort: Option<SortRequest>,
    #[serde(default, rename = "reloadBeforeSearch")]
    pub(crate) reload_before_search: bool,
    #[serde(default, rename = "countOnly")]
    pub(crate) count_only: bool,
    #[serde(default, rename = "snippetFields")]
    pub(crate) snippet_fields: Vec<String>,
}

pub(crate) struct NativeIndex {
    pub(crate) index: Index,
    pub(crate) fields: HashMap<String, FieldInfo>,
    pub(crate) field_names: HashMap<Field, String>,
    pub(crate) default_search_fields: Vec<Field>,
    pub(crate) writer: IndexWriter<TantivyDocument>,
    pub(crate) reader: IndexReader,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TokenizerMode {
    Default,
    Raw,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SortRequest {
    pub(crate) field: String,
    #[serde(default = "default_sort_order")]
    pub(crate) order: SortOrder,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug)]
pub(crate) struct BuiltSchema {
    pub(crate) schema: Schema,
    pub(crate) fields: HashMap<String, FieldInfo>,
    pub(crate) field_names: HashMap<Field, String>,
    pub(crate) default_search_fields: Vec<Field>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FieldInfo {
    pub(crate) field: Field,
    pub(crate) kind: FieldKind,
    pub(crate) stored: bool,
    pub(crate) indexed: bool,
    pub(crate) fast: bool,
    pub(crate) experimental: bool,
}

fn default_true() -> bool {
    true
}

fn default_writer_threads() -> usize {
    1
}

fn default_writer_memory_bytes() -> usize {
    50_000_000
}

fn default_limit() -> usize {
    20
}

fn default_sort_order() -> SortOrder {
    SortOrder::Desc
}
