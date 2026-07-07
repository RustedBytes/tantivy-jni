import Foundation

/// The kind of a schema field. The raw values are the on-the-wire names shared
/// with the native layer (and the Android bindings).
public enum FieldType: String, Sendable {
    case text
    case string
    case i64
    case u64
    case f64
    case bool
    case bytes
    case date
    case json
    case facet
    case ipAddr = "ipaddr"
}

/// Tokenizer applied to a text/string field.
public enum TokenizerMode: String, Sendable {
    /// Standard tokenizer (lowercased, split on word boundaries).
    case `default`
    /// Raw, untokenized — the whole value is a single token.
    case raw
}

/// Sort direction for `SortRequest`.
public enum SortOrder: String, Sendable {
    case asc
    case desc
}
