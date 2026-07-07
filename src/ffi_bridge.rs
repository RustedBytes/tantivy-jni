//! C ABI bridge for non-JVM consumers (notably Swift on iOS/macOS).
//!
//! This module mirrors [`jni_bridge`](crate::jni_bridge): it is a thin
//! marshalling layer over the same core operations (`open_index`,
//! `add_documents`, `search`, ...). Where the JNI bridge exchanges Java
//! strings and throws Java exceptions, this bridge exchanges UTF-8 C strings
//! and reports failures through an out-parameter carrying a JSON error
//! envelope of the shape `{"kind": "...", "message": "..."}`.
//!
//! Ownership contract for callers:
//! - Every non-null `char*` returned by a function in this module (including
//!   the string written to `out_error`) is heap-allocated by Rust and MUST be
//!   released with [`tantivy_string_free`]. Freeing it any other way, or twice,
//!   is undefined behaviour.
//! - Input `char*` arguments are borrowed for the duration of the call only;
//!   the caller keeps ownership.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

use serde_json::json;

use crate::{
    NativeError, NativeResult, add_documents, close_index, commit, commit_and_refresh,
    delete_all_documents, delete_query, delete_term, open_index, refresh, schema_info, search,
};

/// Stable error category shared with the Swift layer. Kept in sync with the
/// exception classes the JNI bridge throws so both bindings classify failures
/// the same way.
fn error_kind(error: &NativeError) -> &'static str {
    match error {
        NativeError::Schema(_) => "schema",
        NativeError::Open(_) => "open",
        NativeError::Write(_) => "write",
        NativeError::Search(_) => "search",
        NativeError::InvalidHandle(_) => "closed",
        NativeError::Panic
        | NativeError::State(_)
        | NativeError::Json(_)
        | NativeError::Tantivy(_) => "native",
    }
}

/// Write a JSON error envelope into `out_error`, if the caller supplied one.
///
/// # Safety
/// `out_error` must be null or a valid, writable `*mut *mut c_char`.
unsafe fn set_error(out_error: *mut *mut c_char, error: &NativeError) {
    if out_error.is_null() {
        return;
    }
    let payload = json!({
        "kind": error_kind(error),
        "message": error.to_string(),
    })
    .to_string();
    // Error messages are Rust strings and serde output never contains interior
    // NUL bytes, but fall back to a fixed envelope rather than panic if it ever
    // does.
    let owned = CString::new(payload).unwrap_or_else(|_| {
        CString::new(r#"{"kind":"native","message":"error contained a NUL byte"}"#)
            .expect("static envelope is NUL-free")
    });
    unsafe { *out_error = owned.into_raw() };
}

/// Clear the caller's error slot before running an operation.
///
/// # Safety
/// `out_error` must be null or a valid, writable `*mut *mut c_char`.
unsafe fn clear_error(out_error: *mut *mut c_char) {
    if !out_error.is_null() {
        unsafe { *out_error = ptr::null_mut() };
    }
}

/// Copy a borrowed C string argument into an owned `String`.
///
/// # Safety
/// `ptr` must be null or point to a valid, NUL-terminated C string that stays
/// valid for the duration of this call.
unsafe fn read_arg(ptr: *const c_char, name: &str) -> NativeResult<String> {
    if ptr.is_null() {
        return Err(NativeError::State(format!("null argument: {name}")));
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(ToOwned::to_owned)
        .map_err(|error| NativeError::State(format!("invalid UTF-8 in {name}: {error}")))
}

/// Run a fallible operation that yields a handle, reporting failures through
/// `out_error` and returning `0` (never a valid handle) on error.
///
/// # Safety
/// `out_error` must satisfy the [`set_error`]/[`clear_error`] contract.
unsafe fn run_long(out_error: *mut *mut c_char, action: impl FnOnce() -> NativeResult<i64>) -> i64 {
    unsafe { clear_error(out_error) };
    match catch_unwind(AssertUnwindSafe(action)).unwrap_or(Err(NativeError::Panic)) {
        Ok(value) => value,
        Err(error) => {
            unsafe { set_error(out_error, &error) };
            0
        }
    }
}

/// Run a fallible operation with no successful payload.
///
/// # Safety
/// `out_error` must satisfy the [`set_error`]/[`clear_error`] contract.
unsafe fn run_unit(out_error: *mut *mut c_char, action: impl FnOnce() -> NativeResult<()>) {
    unsafe { clear_error(out_error) };
    if let Err(error) = catch_unwind(AssertUnwindSafe(action)).unwrap_or(Err(NativeError::Panic)) {
        unsafe { set_error(out_error, &error) };
    }
}

/// Run a fallible operation that yields a JSON string. On success returns a
/// freshly allocated C string (caller frees with [`tantivy_string_free`]); on
/// error returns null and writes an envelope to `out_error`.
///
/// # Safety
/// `out_error` must satisfy the [`set_error`]/[`clear_error`] contract.
unsafe fn run_string(
    out_error: *mut *mut c_char,
    action: impl FnOnce() -> NativeResult<String>,
) -> *mut c_char {
    unsafe { clear_error(out_error) };
    match catch_unwind(AssertUnwindSafe(action)).unwrap_or(Err(NativeError::Panic)) {
        Ok(value) => match CString::new(value) {
            Ok(owned) => owned.into_raw(),
            Err(_) => {
                unsafe {
                    set_error(
                        out_error,
                        &NativeError::State("native result contained a NUL byte".to_string()),
                    )
                };
                ptr::null_mut()
            }
        },
        Err(error) => {
            unsafe { set_error(out_error, &error) };
            ptr::null_mut()
        }
    }
}

/// Release a string previously returned by this module. Passing null is a
/// no-op; passing any other pointer not obtained from this module, or the same
/// pointer twice, is undefined behaviour.
///
/// # Safety
/// See above.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tantivy_string_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    drop(unsafe { CString::from_raw(ptr) });
}

/// Version of the native library (the crate version). Caller frees the result.
#[unsafe(no_mangle)]
pub extern "C" fn tantivy_ffi_version() -> *mut c_char {
    CString::new(env!("CARGO_PKG_VERSION"))
        .map(CString::into_raw)
        .unwrap_or(ptr::null_mut())
}

/// Open (or create) an index. Returns a positive handle on success, or `0`
/// with `out_error` set on failure.
///
/// # Safety
/// The string arguments must be null or valid NUL-terminated UTF-8 C strings;
/// `out_error` must be null or a valid writable slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tantivy_open_index(
    path: *const c_char,
    schema_json: *const c_char,
    options_json: *const c_char,
    out_error: *mut *mut c_char,
) -> i64 {
    unsafe {
        run_long(out_error, || {
            let path = read_arg(path, "path")?;
            let schema_json = read_arg(schema_json, "schema_json")?;
            let options_json = read_arg(options_json, "options_json")?;
            open_index(&path, &schema_json, &options_json)
        })
    }
}

