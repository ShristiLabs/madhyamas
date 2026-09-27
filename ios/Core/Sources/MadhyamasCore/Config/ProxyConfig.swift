import Foundation

/// Persistent proxy configuration — iOS counterpart of the Android
/// `ProxyConfig`. `selectedBundleIDs` replaces Android's
/// `selectedPackages` (iOS cannot enumerate installed apps, so the list is
/// managed manually).
public struct ProxyConfig: Codable, Equatable {
    public var proxyHost: String
    public var proxyPort: Int
    /// Full API base URL from the QR payload (`api=`), e.g.
    /// `https://host/api`. Used for enrollment and CA download.
    public var apiBaseUrl: String?
    /// QR payload tls=1 (issue #110): TLS-wrap the proxy connection.
    public var useTls: Bool
    /// Device name from the QR payload (display metadata).
    public var deviceName: String?
    /// Per-app capture: bundle identifiers routed through the tunnel.
    public var selectedBundleIDs: Set<String>

    public init(proxyHost: String = "127.0.0.1",
                proxyPort: Int = 8888,
                apiBaseUrl: String? = nil,
                useTls: Bool = false,
                deviceName: String? = nil,
                selectedBundleIDs: Set<String> = []) {
        self.proxyHost = proxyHost
        self.proxyPort = proxyPort
        self.apiBaseUrl = apiBaseUrl
        self.useTls = useTls
        self.deviceName = deviceName
        self.selectedBundleIDs = selectedBundleIDs
    }
}
