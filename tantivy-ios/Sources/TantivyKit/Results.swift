import Foundation

/// Result of adding documents.
public struct WriteResult {
    public let documentsAdded: Int
}

/// Result of deleting by term.
public struct DeleteResult {
    public let termsDeleted: Int
}

/// Result of a commit (and delete-query / delete-all, which report an opstamp).
public struct CommitResult {
    public let opstamp: Int64
}

/// Result of a reader refresh.
public struct RefreshResult {
    public let refreshed: Bool
}

/// Result of a combined commit + refresh.
public struct CommitRefreshResult {
    public let opstamp: Int64
    public let refreshed: Bool
}

/// A single search hit.
public struct SearchHit {
    /// Relevance score (0 for sorted searches).
    public let score: Double
    /// Stored field values keyed by field name.
    public let fields: [String: [FieldValue]]
    /// Highlighted HTML snippets keyed by field name (empty unless requested).
    public let snippets: [String: String]
}

/// A page of search results.
public struct SearchPage {
    public let totalHits: Int
    public let hits: [SearchHit]
}

/// One field as reported by `schemaInfo()`.
public struct SchemaFieldInfo {
    public let name: String
    public let type: FieldType
    public let stored: Bool
    public let indexed: Bool
    public let fast: Bool
    public let experimental: Bool
}

/// The index schema as reported by `schemaInfo()`.
public struct SchemaInfo {
    public let fields: [SchemaFieldInfo]
    public let defaultSearchFields: [String]
}

// MARK: - Decoding from native JSON objects

extension WriteResult {
    static func decode(_ object: [String: Any]) -> WriteResult {
        WriteResult(documentsAdded: (object["documentsAdded"] as? NSNumber)?.intValue ?? 0)
    }
}

extension DeleteResult {
    static func decode(_ object: [String: Any]) -> DeleteResult {
        DeleteResult(termsDeleted: (object["termsDeleted"] as? NSNumber)?.intValue ?? 0)
    }
}

extension CommitResult {
    static func decode(_ object: [String: Any]) -> CommitResult {
        CommitResult(opstamp: (object["opstamp"] as? NSNumber)?.int64Value ?? 0)
    }
}

extension RefreshResult {
    static func decode(_ object: [String: Any]) -> RefreshResult {
        RefreshResult(refreshed: (object["refreshed"] as? NSNumber)?.boolValue ?? false)
    }
}

extension CommitRefreshResult {
    static func decode(_ object: [String: Any]) -> CommitRefreshResult {
        CommitRefreshResult(
            opstamp: (object["opstamp"] as? NSNumber)?.int64Value ?? 0,
            refreshed: (object["refreshed"] as? NSNumber)?.boolValue ?? false
        )
    }
}

extension SearchPage {
    static func decode(_ object: [String: Any]) throws -> SearchPage {
        let totalHits = (object["totalHits"] as? NSNumber)?.intValue ?? 0
        let rawHits = object["hits"] as? [[String: Any]] ?? []
        let hits = try rawHits.map { try SearchHit.decode($0) }
        return SearchPage(totalHits: totalHits, hits: hits)
    }
}

extension SearchHit {
    static func decode(_ object: [String: Any]) throws -> SearchHit {
        let score = (object["score"] as? NSNumber)?.doubleValue ?? 0
        var fields: [String: [FieldValue]] = [:]
        if let rawFields = object["fields"] as? [String: Any] {
            for (name, value) in rawFields {
                let entries = value as? [[String: Any]] ?? []
                fields[name] = try entries.map { try FieldValue.fromWire($0) }
            }
        }
        var snippets: [String: String] = [:]
        if let rawSnippets = object["snippets"] as? [String: Any] {
            for (name, value) in rawSnippets {
                if let text = value as? String { snippets[name] = text }
            }
        }
        return SearchHit(score: score, fields: fields, snippets: snippets)
    }
}

extension SchemaInfo {
    static func decode(_ object: [String: Any]) -> SchemaInfo {
        let rawFields = object["fields"] as? [[String: Any]] ?? []
        let fields = rawFields.map { field -> SchemaFieldInfo in
            SchemaFieldInfo(
                name: field["name"] as? String ?? "",
                type: FieldType(rawValue: field["type"] as? String ?? "") ?? .text,
                stored: (field["stored"] as? NSNumber)?.boolValue ?? false,
                indexed: (field["indexed"] as? NSNumber)?.boolValue ?? false,
                fast: (field["fast"] as? NSNumber)?.boolValue ?? false,
                experimental: (field["experimental"] as? NSNumber)?.boolValue ?? false
            )
        }
        let defaults = object["defaultSearchFields"] as? [String] ?? []
        return SchemaInfo(fields: fields, defaultSearchFields: defaults)
    }
}
