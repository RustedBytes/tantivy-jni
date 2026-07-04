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

#[test]
fn test_ram_directory_date_json_search_options() {
    let schema = json!({
        "fields": [
            { "name": "title", "type": "text", "stored": true, "indexed": true },
            { "name": "published", "type": "date", "stored": true, "indexed": true, "fast": true },
            { "name": "metadata", "type": "json", "stored": true, "indexed": true }
        ],
        "defaultSearchFields": ["title"]
    })
    .to_string();

    let handle = open_index(
        ":memory:",
        &schema,
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();

    let doc_json = json!({
        "documents": [{
            "fields": {
                "title": [{ "type": "text", "value": "Tantivy JNI release is cool" }],
                "published": [{ "type": "date", "value": 1609459200000i64 }],
                "metadata": [{ "type": "json", "value": { "tags": ["rust", "jni"], "rating": 5 } }]
            }
        }]
    })
    .to_string();

    add_documents(handle, &doc_json).unwrap();
    commit_and_refresh(handle).unwrap();

    // 1. Check basic retrieval of new types
    let result = search(
        handle,
        &json!({
            "query": "JNI",
            "limit": 10,
            "offset": 0
        })
        .to_string(),
    )
    .unwrap();

    let result: JsonValue = serde_json::from_str(&result).unwrap();
    let hits = result["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(
        hits[0]["fields"]["published"][0]["value"].as_i64(),
        Some(1609459200000i64)
    );
    assert_eq!(
        hits[0]["fields"]["metadata"][0]["value"]["tags"][0].as_str(),
        Some("rust")
    );

    // 2. Check highlighting (snippet fields)
    let highlight_result = search(
        handle,
        &json!({
            "query": "release",
            "limit": 10,
            "offset": 0,
            "snippetFields": ["title"]
        })
        .to_string(),
    )
    .unwrap();
    let highlight_result: JsonValue = serde_json::from_str(&highlight_result).unwrap();
    let snippet = highlight_result["hits"][0]["snippets"]["title"]
        .as_str()
        .unwrap();
    assert!(snippet.contains("<b>release</b>"));

    // 3. Check count only
    let count_result = search(
        handle,
        &json!({
            "query": "cool",
            "limit": 10,
            "offset": 0,
            "countOnly": true
        })
        .to_string(),
    )
    .unwrap();
    let count_result: JsonValue = serde_json::from_str(&count_result).unwrap();
    assert_eq!(count_result["totalHits"].as_i64(), Some(1));
    assert_eq!(count_result["hits"].as_array().unwrap().len(), 0);

    // 4. Test delete by query
    let delete_op = crate::delete_query(handle, "cool", &json!([]).to_string()).unwrap();
    let delete_op: JsonValue = serde_json::from_str(&delete_op).unwrap();
    assert!(delete_op["opstamp"].as_u64().is_some());
    commit_and_refresh(handle).unwrap();

    let after_delete = search(
        handle,
        &json!({
            "query": "cool",
            "limit": 10,
            "offset": 0,
            "countOnly": true
        })
        .to_string(),
    )
    .unwrap();
    let after_delete: JsonValue = serde_json::from_str(&after_delete).unwrap();
    assert_eq!(after_delete["totalHits"].as_i64(), Some(0));

    close_index(handle).unwrap();
}

#[test]
fn facet_field_indexing() {
    let handle = open_index(
        ":memory:",
        &json!({
            "fields": [
                { "name": "title", "type": "text", "stored": true, "indexed": true },
                { "name": "category", "type": "facet", "stored": true, "indexed": false }
            ],
            "defaultSearchFields": ["title"]
        })
        .to_string(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();

    add_documents(
        handle,
        &json!({
            "documents": [{
                "fields": {
                    "title": [{ "type": "text", "value": "my android phone" }],
                    "category": [{ "type": "facet", "value": "/electronics/phones" }]
                }
            }]
        })
        .to_string(),
    )
    .unwrap();
    commit_and_refresh(handle).unwrap();

    let result = search(
        handle,
        &json!({ "query": "android", "limit": 10, "offset": 0 }).to_string(),
    )
    .unwrap();
    let result: JsonValue = serde_json::from_str(&result).unwrap();
    let hits = result["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 1);
    let category = &hits[0]["fields"]["category"][0];
    assert_eq!(category["type"].as_str(), Some("facet"));
    assert_eq!(category["value"].as_str(), Some("/electronics/phones"));
    close_index(handle).unwrap();
}

#[test]
fn ip_addr_field_indexing() {
    let handle = open_index(
        ":memory:",
        &json!({
            "fields": [
                { "name": "title", "type": "text", "stored": true, "indexed": true },
                { "name": "ip", "type": "ipaddr", "stored": true, "indexed": true, "fast": true }
            ],
            "defaultSearchFields": ["title"]
        })
        .to_string(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();

    add_documents(
        handle,
        &json!({
            "documents": [
                {
                    "fields": {
                        "title": [{ "type": "text", "value": "server alpha" }],
                        "ip": [{ "type": "ipaddr", "value": "192.168.1.1" }]
                    }
                },
                {
                    "fields": {
                        "title": [{ "type": "text", "value": "server beta" }],
                        "ip": [{ "type": "ipaddr", "value": "::1" }]
                    }
                }
            ]
        })
        .to_string(),
    )
    .unwrap();
    commit_and_refresh(handle).unwrap();

    let result = search(
        handle,
        &json!({ "query": "server", "limit": 10, "offset": 0 }).to_string(),
    )
    .unwrap();
    let result: JsonValue = serde_json::from_str(&result).unwrap();
    let hits = result["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 2);
    // Both should have an ip field of type "ipaddr" with string value
    for hit in hits {
        let ip = &hit["fields"]["ip"][0];
        assert_eq!(ip["type"].as_str(), Some("ipaddr"));
        assert!(ip["value"].as_str().is_some());
    }
    close_index(handle).unwrap();
}

#[test]
fn delete_all_documents_clears_index() {
    let handle = open_index(
        ":memory:",
        &json!({
            "fields": [
                { "name": "title", "type": "text", "stored": true, "indexed": true }
            ],
            "defaultSearchFields": ["title"]
        })
        .to_string(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();

    add_documents(
        handle,
        &json!({
            "documents": [
                { "fields": { "title": [{ "type": "text", "value": "first document" }] } },
                { "fields": { "title": [{ "type": "text", "value": "second document" }] } }
            ]
        })
        .to_string(),
    )
    .unwrap();
    commit_and_refresh(handle).unwrap();

    // Verify documents exist
    let before = search(
        handle,
        &json!({ "query": "document", "limit": 10, "offset": 0 }).to_string(),
    )
    .unwrap();
    let before: JsonValue = serde_json::from_str(&before).unwrap();
    assert_eq!(before["hits"].as_array().unwrap().len(), 2);

    // Delete all documents
    let del_result = crate::delete_all_documents(handle).unwrap();
    let del_result: JsonValue = serde_json::from_str(&del_result).unwrap();
    assert!(del_result["opstamp"].as_u64().is_some());

    commit_and_refresh(handle).unwrap();

    // Verify index is empty
    let after = search(
        handle,
        &json!({ "query": "document", "limit": 10, "offset": 0 }).to_string(),
    )
    .unwrap();
    let after: JsonValue = serde_json::from_str(&after).unwrap();
    assert_eq!(after["hits"].as_array().unwrap().len(), 0);
    close_index(handle).unwrap();
}

#[test]
fn search_with_reload_before_search() {
    let handle = open_index(
        ":memory:",
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
                        "title": [{ "type": "text", "value": "test reload" }],
                        "id": [{ "type": "string", "value": "1" }],
                        "price": [{ "type": "i64", "value": 10 }]
                    }
                }
            ]
        })
        .to_string(),
    )
    .unwrap();
    commit(handle).unwrap();
    // Intentionally omit refresh(handle)

    // Using reloadBeforeSearch
    let result = search(
        handle,
        &json!({
            "query": "reload",
            "limit": 10,
            "reloadBeforeSearch": true
        })
        .to_string(),
    )
    .unwrap();
    let result: JsonValue = serde_json::from_str(&result).unwrap();
    let hits = result["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 1);
    close_index(handle).unwrap();
}

#[test]
fn raw_tokenizer_mode() {
    let schema = json!({
        "fields": [
            { "name": "title", "type": "text", "stored": true, "indexed": true, "tokenizer": "raw" }
        ],
        "defaultSearchFields": ["title"]
    })
    .to_string();

    let handle = open_index(
        ":memory:",
        &schema,
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();

    add_documents(
        handle,
        &json!({
            "documents": [{
                "fields": {
                    "title": [{ "type": "text", "value": "Hello World" }]
                }
            }]
        })
        .to_string(),
    )
    .unwrap();
    commit_and_refresh(handle).unwrap();

    // Querying for "Hello" should yield 0 hits because it's stored as "Hello World"
    let result1 = search(
        handle,
        &json!({ "query": "Hello", "limit": 10 }).to_string(),
    )
    .unwrap();
    let result1: JsonValue = serde_json::from_str(&result1).unwrap();
    assert_eq!(result1["hits"].as_array().unwrap().len(), 0);

    // Querying for exactly "Hello World" might need quotes or exact match, but we can search exactly
    let result2 = search(
        handle,
        &json!({ "query": "\"Hello World\"", "limit": 10 }).to_string(),
    )
    .unwrap();
    let result2: JsonValue = serde_json::from_str(&result2).unwrap();
    assert_eq!(result2["hits"].as_array().unwrap().len(), 1);

    close_index(handle).unwrap();
}

#[test]
fn boolean_and_phrase_queries() {
    let handle = open_index(
        ":memory:",
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
                        "title": [{ "type": "text", "value": "apple orange" }],
                        "id": [{ "type": "string", "value": "1" }],
                        "price": [{ "type": "i64", "value": 10 }]
                    }
                },
                {
                    "fields": {
                        "title": [{ "type": "text", "value": "apple banana" }],
                        "id": [{ "type": "string", "value": "2" }],
                        "price": [{ "type": "i64", "value": 15 }]
                    }
                }
            ]
        })
        .to_string(),
    )
    .unwrap();
    commit_and_refresh(handle).unwrap();

    // Must have apple, must NOT have banana
    let result = search(
        handle,
        &json!({ "query": "+apple -banana", "limit": 10 }).to_string(),
    )
    .unwrap();
    let result: JsonValue = serde_json::from_str(&result).unwrap();
    let hits = result["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["fields"]["id"][0]["value"].as_str(), Some("1"));

    close_index(handle).unwrap();
}

#[test]
fn reject_malformed_query() {
    let handle = open_index(
        ":memory:",
        &schema_json(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    )
    .unwrap();

    let error = search(
        handle,
        &json!({ "query": "\"", "limit": 10 }).to_string(), // Malformed query
    )
    .unwrap_err();
    assert!(matches!(error, NativeError::Search(_)));

    close_index(handle).unwrap();
}

#[test]
fn delete_numeric_term() {
    let handle = open_index(
        ":memory:",
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
                        "title": [{ "type": "text", "value": "keep" }],
                        "id": [{ "type": "string", "value": "1" }],
                        "price": [{ "type": "i64", "value": 100 }]
                    }
                },
                {
                    "fields": {
                        "title": [{ "type": "text", "value": "delete" }],
                        "id": [{ "type": "string", "value": "2" }],
                        "price": [{ "type": "i64", "value": 200 }]
                    }
                }
            ]
        })
        .to_string(),
    )
    .unwrap();
    commit_and_refresh(handle).unwrap();

    // Delete document with price 200
    crate::delete_term(
        handle,
        "price",
        &json!({ "type": "i64", "value": 200 }).to_string(),
    )
    .unwrap();
    commit_and_refresh(handle).unwrap();

    let result = search(
        handle,
        &json!({ "query": "*", "limit": 10 }).to_string(),
    )
    .unwrap();
    let result: JsonValue = serde_json::from_str(&result).unwrap();
    let hits = result["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["fields"]["id"][0]["value"].as_str(), Some("1"));

    close_index(handle).unwrap();
}

