import Foundation

/// A field/order pair for sorting results by a fast field.
public struct SortRequest: Sendable {
    public var field: String
    public var order: SortOrder

    public init(field: String, order: SortOrder = .desc) {
        self.field = field
        self.order = order
    }

    func toWire() -> [String: Any] {
        ["field": field, "order": order.rawValue]
    }
}

/// A search query and its options.
public struct SearchRequest: Sendable {
    /// Query string in Tantivy query syntax.
    public var query: String
    /// Maximum number of hits to return.
    public var limit: Int
    /// Number of leading hits to skip.
    public var offset: Int
    /// Fields to parse the query against; empty uses the schema defaults.
    public var defaultFields: [String]
    /// Stored fields to include in each hit; empty returns all stored fields.
    public var selectedFields: [String]
    /// Sort by a fast field instead of by relevance score.
    public var sort: SortRequest?
    /// Reload the reader before searching so recent commits are visible.
    public var reloadBeforeSearch: Bool
    /// Return only `totalHits` without materializing documents.
    public var countOnly: Bool
    /// Fields for which to generate highlighted HTML snippets.
    public var snippetFields: [String]

    public init(
        query: String,
        limit: Int = 20,
        offset: Int = 0,
        defaultFields: [String] = [],
        selectedFields: [String] = [],
        sort: SortRequest? = nil,
        reloadBeforeSearch: Bool = false,
        countOnly: Bool = false,
        snippetFields: [String] = []
    ) {
        self.query = query
        self.limit = limit
        self.offset = offset
        self.defaultFields = defaultFields
        self.selectedFields = selectedFields
        self.sort = sort
        self.reloadBeforeSearch = reloadBeforeSearch
        self.countOnly = countOnly
        self.snippetFields = snippetFields
    }

    func toWire() -> [String: Any] {
        var dict: [String: Any] = [
            "query": query,
            "limit": limit,
            "offset": offset,
            "defaultFields": defaultFields,
            "selectedFields": selectedFields,
            "reloadBeforeSearch": reloadBeforeSearch,
            "countOnly": countOnly,
            "snippetFields": snippetFields,
        ]
        if let sort {
            dict["sort"] = sort.toWire()
        }
        return dict
    }

    func toJSON() throws -> String {
        try JSONCoding.encode(toWire())
    }

    /// Build a request with a small DSL mirroring the Kotlin `query { }` builder.
    public static func build(_ block: (Builder) -> Void) -> SearchRequest {
        let builder = Builder()
        block(builder)
        return builder.build()
    }

    /// Mutable search-request builder.
    public final class Builder {
        public var query: String = ""
        public var limit: Int = 20
        public var offset: Int = 0
        private var defaultFields: [String] = []
        private var selectedFields: [String] = []
        public var sort: SortRequest?
        public var reloadBeforeSearch: Bool = false
        public var countOnly: Bool = false
        private var snippetFields: [String] = []

        public init() {}

        @discardableResult
        public func selectedFields(_ names: String...) -> Builder {
            selectedFields.append(contentsOf: names)
            return self
        }

        @discardableResult
        public func defaultFields(_ names: String...) -> Builder {
            defaultFields.append(contentsOf: names)
            return self
        }

        @discardableResult
        public func snippetFields(_ names: String...) -> Builder {
            snippetFields.append(contentsOf: names)
            return self
        }

        @discardableResult
        public func sortBy(_ field: String, _ order: SortOrder = .desc) -> Builder {
            sort = SortRequest(field: field, order: order)
            return self
        }

        public func build() -> SearchRequest {
            SearchRequest(
                query: query,
                limit: limit,
                offset: offset,
                defaultFields: defaultFields,
                selectedFields: selectedFields,
                sort: sort,
                reloadBeforeSearch: reloadBeforeSearch,
                countOnly: countOnly,
                snippetFields: snippetFields
            )
        }
    }
}
