import Foundation
import NetworkExtension
import MadhyamasCore

/// App Group shared with the tunnel extension.
let appGroupID = "group.com.madhyamas.shared"

/// Mirrors the Android `MainViewModel`: deep-link pairing (key or
/// enrollment-token mode), credential persistence, config persistence, and
/// VPN lifecycle.
@MainActor
final class MainViewModel: ObservableObject {

    enum EnrollState: Equatable {
        case idle
        case enrolling
        case failed(String)
    }

    @Published var paired: PairedCredential?
    @Published var config: ProxyConfig
    @Published var enrollState: EnrollState = .idle
    @Published var pairingMessage: String?
    @Published var vpnActive = false
    @Published var breakerTripped = false

    private let secretStore: SecretStore
    private let configStore: ConfigStore
    private let enrollmentClient: EnrollmentClient
    private let vpn: VpnManager

    init(secretStore: SecretStore = KeychainStore(),
         configStore: ConfigStore = ConfigStore(store: UserDefaultsStore(suiteName: appGroupID)),
         enrollmentClient: EnrollmentClient = URLEnrollmentClient(),
         vpnManager: VpnManager? = nil) {
        self.secretStore = secretStore
        self.configStore = configStore
        self.enrollmentClient = enrollmentClient
        self.vpn = vpnManager ?? VpnManager()
        self.config = configStore.loadConfig() ?? ProxyConfig()
        self.paired = secretStore.loadCredential()
        self.breakerTripped = configStore.loadBreaker().tripped
    }

    // MARK: - deep link pairing

    func handleDeepLink(_ url: URL) {
        pairingMessage = nil
        switch ConnectUriParser.parse(url.absoluteString) {
        case .error(let reason):
            pairingMessage = reason
        case .ok(let payload):
            config.proxyHost = payload.host
            config.proxyPort = payload.port
            config.useTls = payload.tls
            config.apiBaseUrl = payload.apiUrl ?? config.apiBaseUrl
            config.deviceName = payload.name
            configStore.saveConfig(config)

            if let key = payload.key {
                let credential = PairedCredential(key: key, deviceId: nil, deviceName: payload.name)
                store(credential)
            } else if let token = payload.token, let api = payload.apiUrl {
                redeem(token: token, api: api)
            }
        }
    }

    private func redeem(token: String, api: String) {
        enrollState = .enrolling
        Task { [weak self] in
            guard let self else { return }
            let result = await self.enrollmentClient.enroll(apiBaseUrl: api, token: token)
            switch result {
            case .success(let credential):
                self.store(credential)
                self.enrollState = .idle
            case .failure(let reason):
                self.enrollState = .failed(Self.describe(reason))
            }
        }
    }

    private func store(_ credential: PairedCredential) {
        do {
            try secretStore.saveCredential(credential)
        } catch {
            enrollState = .failed("Could not store the device credential: \(error.localizedDescription)")
            return
        }
        paired = credential
        pairingMessage = "Paired as \(credential.deviceName ?? "device")"
    }

    private static func describe(_ reason: EnrollmentFailure) -> String {
        switch reason {
        case .invalidToken:
            return "The enrollment token was rejected (unknown, expired, or already used)."
        case .malformedToken:
            return "The enrollment token is malformed."
        case .serverError:
            return "The server returned an unexpected response."
        case .network:
            return "Could not reach the server. Check your connection."
        case .badResponse:
            return "The server returned an unreadable response."
        }
    }

    // MARK: - manual config

    func saveConfig() {
        configStore.saveConfig(config)
    }

    // MARK: - VPN lifecycle

    func startVPN() {
        Task { [weak self] in
            guard let self else { return }
            do {
                try await self.vpn.activate()
                self.vpnActive = true
            } catch {
                self.pairingMessage = "Could not start the VPN: \(error.localizedDescription)"
            }
        }
    }

    func stopVPN() {
        vpn.deactivate()
        vpnActive = false
    }

    func forget() {
        stopVPN()
        try? secretStore.clearCredential()
        paired = nil
    }

    func refreshStatus() {
        vpnActive = vpn.isActive
        breakerTripped = ConfigStore(store: UserDefaultsStore(suiteName: appGroupID)).loadBreaker().tripped
    }
}
