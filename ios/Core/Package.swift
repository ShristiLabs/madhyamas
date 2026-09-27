// swift-tools-version:5.9
// MadhyamasCore — pure pairing/relay logic shared by the iOS app and its
// tunnel extension. No UIKit / NetworkExtension imports here: everything
// is unit-testable with plain `swift test` on the host.

import PackageDescription

let package = Package(
    name: "MadhyamasCore",
    platforms: [
        .macOS(.v13),
        .iOS(.v16),
    ],
    products: [
        .library(name: "MadhyamasCore", targets: ["MadhyamasCore"]),
    ],
    targets: [
        .target(name: "MadhyamasCore"),
        .testTarget(name: "MadhyamasCoreTests", dependencies: ["MadhyamasCore"]),
    ]
)
