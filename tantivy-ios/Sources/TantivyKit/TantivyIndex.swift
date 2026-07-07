import Foundation

/// A handle to an open Tantivy index.
///
/// Native operations are synchronous and blocking. Each method has a
/// synchronous throwing form and an `async` form; the async forms hop onto a
/// serial background queue so callers never block the main thread and calls
/// against one index never overlap. The index is closed automatically on
/// `deinit`, but calling ``close()`` explicitly when finished is recommended.
public final class TantivyIndex {
    private let handle: Int64
    private let queue: DispatchQueue
    private var isClosed = false
    private let lock = NSLock()

    init(handle: Int64) {
        self.handle = handle
        self.queue = DispatchQueue(label: "TantivyKit.index.\(handle)", qos: .userInitiated)
    }

    deinit {
        // Best-effort close; ignore errors during teardown.
        if !isClosed {
            try? NativeBridge.closeIndex(handle)
        }
    }

    // MARK: - Synchronous API

    /// Add one document.
    @discardableResult
    public func add(_ document: IndexDocument) throws -> WriteResult {
        try add([document])
    }

    /// Add a batch of documents.
    @discardableResult
    public func add(_ documents: [IndexDocument]) throws -> WriteResult {
        let batch: [String: Any] = ["documents": documents.map { $0.toWire() }]
        let json = try NativeBridge.addDocuments(handle, documentsJSON: JSONCoding.encode(batch))
        return WriteResult.decode(try JSONCoding.decodeObject(json))
    }

    /// Delete documents whose `field` matches `value`.
    @discardableResult
    public func deleteTerm(field: String, value: FieldValue) throws -> DeleteResult {
        let valueJSON = try JSONCoding.encode(value.toWire())
        let json = try NativeBridge.deleteTerm(handle, field: field, valueJSON: valueJSON)
        return DeleteResult.decode(try JSONCoding.decodeObject(json))
    }

    /// Delete documents matching `query`. `defaultFields` empty uses the schema
    /// default search fields.
    @discardableResult
    public func deleteQuery(_ query: String, defaultFields: [String] = []) throws -> CommitResult {
        let fieldsJSON = try JSONCoding.encode(defaultFields)
        let json = try NativeBridge.deleteQuery(handle, query: query, defaultFieldsJSON: fieldsJSON)
        return CommitResult.decode(try JSONCoding.decodeObject(json))
    }

    /// Delete every document in the index.
    @discardableResult
    public func deleteAll() throws -> CommitResult {
        let json = try NativeBridge.deleteAllDocuments(handle)
        return CommitResult.decode(try JSONCoding.decodeObject(json))
    }

    /// Commit pending writes to disk.
    @discardableResult
    public func commit() throws -> CommitResult {
        let json = try NativeBridge.commit(handle)
        return CommitResult.decode(try JSONCoding.decodeObject(json))
    }

    /// Reload the reader so committed writes become searchable.
    @discardableResult
    public func refresh() throws -> RefreshResult {
        let json = try NativeBridge.refresh(handle)
        return RefreshResult.decode(try JSONCoding.decodeObject(json))
    }

    /// Commit then reload in a single call.
    @discardableResult
    public func commitAndRefresh() throws -> CommitRefreshResult {
        let json = try NativeBridge.commitAndRefresh(handle)
        return CommitRefreshResult.decode(try JSONCoding.decodeObject(json))
    }

    /// Describe the index schema.
    public func schemaInfo() throws -> SchemaInfo {
        let json = try NativeBridge.schemaInfo(handle)
        return SchemaInfo.decode(try JSONCoding.decodeObject(json))
    }

    /// Run a search.
    public func search(_ request: SearchRequest) throws -> SearchPage {
        let json = try NativeBridge.search(handle, queryJSON: try request.toJSON())
        return try SearchPage.decode(try JSONCoding.decodeObject(json))
    }

    /// Close the index and release native resources. Idempotent.
    public func close() throws {
        lock.lock()
        defer { lock.unlock() }
        guard !isClosed else { return }
        try NativeBridge.closeIndex(handle)
        isClosed = true
    }

    // MARK: - Async API

    @discardableResult
    public func add(_ document: IndexDocument) async throws -> WriteResult {
        try await run { try self.add(document) }
    }

    @discardableResult
    public func add(_ documents: [IndexDocument]) async throws -> WriteResult {
        try await run { try self.add(documents) }
    }

    @discardableResult
    public func deleteTerm(field: String, value: FieldValue) async throws -> DeleteResult {
        try await run { try self.deleteTerm(field: field, value: value) }
    }

    @discardableResult
    public func deleteQuery(_ query: String, defaultFields: [String] = []) async throws -> CommitResult {
        try await run { try self.deleteQuery(query, defaultFields: defaultFields) }
    }

    @discardableResult
    public func deleteAll() async throws -> CommitResult {
        try await run { try self.deleteAll() }
    }

    @discardableResult
    public func commit() async throws -> CommitResult {
        try await run { try self.commit() }
    }

    @discardableResult
    public func refresh() async throws -> RefreshResult {
        try await run { try self.refresh() }
    }

    @discardableResult
    public func commitAndRefresh() async throws -> CommitRefreshResult {
        try await run { try self.commitAndRefresh() }
    }

    public func schemaInfo() async throws -> SchemaInfo {
        try await run { try self.schemaInfo() }
    }

    public func search(_ request: SearchRequest) async throws -> SearchPage {
        try await run { try self.search(request) }
    }

    public func close() async throws {
        try await run { try self.close() }
    }

    /// Run blocking work on this index's serial queue.
    private func run<T>(_ work: @escaping () throws -> T) async throws -> T {
        try await withCheckedThrowingContinuation { continuation in
            queue.async {
                do {
                    continuation.resume(returning: try work())
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }
}
