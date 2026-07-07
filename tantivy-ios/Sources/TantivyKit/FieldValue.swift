import Foundation

/// A single typed field value in a document or search hit.
///
/// Values marshal to/from the native layer as `{"type": <name>, "value": ...}`,
/// matching the Rust wire format used by the Android bindings.
public enum FieldValue {
    case text(String)
    case string(String)
    case i64(Int64)
    case u64(UInt64)
    case f64(Double)
    case bool(Bool)
    case bytes([UInt8])
    /// Stored as milliseconds since the Unix epoch on the wire.
    case date(Date)
    /// Arbitrary JSON. The associated value must be a `JSONSerialization`-
    /// compatible object (dictionary, array, number, string, bool, or NSNull).
    case json(Any)
    /// A facet path, e.g. `/category/rust`.
    case facet(String)
    /// An IP address in textual form (IPv4 or IPv6).
    case ipAddr(String)

    /// The field type this value carries.
    public var type: FieldType {
        switch self {
        case .text: return .text
        case .string: return .string
        case .i64: return .i64
        case .u64: return .u64
        case .f64: return .f64
        case .bool: return .bool
        case .bytes: return .bytes
        case .date: return .date
        case .json: return .json
        case .facet: return .facet
        case .ipAddr: return .ipAddr
        }
    }

    /// Encode into the `{"type":...,"value":...}` wire dictionary.
    func toWire() -> [String: Any] {
        switch self {
        case .text(let value):
            return ["type": FieldType.text.rawValue, "value": value]
        case .string(let value):
            return ["type": FieldType.string.rawValue, "value": value]
        case .i64(let value):
            return ["type": FieldType.i64.rawValue, "value": NSNumber(value: value)]
        case .u64(let value):
            return ["type": FieldType.u64.rawValue, "value": NSNumber(value: value)]
        case .f64(let value):
            return ["type": FieldType.f64.rawValue, "value": NSNumber(value: value)]
        case .bool(let value):
            return ["type": FieldType.bool.rawValue, "value": NSNumber(value: value)]
        case .bytes(let value):
            return ["type": FieldType.bytes.rawValue, "value": value.map { Int($0) }]
        case .date(let value):
            let millis = Int64((value.timeIntervalSince1970 * 1000).rounded())
            return ["type": FieldType.date.rawValue, "value": NSNumber(value: millis)]
        case .json(let value):
            return ["type": FieldType.json.rawValue, "value": value]
        case .facet(let value):
            return ["type": FieldType.facet.rawValue, "value": value]
        case .ipAddr(let value):
            return ["type": FieldType.ipAddr.rawValue, "value": value]
        }
    }

    /// Decode from a `{"type":...,"value":...}` wire dictionary.
    static func fromWire(_ dict: [String: Any]) throws -> FieldValue {
        guard let typeName = dict["type"] as? String else {
            throw TantivyError.decoding("field value missing 'type'")
        }
        guard let type = FieldType(rawValue: typeName) else {
            throw TantivyError.decoding("unknown field value type '\(typeName)'")
        }
        let raw = dict["value"]
        switch type {
        case .text:
            return .text(raw as? String ?? "")
        case .string:
            return .string(raw as? String ?? "")
        case .i64:
            return .i64((raw as? NSNumber)?.int64Value ?? 0)
        case .u64:
            return .u64((raw as? NSNumber)?.uint64Value ?? 0)
        case .f64:
            return .f64((raw as? NSNumber)?.doubleValue ?? 0)
        case .bool:
            return .bool((raw as? NSNumber)?.boolValue ?? false)
        case .bytes:
            let numbers = raw as? [NSNumber] ?? []
            return .bytes(numbers.map { UInt8(truncatingIfNeeded: $0.intValue) })
        case .date:
            let millis = (raw as? NSNumber)?.int64Value ?? 0
            return .date(Date(timeIntervalSince1970: Double(millis) / 1000.0))
        case .json:
            return .json(raw ?? NSNull())
        case .facet:
            return .facet(raw as? String ?? "")
        case .ipAddr:
            return .ipAddr(raw as? String ?? "")
        }
    }
}
