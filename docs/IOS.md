# Tantivy Swift API (iOS / macOS)

`TantivyKit` is a Swift package that exposes the same Rust + Tantivy engine that
backs the Android bindings, this time over a C ABI instead of JNI. The native
code is identical; only the marshalling layer differs.

- The Rust crate is compiled as a **static library** and packaged as an
  **XCFramework** (`TantivyFFI.xcframework`) with iOS device, iOS simulator, and
  macОС slices.
- `TantivyKit` wraps the C ABI in a typed, memory-safe Swift API that mirrors the
  Kotlin `TantivyClient` / `TantivyIndex` design (schema/document/query builders,
  typed results, typed errors).

## Architecture

```
Swift app
  └── TantivyKit (Swift)                tantivy-ios/Sources/TantivyKit
        └── CTantivyFFI (C module)      include/tantivy_ffi.h + module.modulemap
              └── libtantivy_jni.a      staticlib built from src/ffi_bridge.rs
                    └── core ops        open_index / add_documents / search / ...
```

`src/ffi_bridge.rs` is the C-ABI sibling of `src/jni_bridge.rs`: both are thin
layers over the same core functions. JSON is the wire format across the boundary,
exactly as with JNI, so behavior stays consistent across platforms.

## Requirements

- Xcode 15+ (Swift 5.9+)
- Rust with the Apple targets:
  `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `x86_64-apple-ios`,
  and (for `swift test` on a Mac) `aarch64-apple-darwin`, `x86_64-apple-darwin`.
  `scripts/build-ios-native.sh` adds them automatically via `rustup`.

## Build the XCFramework

From the repository root:

```bash
scripts/build-ios-native.sh
```

This builds the static library for each Apple target and writes
`tantivy-ios/TantivyFFI.xcframework`. The framework is a build artifact and is
git-ignored. Set `BUILD_MACOS=0` to skip the macОС slice (it exists so the Swift
package is testable on a Mac without a simulator).

## Use the package

Add `tantivy-ios` as a local Swift package (File ▸ Add Package Dependencies ▸ Add
Local, or a `path:` dependency), then:

```swift
import TantivyKit

let schema = TantivyClient.schema { s in
    s.string("id")
    s.text("title", defaultSearch: true)
    s.i64("publishedAt", fast: true)
}

// Store the index in an app-owned directory (Caches or Application Support).
let dir = FileManager.default
    .urls(for: .cachesDirectory, in: .userDomainMask)[0]
    .appendingPathComponent("articles-index")

let index = try await TantivyClient.open(path: dir.path, schema: schema)

try await index.add(TantivyClient.document { d in
    d.string("id", "1")
    d.text("title", "Tantivy on iOS")
    d.i64("publishedAt", Int64(Date().timeIntervalSince1970 * 1000))
})
_ = try await index.commitAndRefresh()

let page = try await index.search(TantivyClient.query { q in
    q.query = "ios"
    q.selectedFields("id", "title")
    q.sortBy("publishedAt", .desc)
})

for hit in page.hits {
    if case .string(let id)? = hit.fields["id"]?.first {
        print("hit:", id)
    }
}

try index.close()
```

An in-memory index (nothing written to disk) is available for tests and
ephemeral use via `TantivyClient.inMemoryPath` (`":memory:"`).

## Threading

Native calls are synchronous and blocking. Every `TantivyIndex` method has a
synchronous throwing form and an `async` form. The `async` forms run the native
call on a per-index serial queue, so they never block the calling thread (use
them from the main actor freely) and calls against a single index never overlap.
The synchronous forms are exposed for callers that manage their own threading —
do not call them on the main thread.

## Errors

Failures throw `TantivyError`, whose cases map one-to-one to the native error
categories (and to the Android exception hierarchy):

| `TantivyError` | Native kind | Android exception |
| --- | --- | --- |
| `.schema` | `schema` | `SchemaException` |
| `.open` | `open` | `IndexOpenException` |
| `.write` | `write` | `WriteException` |
| `.search` | `search` | `SearchException` |
| `.closed` | `closed` | `TantivyIndexClosedException` |
| `.native` | `native` | `NativeLibraryException` |
| `.decoding` | — | (Swift-side response decoding failure) |

## Field values

`FieldValue` is a typed enum covering every Tantivy field kind: `.text`,
`.string`, `.i64`, `.u64`, `.f64`, `.bool`, `.bytes`, `.date` (marshalled as
epoch milliseconds), `.json` (any `JSONSerialization`-compatible value),
`.facet`, and `.ipAddr`. Search hits decode stored values back into the same
enum.

## Testing

```bash
cd tantivy-ios
swift test
```

`swift test` links the macОС slice of the XCFramework, so build it first with
`scripts/build-ios-native.sh`.
