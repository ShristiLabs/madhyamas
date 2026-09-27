import Foundation
import NetworkExtension

/// Creates/updates the VPN configuration (`NETunnelProviderManager` backed
/// by the app-proxy provider) and starts/stops the tunnel.
///
/// NOTE: per-app routing (`appRules`/`NEAppRule`) is macOS-only API — on
/// iOS it is only settable via MDM. This app therefore runs route-all:
/// while the VPN is on, ALL TCP flows are offered to the provider (UDP is
/// denied by the provider; see MadhyamasProxyProvider).
@MainActor
final class VpnManager: ObservableObject {

    static let tunnelDescription = "Madhyamas"
    static let providerBundleID = "com.madhyamas.ios.tunnel"

    private(set) var manager: NETunnelProviderManager?
    private var statusObserver: NSObjectProtocol?

    var isActive: Bool {
        manager?.connection.status == .connected || manager?.connection.status == .connecting
    }

    init() {
        statusObserver = NotificationCenter.default.addObserver(
            forName: .NEVPNStatusDidChange, object: nil, queue: .main) { _ in
                // The UI polls on foreground/status refresh; nothing to do here.
        }
    }

    deinit {
        if let statusObserver {
            NotificationCenter.default.removeObserver(statusObserver)
        }
    }

    func activate() async throws {
        let manager = try await loadOrCreateManager()
        let protocolConfig = tunnelProtocol(existing: manager.protocolConfiguration as? NETunnelProviderProtocol)
        manager.protocolConfiguration = protocolConfig
        manager.localizedDescription = Self.tunnelDescription
        manager.isOnDemandEnabled = false
        try await manager.saveToPreferences()
        try await manager.loadFromPreferences()
        try manager.connection.startVPNTunnel()
        self.manager = manager
    }

    func deactivate() {
        manager?.connection.stopVPNTunnel()
    }

    private func loadOrCreateManager() async throws -> NETunnelProviderManager {
        let managers = try await NETunnelProviderManager.loadAllFromPreferences()
        if let existing = managers.first(where: { $0.localizedDescription == Self.tunnelDescription }) {
            self.manager = existing
            return existing
        }
        let created = NETunnelProviderManager()
        self.manager = created
        return created
    }

    private func tunnelProtocol(existing: NETunnelProviderProtocol?) -> NETunnelProviderProtocol {
        let config = existing ?? NETunnelProviderProtocol()
        config.providerBundleIdentifier = Self.providerBundleID
        config.serverAddress = "Madhyamas"
        return config
    }
}
