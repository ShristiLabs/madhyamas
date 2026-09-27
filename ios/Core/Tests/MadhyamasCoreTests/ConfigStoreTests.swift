import XCTest
@testable import MadhyamasCore

/// iOS counterpart of android CredentialStoreTest — covers the shared
/// persistence layer (ConfigStore over a KeyValueStore; the credential
/// itself lives in the Keychain inside the app target, which host `swift
/// test` cannot reach).
final class ConfigStoreTests: XCTestCase {

    final class InMemoryStore: KeyValueStore {
        var map: [String: Data] = [:]
        func data(forKey key: String) -> Data? { map[key] }
        func set(_ value: Data?, forKey key: String) { map[key] = value }
    }

    func testLoadReturnsNilWhenEmpty() {
        XCTAssertNil(ConfigStore(store: InMemoryStore()).loadConfig())
    }

    func testSaveLoadRoundTrip() {
        let store = InMemoryStore()
        let configStore = ConfigStore(store: store)
        var config = ProxyConfig(proxyHost: "madhyamas-proxy.shristilabs.com", proxyPort: 8888,
                                 apiBaseUrl: "https://madhyamas-demo.shristilabs.com/api",
                                 useTls: true, deviceName: "Hari's iPhone")
        config.selectedBundleIDs = ["com.apple.mobilesafari"]
        configStore.saveConfig(config)
        XCTAssertEqual(config, configStore.loadConfig())
    }

    func testSaveOverwritesPrevious() {
        let store = InMemoryStore()
        let configStore = ConfigStore(store: store)
        configStore.saveConfig(ProxyConfig(proxyHost: "a", proxyPort: 1))
        configStore.saveConfig(ProxyConfig(proxyHost: "b", proxyPort: 2))
        XCTAssertEqual("b", configStore.loadConfig()?.proxyHost)
    }

    func testClearConfig() {
        let store = InMemoryStore()
        let configStore = ConfigStore(store: store)
        configStore.saveConfig(ProxyConfig())
        configStore.clearConfig()
        XCTAssertNil(configStore.loadConfig())
    }

    func testCorruptDataYieldsNil() {
        let store = InMemoryStore()
        store.map["proxy_config"] = Data("garbage".utf8)
        XCTAssertNil(ConfigStore(store: store).loadConfig())
    }

    func testBreakerRoundTripAndDefault() {
        let store = InMemoryStore()
        let configStore = ConfigStore(store: store)
        XCTAssertEqual(CircuitBreaker(), configStore.loadBreaker(), "defaults when absent")
        var breaker = CircuitBreaker()
        for _ in 0..<3 { breaker.recordRejection() }
        configStore.saveBreaker(breaker)
        XCTAssertTrue(configStore.loadBreaker().tripped)
    }

    func testUserDefaultsStoreWithUniqueSuite() throws {
        let suite = "test.madhyamas.\(UUID().uuidString)"
        let store = UserDefaultsStore(suiteName: suite)
        let configStore = ConfigStore(store: store)
        configStore.saveConfig(ProxyConfig(proxyHost: "h", proxyPort: 9))
        XCTAssertEqual("h", configStore.loadConfig()?.proxyHost)
        configStore.clearConfig()
        XCTAssertNil(configStore.loadConfig())
    }
}
