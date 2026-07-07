# TantivyKit

Swift bindings for [Tantivy](https://github.com/quickwit-oss/tantivy) on iOS and
macOS, backed by the same Rust core as the Android Kotlin bindings.

## Quick start

From the repository root, build the native framework this package links against:

```bash
scripts/build-ios-native.sh
```

This writes `TantivyFFI.xcframework` next to this README (it is a git-ignored
build artifact). Then build and test the package:

```bash
swift build
swift test
```

Add it to an app as a local Swift package (a `path:` dependency pointing at this
directory, or File ▸ Add Package Dependencies ▸ Add Local in Xcode).

```swift
import TantivyKit

let schema = TantivyClient.schema { s in
    s.string("id")
    s.text("title", defaultSearch: true)
}
let index = try await TantivyClient.open(path: indexDir.path, schema: schema)
```

See [../docs/IOS.md](../docs/IOS.md) for the full guide.
