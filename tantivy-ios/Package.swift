// swift-tools-version:5.9
import PackageDescription

// TantivyKit — Swift bindings for Tantivy on iOS/macOS.
//
// The native code is the same Rust crate that powers the Android JNI bindings,
// compiled as a static library and packaged as `TantivyFFI.xcframework`. Build
// (or refresh) that framework with `scripts/build-ios-native.sh` before
// resolving this package.
let package = Package(
    name: "TantivyKit",
    platforms: [
        .iOS(.v13),
        .macOS(.v11),
    ],
    products: [
        .library(name: "TantivyKit", targets: ["TantivyKit"]),
    ],
    targets: [
        .binaryTarget(
            name: "CTantivyFFI",
            path: "TantivyFFI.xcframework"
        ),
        .target(
            name: "TantivyKit",
            dependencies: ["CTantivyFFI"]
        ),
        .testTarget(
            name: "TantivyKitTests",
            dependencies: ["TantivyKit"]
        ),
    ]
)
