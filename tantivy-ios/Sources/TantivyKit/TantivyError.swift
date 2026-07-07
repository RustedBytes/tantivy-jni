import Foundation

/// Errors surfaced by TantivyKit. The first six cases map one-to-one to the
/// native error categories (and to the Android exception hierarchy); `decoding`
/// covers malformed responses on the Swift side.
public enum TantivyError: Error, CustomStringConvertible {
    /// Invalid schema definition.
    case schema(String)
    /// Failure opening or creating the index.
    case open(String)
    /// Failure adding, deleting, or committing documents.
    case write(String)
    /// Failure parsing or executing a search.
    case search(String)
    /// The index handle is closed or invalid.
    case closed(String)
    /// An internal/native failure not covered above (panic, lock, JSON, ...).
    case native(String)
    /// A native response could not be decoded on the Swift side.
    case decoding(String)

    public var message: String {
        switch self {
        case .schema(let m), .open(let m), .write(let m), .search(let m),
            .closed(let m), .native(let m), .decoding(let m):
            return m
        }
    }

    public var description: String {
        switch self {
        case .schema: return "schema error: \(message)"
        case .open: return "index open error: \(message)"
        case .write: return "write error: \(message)"
        case .search: return "search error: \(message)"
        case .closed: return "index closed: \(message)"
        case .native: return "native error: \(message)"
        case .decoding: return "decoding error: \(message)"
        }
    }

    /// Build an error from the native `{"kind":...,"message":...}` envelope.
    static func from(kind: String, message: String) -> TantivyError {
        switch kind {
        case "schema": return .schema(message)
        case "open": return .open(message)
        case "write": return .write(message)
        case "search": return .search(message)
        case "closed": return .closed(message)
        default: return .native(message)
        }
    }
}
