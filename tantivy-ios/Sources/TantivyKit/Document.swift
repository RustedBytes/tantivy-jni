import Foundation

/// A document to index: a map of field name to one or more typed values.
public struct IndexDocument {
    public let fields: [String: [FieldValue]]

    public init(fields: [String: [FieldValue]]) {
        self.fields = fields
    }

    func toWire() -> [String: Any] {
        var wireFields: [String: Any] = [:]
        for (name, values) in fields {
            wireFields[name] = values.map { $0.toWire() }
        }
        return ["fields": wireFields]
    }

    /// Build a document with a small DSL mirroring the Kotlin `document { }`
    /// builder.
    ///
    /// ```swift
    /// let doc = IndexDocument.build { d in
    ///     d.string("id", "1")
    ///     d.text("title", "Tantivy on iOS")
    ///     d.i64("publishedAt", 1_700_000_000_000)
    /// }
    /// ```
    public static func build(_ block: (Builder) -> Void) -> IndexDocument {
        let builder = Builder()
        block(builder)
        return builder.build()
    }

    /// Mutable document builder. Repeated calls for the same field append
    /// values (fields are multi-valued).
    public final class Builder {
        private var fields: [String: [FieldValue]] = [:]

        public init() {}

        @discardableResult
        public func field(_ name: String, _ value: FieldValue) -> Builder {
            fields[name, default: []].append(value)
            return self
        }

        @discardableResult public func text(_ name: String, _ value: String) -> Builder { field(name, .text(value)) }
        @discardableResult public func string(_ name: String, _ value: String) -> Builder { field(name, .string(value)) }
        @discardableResult public func i64(_ name: String, _ value: Int64) -> Builder { field(name, .i64(value)) }
        @discardableResult public func u64(_ name: String, _ value: UInt64) -> Builder { field(name, .u64(value)) }
        @discardableResult public func f64(_ name: String, _ value: Double) -> Builder { field(name, .f64(value)) }
        @discardableResult public func bool(_ name: String, _ value: Bool) -> Builder { field(name, .bool(value)) }
        @discardableResult public func bytes(_ name: String, _ value: [UInt8]) -> Builder { field(name, .bytes(value)) }
        @discardableResult public func date(_ name: String, _ value: Date) -> Builder { field(name, .date(value)) }
        @discardableResult public func json(_ name: String, _ value: Any) -> Builder { field(name, .json(value)) }
        @discardableResult public func facet(_ name: String, _ path: String) -> Builder { field(name, .facet(path)) }
        @discardableResult public func ipAddr(_ name: String, _ address: String) -> Builder { field(name, .ipAddr(address)) }

        @discardableResult
        public func repeated(_ name: String, _ values: [FieldValue]) -> Builder {
            for value in values { field(name, value) }
            return self
        }

        public func build() -> IndexDocument {
            IndexDocument(fields: fields)
        }
    }
}
