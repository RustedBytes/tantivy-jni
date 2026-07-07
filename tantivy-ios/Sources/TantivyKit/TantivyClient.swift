import Foundation

/// Entry point for TantivyKit. Mirrors the Kotlin `TantivyClient` facade:
/// builders for schemas, documents, and queries, plus `open`.
public enum TantivyClient {
    /// Path value that opens a purely in-memory index (nothing is written to
    /// disk). Useful for tests and ephemeral indexes.
    public static let inMemoryPath = ":memory:"

    /// The native library version.
    public static var nativeVersion: String { NativeBridge.version() }

    /// Open (or create) an index at `path` with the given schema and options.
    ///
    /// Pass ``inMemoryPath`` for an in-memory index.
    public static func open(
        path: String,
        schema: IndexSchema,
        options: IndexOptions = IndexOptions()
    ) throws -> TantivyIndex {
        let handle = try NativeBridge.openIndex(
            path: path,
            schemaJSON: try schema.toJSON(),
            optionsJSON: try options.toJSON()
        )
        return TantivyIndex(handle: handle)
    }

    /// Open (or create) an index off the main thread.
    public static func open(
        path: String,
        schema: IndexSchema,
        options: IndexOptions = IndexOptions()
    ) async throws -> TantivyIndex {
        try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global(qos: .userInitiated).async {
                do {
                    continuation.resume(returning: try open(path: path, schema: schema, options: options))
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// Build a schema with the schema DSL.
    public static func schema(_ block: (IndexSchema.Builder) -> Void) -> IndexSchema {
        IndexSchema.build(block)
    }

    /// Build a document with the document DSL.
    public static func document(_ block: (IndexDocument.Builder) -> Void) -> IndexDocument {
        IndexDocument.build(block)
    }

    /// Build a search request with the query DSL.
    public static func query(_ block: (SearchRequest.Builder) -> Void) -> SearchRequest {
        SearchRequest.build(block)
    }
}
