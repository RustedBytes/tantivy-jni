use serde_json::{Value as JsonValue, json};
use std::fs;
use std::time::Instant;

use crate::NativeError;
use crate::model::SchemaRequest;
use crate::schema::build_schema;
use crate::{
    add_documents, close_index, commit, commit_and_refresh, open_index, refresh, schema_info,
    search,
};

fn schema_json() -> String {
    json!({
        "fields": [
                { "name": "title", "type": "text", "stored": true, "indexed": true },
                { "name": "id", "type": "string", "stored": true, "indexed": true },
                { "name": "price", "type": "i64", "stored": true, "indexed": true, "fast": true }
        ],
        "defaultSearchFields": ["title"]
    })
    .to_string()
}

#[test]
fn supports_selected_fields_sort_and_schema_info() {
    let dir = tempfile::tempdir().unwrap();
    let handle = open_index(
        dir.path().to_str().unwrap(),
        &schema_json(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();
    add_documents(
        handle,
        &json!({
            "documents": [
                {
                    "fields": {
                        "title": [{ "type": "text", "value": "android search" }],
                        "id": [{ "type": "string", "value": "expensive" }],
                        "price": [{ "type": "i64", "value": 20 }]
                    }
                },
                {
                    "fields": {
                        "title": [{ "type": "text", "value": "android search" }],
                        "id": [{ "type": "string", "value": "cheap" }],
                        "price": [{ "type": "i64", "value": 5 }]
                    }
                }
            ]
        })
        .to_string(),
    )
    .unwrap();
    let commit = commit_and_refresh(handle).unwrap();
    let commit: JsonValue = serde_json::from_str(&commit).unwrap();
    assert_eq!(commit["refreshed"].as_bool(), Some(true));

    let result = search(
        handle,
        &json!({
            "query": "android",
            "limit": 10,
            "offset": 0,
            "selectedFields": ["id"],
            "sort": { "field": "price", "order": "asc" }
        })
        .to_string(),
    )
    .unwrap();
    let result: JsonValue = serde_json::from_str(&result).unwrap();
    let hits = result["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 2);
    assert!(hits[0]["fields"].get("title").is_none());
    assert_eq!(hits[0]["fields"]["id"][0]["value"].as_str(), Some("cheap"));

    let schema_info = schema_info(handle).unwrap();
    let schema_info: JsonValue = serde_json::from_str(&schema_info).unwrap();
    assert_eq!(schema_info["fields"].as_array().unwrap().len(), 3);
    close_index(handle).unwrap();
}

#[test]
fn schema_requires_fields() {
    let error = build_schema(&SchemaRequest {
        fields: Vec::new(),
        default_search_fields: Vec::new(),
    })
    .unwrap_err();
    assert!(matches!(error, NativeError::Schema(_)));
}

#[test]
fn creates_adds_and_searches_index() {
    let dir = tempfile::tempdir().unwrap();
    let handle = open_index(
        dir.path().to_str().unwrap(),
        &schema_json(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();
    add_documents(
        handle,
        &json!({
            "documents": [{
                "fields": {
                    "title": [{ "type": "text", "value": "hello android search" }],
                    "id": [{ "type": "string", "value": "doc-1" }]
                }
            }]
        })
        .to_string(),
    )
    .unwrap();
    commit(handle).unwrap();
    refresh(handle).unwrap();
    let result = search(
        handle,
        &json!({ "query": "android", "limit": 10, "offset": 0 }).to_string(),
    )
    .unwrap();
    let result: JsonValue = serde_json::from_str(&result).unwrap();
    assert_eq!(result["hits"].as_array().unwrap().len(), 1);
    close_index(handle).unwrap();
}

#[test]
fn rejects_oversized_search_limit() {
    let dir = tempfile::tempdir().unwrap();
    let handle = open_index(
        dir.path().to_str().unwrap(),
        &schema_json(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();

    let error = search(
        handle,
        &json!({ "query": "android", "limit": 1001, "offset": 0 }).to_string(),
    )
    .unwrap_err();
    assert!(matches!(error, NativeError::Search(_)));
    close_index(handle).unwrap();
}

#[test]
fn rejects_unknown_default_search_field() {
    let dir = tempfile::tempdir().unwrap();
    let handle = open_index(
        dir.path().to_str().unwrap(),
        &schema_json(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();

    let error = search(
        handle,
        &json!({
            "query": "android",
            "limit": 10,
            "offset": 0,
            "defaultFields": ["missing"]
        })
        .to_string(),
    )
    .unwrap_err();
    assert!(matches!(error, NativeError::Search(_)));
    close_index(handle).unwrap();
}

#[test]
fn rejects_sort_on_non_fast_field() {
    let dir = tempfile::tempdir().unwrap();
    let handle = open_index(
        dir.path().to_str().unwrap(),
        &schema_json(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();

    let error = search(
        handle,
        &json!({
            "query": "android",
            "limit": 10,
            "offset": 0,
            "sort": { "field": "id", "order": "asc" }
        })
        .to_string(),
    )
    .unwrap_err();
    assert!(matches!(error, NativeError::Search(_)));
    close_index(handle).unwrap();
}

#[test]
fn rejects_unknown_selected_field() {
    let dir = tempfile::tempdir().unwrap();
    let handle = open_index(
        dir.path().to_str().unwrap(),
        &schema_json(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();

    let error = search(
        handle,
        &json!({
            "query": "android",
            "limit": 10,
            "offset": 0,
            "selectedFields": ["missing"]
        })
        .to_string(),
    )
    .unwrap_err();
    assert!(matches!(error, NativeError::Search(_)));
    close_index(handle).unwrap();
}

#[test]
fn reopens_committed_index_without_create() {
    let dir = tempfile::tempdir().unwrap();
    let handle = open_index(
        dir.path().to_str().unwrap(),
        &schema_json(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();
    add_documents(
        handle,
        &json!({
            "documents": [{
                "fields": {
                    "title": [{ "type": "text", "value": "persisted android document" }],
                    "id": [{ "type": "string", "value": "doc-persisted" }]
                }
            }]
        })
        .to_string(),
    )
    .unwrap();
    commit_and_refresh(handle).unwrap();
    close_index(handle).unwrap();

    let reopened = open_index(
        dir.path().to_str().unwrap(),
        &schema_json(),
        &json!({ "create": false, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();
    refresh(reopened).unwrap();
    let result = search(
        reopened,
        &json!({ "query": "persisted", "limit": 10, "offset": 0 }).to_string(),
    )
    .unwrap();
    let result: JsonValue = serde_json::from_str(&result).unwrap();
    assert_eq!(result["hits"].as_array().unwrap().len(), 1);
    close_index(reopened).unwrap();
}

#[test]
fn rejects_corrupted_existing_index_directory() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("meta.json"), "{not-valid-index-meta").unwrap();

    let error = open_index(
        dir.path().to_str().unwrap(),
        &schema_json(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap_err();

    assert!(matches!(error, NativeError::Open(_)));
}

#[test]
fn delete_term_removes_document_after_commit_and_refresh() {
    let dir = tempfile::tempdir().unwrap();
    let handle = open_index(
        dir.path().to_str().unwrap(),
        &schema_json(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();
    add_documents(
        handle,
        &json!({
            "documents": [{
                "fields": {
                    "title": [{ "type": "text", "value": "delete me" }],
                    "id": [{ "type": "string", "value": "doc-delete" }]
                }
            }]
        })
        .to_string(),
    )
    .unwrap();
    commit_and_refresh(handle).unwrap();

    crate::delete_term(
        handle,
        "id",
        &json!({ "type": "string", "value": "doc-delete" }).to_string(),
    )
    .unwrap();
    commit_and_refresh(handle).unwrap();

    let result = search(
        handle,
        &json!({ "query": "delete", "limit": 10, "offset": 0 }).to_string(),
    )
    .unwrap();
    let result: JsonValue = serde_json::from_str(&result).unwrap();
    assert_eq!(result["hits"].as_array().unwrap().len(), 0);
    close_index(handle).unwrap();
}

#[test]
fn rejects_invalid_open_options() {
    let dir = tempfile::tempdir().unwrap();
    let error = open_index(
        dir.path().to_str().unwrap(),
        &schema_json(),
        &json!({ "create": true, "writerThreads": 0, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap_err();
    assert!(matches!(error, NativeError::Open(_)));
}

#[test]
fn rejects_wrong_document_field_type() {
    let dir = tempfile::tempdir().unwrap();
    let handle = open_index(
        dir.path().to_str().unwrap(),
        &schema_json(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();

    let error = add_documents(
        handle,
        &json!({
            "documents": [{
                "fields": {
                    "price": [{ "type": "text", "value": "not a number" }]
                }
            }]
        })
        .to_string(),
    )
    .unwrap_err();
    assert!(matches!(error, NativeError::Write(_)));
    close_index(handle).unwrap();
}

#[test]
fn rejects_operations_after_close() {
    let dir = tempfile::tempdir().unwrap();
    let handle = open_index(
        dir.path().to_str().unwrap(),
        &schema_json(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();
    close_index(handle).unwrap();

    let error = search(
        handle,
        &json!({ "query": "android", "limit": 10, "offset": 0 }).to_string(),
    )
    .unwrap_err();
    assert!(matches!(error, NativeError::InvalidHandle(_)));
}

#[test]
fn rejects_double_close() {
    let dir = tempfile::tempdir().unwrap();
    let handle = open_index(
        dir.path().to_str().unwrap(),
        &schema_json(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();

    close_index(handle).unwrap();
    let error = close_index(handle).unwrap_err();
    assert!(matches!(error, NativeError::InvalidHandle(_)));
}

#[test]
fn rejects_malformed_json_contracts() {
    let dir = tempfile::tempdir().unwrap();
    let error = open_index(
        dir.path().to_str().unwrap(),
        "{not-json",
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap_err();
    assert!(matches!(error, NativeError::Json(_)));

    let handle = open_index(
        dir.path().join("valid").to_str().unwrap(),
        &schema_json(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();
    let error = add_documents(handle, "{not-json").unwrap_err();
    assert!(matches!(error, NativeError::Json(_)));
    close_index(handle).unwrap();
}

#[test]
fn rejects_unknown_schema_default_search_field() {
    let schema: SchemaRequest = serde_json::from_value(json!({
        "fields": [
            { "name": "title", "type": "text", "stored": true, "indexed": true }
        ],
        "defaultSearchFields": ["missing"]
    }))
    .unwrap();

    let error = build_schema(&schema).unwrap_err();
    assert!(matches!(error, NativeError::Schema(_)));
}

#[test]
#[ignore = "performance smoke test; run explicitly with --ignored --nocapture"]
fn indexing_search_smoke_benchmark() {
    let dir = tempfile::tempdir().unwrap();
    let handle = open_index(
        dir.path().to_str().unwrap(),
        &schema_json(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();

    let documents = (0..1_000)
        .map(|index| {
            json!({
                "fields": {
                    "title": [{ "type": "text", "value": format!("android benchmark document {index}") }],
                    "id": [{ "type": "string", "value": format!("doc-{index}") }],
                    "price": [{ "type": "i64", "value": index }]
                }
            })
        })
        .collect::<Vec<_>>();

    let started = Instant::now();
    add_documents(handle, &json!({ "documents": documents }).to_string()).unwrap();
    let add_elapsed = started.elapsed();

    let started = Instant::now();
    commit_and_refresh(handle).unwrap();
    let commit_refresh_elapsed = started.elapsed();

    let started = Instant::now();
    let result = search(
        handle,
        &json!({
            "query": "android",
            "limit": 20,
            "offset": 0,
            "sort": { "field": "price", "order": "desc" }
        })
        .to_string(),
    )
    .unwrap();
    let search_elapsed = started.elapsed();

    let result: JsonValue = serde_json::from_str(&result).unwrap();
    assert_eq!(result["hits"].as_array().unwrap().len(), 20);
    eprintln!("indexed 1000 docs in {add_elapsed:?}");
    eprintln!("commit+refresh took {commit_refresh_elapsed:?}");
    eprintln!("sorted search took {search_elapsed:?}");
    close_index(handle).unwrap();
}
