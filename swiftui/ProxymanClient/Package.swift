// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "ProxymanClient",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .executable(name: "ProxymanClient", targets: ["ProxymanClient"])
    ],
    targets: [
        .executableTarget(name: "ProxymanClient")
    ]
)
