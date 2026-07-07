import Foundation
import CTantivyFFI

/// Thin Swift wrapper over the C FFI. Converts strings, frees native
/// allocations, and turns error envelopes into `TantivyError`s. Everything here
/// is synchronous and blocking; concurrency policy lives in `TantivyIndex`.
enum NativeBridge {
    /// Version string reported by the native library.
    static func version() -> String {
        guard let ptr = tantivy_ffi_version() else { return "unknown" }
        defer { tantivy_string_free(ptr) }
        return String(cString: ptr)
    }

    static func openIndex(path: String, schemaJSON: String, optionsJSON: String) throws -> Int64 {
        var error: UnsafeMutablePointer<CChar>?
        let handle = path.withCString { pathPtr in
            schemaJSON.withCString { schemaPtr in
                optionsJSON.withCString { optionsPtr in
                    tantivy_open_index(pathPtr, schemaPtr, optionsPtr, &error)
                }
            }
        }
        try throwIfError(error)
        return handle
    }

    static func closeIndex(_ handle: Int64) throws {
        var error: UnsafeMutablePointer<CChar>?
        tantivy_close_index(handle, &error)
        try throwIfError(error)
    }

    static func addDocuments(_ handle: Int64, documentsJSON: String) throws -> String {
        try callString { errorPtr in
            documentsJSON.withCString { tantivy_add_documents(handle, $0, errorPtr) }
        }
    }

    static func deleteTerm(_ handle: Int64, field: String, valueJSON: String) throws -> String {
        try callString { errorPtr in
            field.withCString { fieldPtr in
                valueJSON.withCString { valuePtr in
                    tantivy_delete_term(handle, fieldPtr, valuePtr, errorPtr)
                }
            }
        }
    }

    static func deleteQuery(_ handle: Int64, query: String, defaultFieldsJSON: String) throws -> String {
        try callString { errorPtr in
            query.withCString { queryPtr in
                defaultFieldsJSON.withCString { fieldsPtr in
                    tantivy_delete_query(handle, queryPtr, fieldsPtr, errorPtr)
                }
            }
        }
    }

    static func deleteAllDocuments(_ handle: Int64) throws -> String {
        try callString { tantivy_delete_all_documents(handle, $0) }
    }

    static func commit(_ handle: Int64) throws -> String {
        try callString { tantivy_commit(handle, $0) }
    }

    static func refresh(_ handle: Int64) throws -> String {
        try callString { tantivy_refresh(handle, $0) }
    }

    static func commitAndRefresh(_ handle: Int64) throws -> String {
        try callString { tantivy_commit_and_refresh(handle, $0) }
    }

    static func schemaInfo(_ handle: Int64) throws -> String {
        try callString { tantivy_schema_info(handle, $0) }
    }

    static func search(_ handle: Int64, queryJSON: String) throws -> String {
        try callString { errorPtr in
            queryJSON.withCString { tantivy_search(handle, $0, errorPtr) }
        }
    }

    // MARK: - Plumbing

    /// Run a native call that returns an owned `char *` and reports errors via
    /// an out-parameter, converting the result to a Swift `String`.
    private static func callString(
        _ body: (UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>) -> UnsafeMutablePointer<CChar>?
    ) throws -> String {
        var error: UnsafeMutablePointer<CChar>?
        let result = body(&error)
        try throwIfError(error)
        guard let result else {
            throw TantivyError.native("native call returned no value and no error")
        }
        defer { tantivy_string_free(result) }
        return String(cString: result)
    }

    /// If `error` is non-null, decode the envelope, free it, and throw.
    private static func throwIfError(_ error: UnsafeMutablePointer<CChar>?) throws {
        guard let error else { return }
        defer { tantivy_string_free(error) }
        let json = String(cString: error)
        guard
            let object = try? JSONCoding.decodeObject(json),
            let kind = object["kind"] as? String,
            let message = object["message"] as? String
        else {
            throw TantivyError.native(json)
        }
        throw TantivyError.from(kind: kind, message: message)
    }
}
