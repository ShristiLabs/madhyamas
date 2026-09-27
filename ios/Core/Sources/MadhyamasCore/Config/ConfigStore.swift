import Foundation

/// Minimal persistence abstraction so `ConfigStore` is testable without
/// UserDefaults suites. The app and the tunnel extension share one via an
/// App Group.
public protocol KeyValueStore: AnyObject {
    func data(forKey key: String) -> Data?
    func set(_ value: Data?, forKey key: String)
}

public final class UserDefaultsStore: KeyValueStore {
    private let defaults: UserDefaults

    public init(suiteName: String?) {
        if let suiteName, let defaults = UserDefaults(suiteName: suiteName) {
            self.defaults = defaults
        } else {
            self.defaults = .standard
        }
    }

    public func data(forKey key: String) -> Data? {
        defaults.data(forKey: key)
    }

    public func set(_ value: Data?, forKey key: String) {
        defaults.set(value, forKey: key)
    }
}

/// Persists `ProxyConfig` (and the `CircuitBreaker` snapshot the tunnel
/// extension writes) to a key-value store — the App Group in production.
public final class ConfigStore {
    public let store: KeyValueStore
    public let configKey: String
    public let breakerKey: String

    public init(store: KeyValueStore,
                configKey: String = "proxy_config",
                breakerKey: String = "breaker_state") {
        self.store = store
        self.configKey = configKey
        self.breakerKey = breakerKey
    }

    public func loadConfig() -> ProxyConfig? {
        guard let data = store.data(forKey: configKey) else { return nil }
        return try? JSONDecoder().decode(ProxyConfig.self, from: data)
    }

    public func saveConfig(_ config: ProxyConfig) {
        if let data = try? JSONEncoder().encode(config) {
            store.set(data, forKey: configKey)
        }
    }

    public func clearConfig() {
        store.set(nil, forKey: configKey)
    }

    public func loadBreaker() -> CircuitBreaker {
        guard let data = store.data(forKey: breakerKey),
              let breaker = try? JSONDecoder().decode(CircuitBreaker.self, from: data) else {
            return CircuitBreaker()
        }
        return breaker
    }

    public func saveBreaker(_ breaker: CircuitBreaker) {
        if let data = try? JSONEncoder().encode(breaker) {
            store.set(data, forKey: breakerKey)
        }
    }
}