#[test]
fn all_field_types_indexing_and_retrieval() {
    let handle = open_index(
        ":memory:",
        &json!({
            "fields": [
                { "name": "title", "type": "text", "stored": true, "indexed": true },
                { "name": "u64_val", "type": "u64", "stored": true, "indexed": true, "fast": true },
                { "name": "f64_val", "type": "f64", "stored": true, "indexed": true, "fast": true },
                { "name": "bool_val", "type": "bool", "stored": true, "indexed": true, "fast": true },
                { "name": "bytes_val", "type": "bytes", "stored": true, "indexed": true, "fast": true }
            ],
            "defaultSearchFields": ["title"]
        }).to_string(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    ).unwrap();

    add_documents(
        handle,
        &json!({
            "documents": [
                {
                    "fields": {
                        "title": [{ "type": "text", "value": "test types" }],
                        "u64_val": [{ "type": "u64", "value": 42 }],
                        "f64_val": [{ "type": "f64", "value": 3.14 }],
                        "bool_val": [{ "type": "bool", "value": true }],
                        "bytes_val": [{ "type": "bytes", "value": [104, 101, 108, 108, 111] }] // "hello"
                    }
                }
            ]
        }).to_string(),
    ).unwrap();
    commit_and_refresh(handle).unwrap();

    let result = search(handle, &json!({ "query": "test", "limit": 10 }).to_string()).unwrap();
    let result: JsonValue = serde_json::from_str(&result).unwrap();
    let hits = result["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 1);
    
    let doc_fields = &hits[0]["fields"];
    assert_eq!(doc_fields["u64_val"][0]["value"].as_u64(), Some(42));
    assert_eq!(doc_fields["f64_val"][0]["value"].as_f64(), Some(3.14));
    assert_eq!(doc_fields["bool_val"][0]["value"].as_bool(), Some(true));
    
    let bytes_arr = doc_fields["bytes_val"][0]["value"].as_array().unwrap();
    assert_eq!(bytes_arr.len(), 5);
    assert_eq!(bytes_arr[0].as_u64(), Some(104));

    // Test delete by each of these numeric/bool terms
    crate::delete_term(handle, "u64_val", &json!({ "type": "u64", "value": 42 }).to_string()).unwrap();
    crate::delete_term(handle, "f64_val", &json!({ "type": "f64", "value": 3.14 }).to_string()).unwrap();
    crate::delete_term(handle, "bool_val", &json!({ "type": "bool", "value": true }).to_string()).unwrap();
    crate::delete_term(handle, "bytes_val", &json!({ "type": "bytes", "value": [104, 101, 108, 108, 111] }).to_string()).unwrap();

    close_index(handle).unwrap();
}

#[test]
fn delete_json_field_fails() {
    let handle = open_index(
        ":memory:",
        &json!({
            "fields": [
                { "name": "meta", "type": "json", "stored": true, "indexed": true }
            ],
            "defaultSearchFields": ["meta"]
        }).to_string(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    ).unwrap();

    let err = crate::delete_term(handle, "meta", &json!({ "type": "json", "value": {} }).to_string()).unwrap_err();
    assert!(matches!(err, NativeError::Write(_)));

    close_index(handle).unwrap();
}

#[test]
fn reject_invalid_json_values_for_types() {
    let handle = open_index(
        ":memory:",
        &json!({
            "fields": [
                { "name": "u64_val", "type": "u64", "stored": true, "indexed": true },
                { "name": "f64_val", "type": "f64", "stored": true, "indexed": true },
                { "name": "bool_val", "type": "bool", "stored": true, "indexed": true },
                { "name": "bytes_val", "type": "bytes", "stored": true, "indexed": true }
            ],
            "defaultSearchFields": []
        }).to_string(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    ).unwrap();

    // Wrong type for u64
    assert!(matches!(
        add_documents(handle, &json!({ "documents": [{ "fields": { "u64_val": [{ "type": "u64", "value": "string" }] } }] }).to_string()).unwrap_err(),
        NativeError::Write(_)
    ));

    // Wrong type for f64
    assert!(matches!(
        add_documents(handle, &json!({ "documents": [{ "fields": { "f64_val": [{ "type": "f64", "value": "string" }] } }] }).to_string()).unwrap_err(),
        NativeError::Write(_)
    ));

    // Wrong type for bool
    assert!(matches!(
        add_documents(handle, &json!({ "documents": [{ "fields": { "bool_val": [{ "type": "bool", "value": "string" }] } }] }).to_string()).unwrap_err(),
        NativeError::Write(_)
    ));

    // Wrong type for bytes
    assert!(matches!(
        add_documents(handle, &json!({ "documents": [{ "fields": { "bytes_val": [{ "type": "bytes", "value": "string" }] } }] }).to_string()).unwrap_err(),
        NativeError::Write(_)
    ));

    // Out of bounds byte value
    assert!(matches!(
        add_documents(handle, &json!({ "documents": [{ "fields": { "bytes_val": [{ "type": "bytes", "value": [256] }] } }] }).to_string()).unwrap_err(),
        NativeError::Write(_)
    ));

    close_index(handle).unwrap();
}

#[test]
fn reject_oversized_search_offset() {
    let handle = open_index(
        ":memory:",
        &schema_json(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    ).unwrap();

    let err = search(
        handle,
        &json!({ "query": "test", "limit": 10, "offset": 100001 }).to_string(),
    ).unwrap_err();
    assert!(matches!(err, NativeError::Search(_)));

    close_index(handle).unwrap();
}

#[test]
fn reject_unknown_snippet_field() {
    let handle = open_index(
        ":memory:",
        &schema_json(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    ).unwrap();

    let err = search(
        handle,
        &json!({ "query": "test", "limit": 10, "snippetFields": ["unknown"] }).to_string(),
    ).unwrap_err();
    assert!(matches!(err, NativeError::Search(_)));

    close_index(handle).unwrap();
}

#[test]
fn reject_not_stored_selected_field() {
    let handle = open_index(
        ":memory:",
        &json!({
            "fields": [
                { "name": "title", "type": "text", "stored": true, "indexed": true },
                { "name": "hidden", "type": "text", "stored": false, "indexed": true }
            ],
            "defaultSearchFields": ["title"]
        }).to_string(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    ).unwrap();

    let err = search(
        handle,
        &json!({ "query": "test", "limit": 10, "selectedFields": ["hidden"] }).to_string(),
    ).unwrap_err();
    assert!(matches!(err, NativeError::Search(_)));

    close_index(handle).unwrap();
}

#[test]
fn test_sorting_u64_and_f64_and_asc() {
    let handle = open_index(
        ":memory:",
        &json!({
            "fields": [
                { "name": "title", "type": "text", "stored": true, "indexed": true },
                { "name": "u64_val", "type": "u64", "stored": true, "indexed": true, "fast": true },
                { "name": "f64_val", "type": "f64", "stored": true, "indexed": true, "fast": true }
            ],
            "defaultSearchFields": ["title"]
        }).to_string(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    ).unwrap();

    add_documents(
        handle,
        &json!({
            "documents": [
                { "fields": { "title": [{ "type": "text", "value": "test" }], "u64_val": [{ "type": "u64", "value": 10 }], "f64_val": [{ "type": "f64", "value": 1.1 }] } },
                { "fields": { "title": [{ "type": "text", "value": "test" }], "u64_val": [{ "type": "u64", "value": 20 }], "f64_val": [{ "type": "f64", "value": 2.2 }] } }
            ]
        }).to_string(),
    ).unwrap();
    commit_and_refresh(handle).unwrap();

    // Sort u64 Asc
    let res = search(handle, &json!({ "query": "test", "limit": 10, "sort": { "field": "u64_val", "order": "asc" } }).to_string()).unwrap();
    let res: JsonValue = serde_json::from_str(&res).unwrap();
    let hits = res["hits"].as_array().unwrap();
    assert_eq!(hits[0]["fields"]["u64_val"][0]["value"].as_u64(), Some(10));

    // Sort f64 Desc
    let res = search(handle, &json!({ "query": "test", "limit": 10, "sort": { "field": "f64_val", "order": "desc" } }).to_string()).unwrap();
    let res: JsonValue = serde_json::from_str(&res).unwrap();
    let hits = res["hits"].as_array().unwrap();
    assert_eq!(hits[0]["fields"]["f64_val"][0]["value"].as_f64(), Some(2.2));

    close_index(handle).unwrap();
}

#[test]
fn reject_sort_by_unsupported_fast_field() {
    let handle = open_index(
        ":memory:",
        &json!({
            "fields": [
                { "name": "title", "type": "text", "stored": true, "indexed": true },
                { "name": "date_val", "type": "date", "stored": true, "indexed": true, "fast": true }
            ],
            "defaultSearchFields": ["title"]
        }).to_string(),
        &json!({ "create": true, "writerThreads": 1, "writerMemoryBytes": 50000000 }).to_string(),
    ).unwrap();

    let err = search(
        handle,
        &json!({ "query": "test", "limit": 10, "sort": { "field": "date_val", "order": "asc" } }).to_string(),
    ).unwrap_err();
    assert!(matches!(err, NativeError::Search(_)));

    close_index(handle).unwrap();
}

#[test]
fn test_default_values_deserialization() {
    // Omitting create, writerThreads, writerMemoryBytes
    let handle = open_index(
        ":memory:",
        &json!({
            "fields": [{ "name": "title", "type": "text", "stored": true, "indexed": true }],
            "defaultSearchFields": ["title"]
        }).to_string(),
        "{}",
    ).unwrap();

    // Omitting limit, offset, defaultFields, selectedFields, sort, countOnly, snippetFields
    let res = search(handle, &json!({ "query": "test" }).to_string()).unwrap();
    let res: JsonValue = serde_json::from_str(&res).unwrap();
    assert_eq!(res["hits"].as_array().unwrap().len(), 0);

    close_index(handle).unwrap();
}

#[test]
fn reject_oversized_writer_memory() {
    let err = open_index(
        ":memory:",
        &schema_json(),
        &json!({ "writerMemoryBytes": 1024 * 1024 * 1024 }).to_string(),
    ).unwrap_err();
    assert!(matches!(err, NativeError::Open(_)));
}

#[test]
fn reject_undersized_writer_memory() {
    let err = open_index(
        ":memory:",
        &schema_json(),
        &json!({ "writerMemoryBytes": 1000 }).to_string(),
    ).unwrap_err();
    assert!(matches!(err, NativeError::Open(_)));
}

#[test]
fn reject_oversized_writer_threads() {
    let err = open_index(
        ":memory:",
        &schema_json(),
        &json!({ "writerThreads": 100 }).to_string(),
    ).unwrap_err();
    assert!(matches!(err, NativeError::Open(_)));
}

#[test]
fn reject_too_many_documents_in_batch() {
    let handle = open_index(
        ":memory:",
        &schema_json(),
        "{}",
    ).unwrap();

    let mut docs = Vec::new();
    for _ in 0..10_001 {
        docs.push(json!({ "fields": { "title": [{ "type": "text", "value": "test" }] } }));
    }

    let err = add_documents(
        handle,
        &json!({ "documents": docs }).to_string(),
    ).unwrap_err();
    assert!(matches!(err, NativeError::Write(_)));

    close_index(handle).unwrap();
}

#[test]
fn reject_too_many_field_values_in_document() {
    let handle = open_index(
        ":memory:",
        &schema_json(),
        "{}",
    ).unwrap();

    let mut vals = Vec::new();
    for _ in 0..10_001 {
        vals.push(json!({ "type": "text", "value": "test" }));
    }

    let err = add_documents(
        handle,
        &json!({ "documents": [{ "fields": { "title": vals } }] }).to_string(),
    ).unwrap_err();
    assert!(matches!(err, NativeError::Write(_)));

    close_index(handle).unwrap();
}

#[test]
fn reject_delete_term_with_unknown_field() {
    let handle = open_index(
        ":memory:",
        &schema_json(),
        "{}",
    ).unwrap();

    let err = crate::delete_term(
        handle,
        "unknown_field",
        &json!({ "type": "string", "value": "test" }).to_string(),
    ).unwrap_err();
    assert!(matches!(err, NativeError::Write(_)));

    close_index(handle).unwrap();
}

#[test]
fn reject_delete_query_with_unknown_default_field() {
    let handle = open_index(
        ":memory:",
        &schema_json(),
        "{}",
    ).unwrap();

    let err = crate::delete_query(
        handle,
        "test",
        &json!(["unknown_field"]).to_string(),
    ).unwrap_err();
    assert!(matches!(err, NativeError::Write(_)));

    close_index(handle).unwrap();
}

#[test]
fn reject_delete_query_with_empty_fields_overall() {
    let handle = open_index(
        ":memory:",
        &json!({
            "fields": [
                { "name": "u64_val", "type": "u64", "stored": true, "indexed": true }
            ],
            "defaultSearchFields": []
        }).to_string(),
        "{}",
    ).unwrap();

    let err = crate::delete_query(
        handle,
        "test",
        &json!([]).to_string(),
    ).unwrap_err();
    assert!(matches!(err, NativeError::Write(_)));

    close_index(handle).unwrap();
}

#[test]
fn reject_add_documents_with_unknown_field() {
    let handle = open_index(
        ":memory:",
        &schema_json(),
        "{}",
    ).unwrap();

    let err = add_documents(
        handle,
        &json!({
            "documents": [
                {
                    "fields": {
                        "unknown": [{ "type": "text", "value": "test" }]
                    }
                }
            ]
        }).to_string(),
    ).unwrap_err();
    assert!(matches!(err, NativeError::Write(_)));

    close_index(handle).unwrap();
}

#[test]
fn reject_malformed_json_in_delete_query() {
    let handle = open_index(
        ":memory:",
        &schema_json(),
        "{}",
    ).unwrap();

    let err = crate::delete_query(
        handle,
        "test",
        "{invalid}",
    ).unwrap_err();
    assert!(matches!(err, NativeError::Json(_)));

    close_index(handle).unwrap();
}
