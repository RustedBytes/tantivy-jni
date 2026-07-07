import Foundation

/// Small helpers around `JSONSerialization` used to marshal wire dictionaries
/// to/from the compact JSON strings the native layer exchanges.
enum JSONCoding {
    /// Serialize a wire object (dictionary or array) to a compact JSON string.
    static func encode(_ object: Any) throws -> String {
        let data = try JSONSerialization.data(withJSONObject: object, options: [])
        guard let string = String(data: data, encoding: .utf8) else {
            throw TantivyError.decoding("could not encode request as UTF-8")
        }
        return string
    }

    /// Parse a JSON string into a `[String: Any]` object.
    static func decodeObject(_ json: String) throws -> [String: Any] {
        guard let data = json.data(using: .utf8) else {
            throw TantivyError.decoding("native response was not valid UTF-8")
        }
        guard let object = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw TantivyError.decoding("native response was not a JSON object")
        }
        return object
    }
}
