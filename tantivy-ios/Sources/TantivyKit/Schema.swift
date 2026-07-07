import Foundation

/// One field definition in an index schema.
public struct SchemaField: Sendable {
    public var name: String
    public var type: FieldType
    public var stored: Bool
    public var indexed: Bool
    public var fast: Bool
    public var tokenizer: TokenizerMode?
    public var experimental: Bool

    public init(
        name: String,
        type: FieldType,
        stored: Bool = true,
        indexed: Bool = true,
        fast: Bool = false,
        tokenizer: TokenizerMode? = nil,
        experimental: Bool = false
    ) {
        self.name = name
        self.type = type
        self.stored = stored
        self.indexed = indexed
        self.fast = fast
        self.tokenizer = tokenizer
        self.experimental = experimental
    }

    func toWire() -> [String: Any] {
        var dict: [String: Any] = [
            "name": name,
            "type": type.rawValue,
            "stored": stored,
            "indexed": indexed,
            "fast": fast,
            "experimental": experimental,
        ]
        if let tokenizer {
            dict["tokenizer"] = tokenizer.rawValue
        }
        return dict
    }
}

/// An index schema: an ordered list of fields plus the fields searched by
/// default when a query does not name any.
public struct IndexSchema: Sendable {
    public let fields: [SchemaField]
    public let defaultSearchFields: [String]

    public init(fields: [SchemaField], defaultSearchFields: [String]) {
        self.fields = fields
        self.defaultSearchFields = defaultSearchFields
    }

    func toWire() -> [String: Any] {
        [
            "fields": fields.map { $0.toWire() },
            "defaultSearchFields": defaultSearchFields,
        ]
    }

    func toJSON() throws -> String {
        try JSONCoding.encode(toWire())
    }

    /// Build a schema with a small DSL mirroring the Kotlin `schema { }` builder.
    ///
    /// ```swift
    /// let schema = IndexSchema.build { s in
    ///     s.string("id")
    ///     s.text("title", defaultSearch: true)
    ///     s.i64("publishedAt", fast: true)
    /// }
    /// ```
    public static func build(_ block: (Builder) -> Void) -> IndexSchema {
        let builder = Builder()
        block(builder)
        return builder.build()
    }

    /// Mutable schema builder.
    public final class Builder {
        private var fields: [SchemaField] = []
        private var defaultSearchFields: [String] = []

        public init() {}

        @discardableResult
        public func text(
            _ name: String,
            stored: Bool = true,
            indexed: Bool = true,
            defaultSearch: Bool = true,
            tokenizer: TokenizerMode = .default
        ) -> Builder {
            field(name, type: .text, stored: stored, indexed: indexed, tokenizer: tokenizer)
            if defaultSearch { defaultSearchFields.append(name) }
            return self
        }

        @discardableResult
        public func string(
            _ name: String,
            stored: Bool = true,
            indexed: Bool = true,
            defaultSearch: Bool = false,
            tokenizer: TokenizerMode = .raw
        ) -> Builder {
            field(name, type: .string, stored: stored, indexed: indexed, tokenizer: tokenizer)
            if defaultSearch { defaultSearchFields.append(name) }
            return self
        }

        @discardableResult
        public func i64(_ name: String, stored: Bool = true, indexed: Bool = true, fast: Bool = false) -> Builder {
            field(name, type: .i64, stored: stored, indexed: indexed, fast: fast)
        }

        @discardableResult
        public func u64(_ name: String, stored: Bool = true, indexed: Bool = true, fast: Bool = false) -> Builder {
            field(name, type: .u64, stored: stored, indexed: indexed, fast: fast)
        }

        @discardableResult
        public func f64(_ name: String, stored: Bool = true, indexed: Bool = true, fast: Bool = false) -> Builder {
            field(name, type: .f64, stored: stored, indexed: indexed, fast: fast)
        }

        @discardableResult
        public func bool(_ name: String, stored: Bool = true, indexed: Bool = true, fast: Bool = false) -> Builder {
            field(name, type: .bool, stored: stored, indexed: indexed, fast: fast)
        }

        @discardableResult
        public func bytes(_ name: String, stored: Bool = true, indexed: Bool = true, fast: Bool = false) -> Builder {
            field(name, type: .bytes, stored: stored, indexed: indexed, fast: fast)
        }

        @discardableResult
        public func date(_ name: String, stored: Bool = true, indexed: Bool = true, fast: Bool = false) -> Builder {
            field(name, type: .date, stored: stored, indexed: indexed, fast: fast)
        }

        @discardableResult
        public func json(_ name: String, stored: Bool = true, indexed: Bool = true, fast: Bool = false) -> Builder {
            field(name, type: .json, stored: stored, indexed: indexed, fast: fast)
        }

        @discardableResult
        public func facet(_ name: String, stored: Bool = true) -> Builder {
            field(name, type: .facet, stored: stored)
        }

        @discardableResult
        public func ipAddr(_ name: String, stored: Bool = true, indexed: Bool = true, fast: Bool = false) -> Builder {
            field(name, type: .ipAddr, stored: stored, indexed: indexed, fast: fast)
        }

        @discardableResult
        public func field(
            _ name: String,
            type: FieldType,
            stored: Bool = true,
            indexed: Bool = true,
            fast: Bool = false,
            tokenizer: TokenizerMode? = nil,
            experimental: Bool = false
        ) -> Builder {
            fields.append(
                SchemaField(
                    name: name,
                    type: type,
                    stored: stored,
                    indexed: indexed,
                    fast: fast,
                    tokenizer: tokenizer,
                    experimental: experimental
                )
            )
            return self
        }

        @discardableResult
        public func defaultSearchFields(_ names: String...) -> Builder {
            defaultSearchFields.append(contentsOf: names)
            return self
        }

        public func build() -> IndexSchema {
            // Preserve order while removing duplicate default-search fields.
            var seen = Set<String>()
            let uniqueDefaults = defaultSearchFields.filter { seen.insert($0).inserted }
            return IndexSchema(fields: fields, defaultSearchFields: uniqueDefaults)
        }
    }
}