/// Close a handle, releasing all native resources for the index.
///
/// # Safety
/// `out_error` must be null or a valid writable slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tantivy_close_index(handle: i64, out_error: *mut *mut c_char) {
    unsafe { run_unit(out_error, || close_index(handle)) }
}

/// Add a batch of documents. `documents_json` is the batch wire format
/// (`{"documents": [...]}`). Returns `{"documentsAdded": n}` JSON.
///
/// # Safety
/// See [`tantivy_open_index`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tantivy_add_documents(
    handle: i64,
    documents_json: *const c_char,
    out_error: *mut *mut c_char,
) -> *mut c_char {
    unsafe {
        run_string(out_error, || {
            let documents_json = read_arg(documents_json, "documents_json")?;
            add_documents(handle, &documents_json)
        })
    }
}

/// Delete documents matching a term. Returns `{"termsDeleted": 1}` JSON.
///
/// # Safety
/// See [`tantivy_open_index`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tantivy_delete_term(
    handle: i64,
    field: *const c_char,
    value_json: *const c_char,
    out_error: *mut *mut c_char,
) -> *mut c_char {
    unsafe {
        run_string(out_error, || {
            let field = read_arg(field, "field")?;
            let value_json = read_arg(value_json, "value_json")?;
            delete_term(handle, &field, &value_json)
        })
    }
}

/// Delete documents matching a query. Returns `{"opstamp": n}` JSON.
///
/// # Safety
/// See [`tantivy_open_index`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tantivy_delete_query(
    handle: i64,
    query: *const c_char,
    default_fields_json: *const c_char,
    out_error: *mut *mut c_char,
) -> *mut c_char {
    unsafe {
        run_string(out_error, || {
            let query = read_arg(query, "query")?;
            let default_fields_json = read_arg(default_fields_json, "default_fields_json")?;
            delete_query(handle, &query, &default_fields_json)
        })
    }
}

/// Delete every document in the index. Returns `{"opstamp": n}` JSON.
///
/// # Safety
/// `out_error` must be null or a valid writable slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tantivy_delete_all_documents(
    handle: i64,
    out_error: *mut *mut c_char,
) -> *mut c_char {
    unsafe { run_string(out_error, || delete_all_documents(handle)) }
}

/// Commit pending writes. Returns `{"opstamp": n}` JSON.
///
/// # Safety
/// `out_error` must be null or a valid writable slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tantivy_commit(handle: i64, out_error: *mut *mut c_char) -> *mut c_char {
    unsafe { run_string(out_error, || commit(handle)) }
}

/// Reload the reader so committed writes become searchable. Returns
/// `{"refreshed": true}` JSON.
///
/// # Safety
/// `out_error` must be null or a valid writable slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tantivy_refresh(handle: i64, out_error: *mut *mut c_char) -> *mut c_char {
    unsafe { run_string(out_error, || refresh(handle)) }
}

/// Commit then reload in one call. Returns `{"opstamp": n, "refreshed": true}`.
///
/// # Safety
/// `out_error` must be null or a valid writable slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tantivy_commit_and_refresh(
    handle: i64,
    out_error: *mut *mut c_char,
) -> *mut c_char {
    unsafe { run_string(out_error, || commit_and_refresh(handle)) }
}

/// Describe the index schema. Returns `{"fields": [...], "defaultSearchFields": [...]}`.
///
/// # Safety
/// `out_error` must be null or a valid writable slot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tantivy_schema_info(
    handle: i64,
    out_error: *mut *mut c_char,
) -> *mut c_char {
    unsafe { run_string(out_error, || schema_info(handle)) }
}

/// Run a search. `query_json` is the search request wire format. Returns
/// `{"totalHits": n, "hits": [...]}` JSON.
///
/// # Safety
/// See [`tantivy_open_index`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tantivy_search(
    handle: i64,
    query_json: *const c_char,
    out_error: *mut *mut c_char,
) -> *mut c_char {
    unsafe {
        run_string(out_error, || {
            let query_json = read_arg(query_json, "query_json")?;
            search(handle, &query_json)
        })
    }
}
