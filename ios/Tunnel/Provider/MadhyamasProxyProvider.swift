import Foundation
import NetworkExtension
import MadhyamasCore

/// Per-app VPN proxy provider — the iOS counterpart of the Android
/// `MadhyamasVpnService`.
///
/// The system hands us fully-established TCP flows for the selected apps
/// (`NEAppRule` per-app routing configured by `VpnManager`); each flow is
/// re-originated to the Madhyamas proxy with an app-authored CONNECT
/// carrying the paired device credential. UDP flows (QUIC, DNS) are denied,
/// matching the Android app's IPv4/TCP-only behavior.
final class MadhyamasProxyProvider: NEAppProxyProvider {

    /// App Group shared with the containing app.
    private static let appGroup = "group.com.madhyamas.shared"

    /// Concurrent-flow cap: provider extensions live under a ~50 MB jetsam
    /// limit, so overflow flows are denied instead of crashing the provider.
    private static let maxConcurrentFlows = 64

    private let queue = DispatchQueue(label: "com.madhyamas.tunnel.provider")
    private var configStore: ConfigStore!
    private var config: ProxyConfig?
    private var proxyAuthorization: String?
    private var breaker = CircuitBreaker()
    private var flows: [ObjectIdentifier: FlowPipe] = [:]

    override func startProxy(options: [String: Any]?, completionHandler: @escaping (Error?) -> Void) {
        configStore = ConfigStore(store: UserDefaultsStore(suiteName: Self.appGroup))
        config = configStore.loadConfig()
        breaker = configStore.loadBreaker()

        if let credential = KeychainStore().loadCredential() {
            proxyAuthorization = ProxyAuth.basicHeaderValue(deviceKey: credential.key)
        }
        if proxyAuthorization == nil {
            completionHandler(ProviderError.notPaired)
            return
        }
        completionHandler(nil)
    }

    override func stopProxy(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        for pipe in flows.values {
            pipe.stop()
        }
        flows.removeAll()
        completionHandler()
    }

    override func handleNewFlow(_ flow: NEAppProxyFlow) -> Bool {
        guard let tcpFlow = flow as? NEAppProxyTCPFlow else {
            return false // UDP/QUIC not captured (parity with Android)
        }
        guard !breaker.tripped else {
            return false // 3×407 circuit breaker: fail fast
        }
        guard let config, let proxyAuthorization else {
            return false
        }
        guard flows.count < Self.maxConcurrentFlows else {
            return false
        }

        let pipe = FlowPipe(flow: tcpFlow, config: config,
                            proxyAuthorization: proxyAuthorization, queue: queue) { [weak self] outcome in
            self?.handleOutcome(outcome)
        } onClosed: { [weak self] pipe in
            self?.flows.removeValue(forKey: ObjectIdentifier(pipe))
        }
        flows[ObjectIdentifier(pipe)] = pipe
        pipe.start()
        return true
    }

    private func handleOutcome(_ outcome: FlowPipe.Outcome) {
        switch outcome {
        case .established:
            breaker.recordSuccess()
        case .rejected:
            breaker.recordRejection()
        case .failed, .unreachable, .tlsError:
            break
        }
        configStore.saveBreaker(breaker)
    }

    enum ProviderError: LocalizedError {
        case notPaired
        var errorDescription: String? {
            switch self {
            case .notPaired:
                return "No paired device credential in the Keychain. Pair from the app first."
            }
        }
    }
}
