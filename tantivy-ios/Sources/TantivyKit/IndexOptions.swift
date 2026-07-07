import Foundation

/// Options controlling how an index is opened and how its writer is configured.
public struct IndexOptions: Sendable {
    /// Create the index if it does not already exist on disk.
    public var create: Bool
    /// Number of writer threads (1...validated max).
    public var writerThreads: Int
    /// Writer memory budget in bytes.
    public var writerMemoryBytes: Int

    public init(
        create: Bool = true,
        writerThreads: Int = 1,
        writerMemoryBytes: Int = 50_000_000
    ) {
        self.create = create
        self.writerThreads = writerThreads
        self.writerMemoryBytes = writerMemoryBytes
    }

    func toWire() -> [String: Any] {
        [
            "create": create,
            "writerThreads": writerThreads,
            "writerMemoryBytes": writerMemoryBytes,
        ]
    }

    func toJSON() throws -> String {
        try JSONCoding.encode(toWire())
    }
}
