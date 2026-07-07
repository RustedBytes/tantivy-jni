import XCTest
@testable import TantivyKit

final class TantivyKitTests: XCTestCase {
    private func makeSchema() -> IndexSchema {
        TantivyClient.schema { schema in
            schema.string("id")
            schema.text("title", defaultSearch: true)
            schema.i64("publishedAt", fast: true)
        }
    }

    private func openInMemory() throws -> TantivyIndex {
        try TantivyClient.open(path: TantivyClient.inMemoryPath, schema: makeSchema())
    }

    func testNativeVersionIsReported() {
        XCTAssertFalse(TantivyClient.nativeVersion.isEmpty)
        XCTAssertNotEqual(TantivyClient.nativeVersion, "unknown")
    }

    func testIndexAddCommitSearch() throws {
        let index = try openInMemory()
        defer { try? index.close() }

        let write = try index.add([
            TantivyClient.document { doc in
                doc.string("id", "1")
                doc.text("title", "Tantivy on iOS")
                doc.i64("publishedAt", 1_700_000_000_000)
            },
            TantivyClient.document { doc in
                doc.string("id", "2")
                doc.text("title", "Search engines in Rust")
                doc.i64("publishedAt", 1_700_000_100_000)
            },
        ])
        XCTAssertEqual(write.documentsAdded, 2)

        _ = try index.commitAndRefresh()

        let page = try index.search(
            TantivyClient.query { query in
                query.query = "tantivy"
                query.selectedFields("id", "title")
            }
        )
        XCTAssertEqual(page.totalHits, 1)
        XCTAssertEqual(page.hits.count, 1)

        let idValues = page.hits[0].fields["id"] ?? []
        guard case .string(let id)? = idValues.first else {
            return XCTFail("expected a string id field, got \(idValues)")
        }
        XCTAssertEqual(id, "1")
    }

    func testSortedSearchByFastField() throws {
        let index = try openInMemory()
        defer { try? index.close() }

        for i in 0..<3 {
            _ = try index.add(TantivyClient.document { doc in
                doc.string("id", "\(i)")
                doc.text("title", "rust document \(i)")
                doc.i64("publishedAt", Int64(1_700_000_000_000 + i))
            })
        }
        _ = try index.commitAndRefresh()

        let page = try index.search(TantivyClient.query { query in
            query.query = "rust"
            query.sortBy("publishedAt", .desc)
            query.selectedFields("id")
        })
        XCTAssertEqual(page.totalHits, 3)
        let ids = page.hits.compactMap { hit -> String? in
            if case .string(let id)? = hit.fields["id"]?.first { return id }
            return nil
        }
        XCTAssertEqual(ids, ["2", "1", "0"])
    }

    func testDeleteTerm() throws {
        let index = try openInMemory()
        defer { try? index.close() }

        _ = try index.add(TantivyClient.document { doc in
            doc.string("id", "keep")
            doc.text("title", "keep me")
        })
        _ = try index.add(TantivyClient.document { doc in
            doc.string("id", "drop")
            doc.text("title", "remove me")
        })
        _ = try index.commitAndRefresh()

        let deleted = try index.deleteTerm(field: "id", value: .string("drop"))
        XCTAssertEqual(deleted.termsDeleted, 1)
        _ = try index.commitAndRefresh()

        let page = try index.search(TantivyClient.query { $0.query = "me" })
        XCTAssertEqual(page.totalHits, 1)
    }

    func testSchemaInfo() throws {
        let index = try openInMemory()
        defer { try? index.close() }

        let info = try index.schemaInfo()
        XCTAssertEqual(Set(info.fields.map(\.name)), ["id", "title", "publishedAt"])
        XCTAssertEqual(info.defaultSearchFields, ["title"])
        let publishedAt = info.fields.first { $0.name == "publishedAt" }
        XCTAssertEqual(publishedAt?.type, .i64)
        XCTAssertEqual(publishedAt?.fast, true)
    }

    func testErrorsAreTyped() throws {
        let index = try openInMemory()
        defer { try? index.close() }

        // Unknown field on a document is a write error.
        XCTAssertThrowsError(
            try index.add(TantivyClient.document { $0.text("nope", "x") })
        ) { error in
            guard case TantivyError.write = error else {
                return XCTFail("expected .write, got \(error)")
            }
        }
    }

    func testAsyncRoundTrip() async throws {
        let index = try await TantivyClient.open(
            path: TantivyClient.inMemoryPath,
            schema: makeSchema()
        )
        defer { try? index.close() }

        _ = try await index.add(TantivyClient.document { doc in
            doc.string("id", "1")
            doc.text("title", "async tantivy")
        })
        _ = try await index.commitAndRefresh()
        let page = try await index.search(TantivyClient.query { $0.query = "async" })
        XCTAssertEqual(page.totalHits, 1)
    }
}
