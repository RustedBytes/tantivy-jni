/*
 * Tantivy JNI — C ABI for non-JVM consumers (Swift on iOS/macOS).
 *
 * This header declares the C entry points implemented in `src/ffi_bridge.rs`.
 * It mirrors the JNI bridge: a thin layer over the same core operations,
 * exchanging UTF-8 C strings and reporting failures through an out-parameter.
 *
 * Ownership:
 *   - Every non-NULL `char *` returned by these functions — including the
 *     string written to `out_error` — is owned by the caller and MUST be
 *     released with `tantivy_string_free`. Do not free it any other way and
 *     never free the same pointer twice.
 *   - `const char *` arguments are borrowed for the duration of the call only.
 *
 * Errors:
 *   - On failure, `*out_error` is set to a heap-allocated JSON string of the
 *     form {"kind":"schema|open|write|search|closed|native","message":"..."}.
 *   - On success, `*out_error` is set to NULL.
 *   - Functions returning `char *` return NULL on failure.
 *   - `tantivy_open_index` returns 0 on failure (0 is never a valid handle).
 *   - `out_error` may be NULL if the caller does not want error details.
 *
 * Threading: handles are safe to use from multiple threads; each index
 * serializes native access internally.
 */

#ifndef TANTIVY_FFI_H
#define TANTIVY_FFI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Free a string previously returned by this library. NULL is a no-op. */
void tantivy_string_free(char *ptr);

/* Native library version (crate version). Caller frees the result. */
char *tantivy_ffi_version(void);

/* Open or create an index. Returns a positive handle, or 0 on failure. */
int64_t tantivy_open_index(const char *path, const char *schema_json,
                           const char *options_json, char **out_error);

/* Close a handle and release its native resources. */
void tantivy_close_index(int64_t handle, char **out_error);

/* Add a batch of documents. Returns {"documentsAdded":n} JSON. */
char *tantivy_add_documents(int64_t handle, const char *documents_json,
                            char **out_error);

/* Delete documents matching a term. Returns {"termsDeleted":1} JSON. */
char *tantivy_delete_term(int64_t handle, const char *field,
                          const char *value_json, char **out_error);

/* Delete documents matching a query. Returns {"opstamp":n} JSON. */
char *tantivy_delete_query(int64_t handle, const char *query,
                           const char *default_fields_json, char **out_error);

/* Delete every document. Returns {"opstamp":n} JSON. */
char *tantivy_delete_all_documents(int64_t handle, char **out_error);

/* Commit pending writes. Returns {"opstamp":n} JSON. */
char *tantivy_commit(int64_t handle, char **out_error);

/* Reload the reader. Returns {"refreshed":true} JSON. */
char *tantivy_refresh(int64_t handle, char **out_error);

/* Commit then reload. Returns {"opstamp":n,"refreshed":true} JSON. */
char *tantivy_commit_and_refresh(int64_t handle, char **out_error);

/* Describe the schema. Returns {"fields":[...],"defaultSearchFields":[...]}. */
char *tantivy_schema_info(int64_t handle, char **out_error);

/* Run a search. Returns {"totalHits":n,"hits":[...]} JSON. */
char *tantivy_search(int64_t handle, const char *query_json, char **out_error);

#ifdef __cplusplus
}
#endif

#endif /* TANTIVY_FFI_H */
