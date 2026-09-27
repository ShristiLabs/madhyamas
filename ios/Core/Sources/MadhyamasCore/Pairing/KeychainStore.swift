import Foundation
import Security

/// `SecretStore` backed by the iOS/macOS Keychain — the counterpart of the
/// Android Keystore-sealed CredentialStore. The Keychain is already
/// encrypted at rest, so no AES-GCM sealing layer is needed.
///
/// The whole `PairedCredential` (key + display metadata) is stored as one
/// JSON item. When the app is provisioned with a team, pass the shared
/// `keychainAccessGroup` (e.g. `"$(AppIdentifierPrefix)com.madhyamas.shared"`)
/// so the tunnel extension can read the credential too; nil keeps the item
/// in the default (target-local) access group, which unsigned simulator
/// builds require.
public final class KeychainStore: SecretStore {

    private let service: String
    private let account: String
    private let keychainAccessGroup: String?

    public init(service: String = "com.madhyamas.credential",
                account: String = "device_key",
                keychainAccessGroup: String? = nil) {
        self.service = service
        self.account = account
        self.keychainAccessGroup = keychainAccessGroup
    }

    private var baseQuery: [String: Any] {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
        ]
        if let keychainAccessGroup {
            query[kSecAttrAccessGroup as String] = keychainAccessGroup
        }
        return query
    }

    public func saveCredential(_ credential: PairedCredential) throws {
        var attributes = baseQuery
        attributes[kSecValueData as String] = try JSONEncoder().encode(credential)
        attributes[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        let status = SecItemAdd(attributes as CFDictionary, nil)
        guard status == errSecSuccess else {
            if status == errSecDuplicateItem {
                try clearCredential()
                return try saveCredential(credential)
            }
            throw NSError(domain: NSOSStatusErrorDomain, code: Int(status))
        }
    }

    public func loadCredential() -> PairedCredential? {
        var query = baseQuery
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var item: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &item) == errSecSuccess,
              let data = item as? Data else {
            return nil
        }
        return try? JSONDecoder().decode(PairedCredential.self, from: data)
    }

    public func clearCredential() throws {
        let status = SecItemDelete(baseQuery as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw NSError(domain: NSOSStatusErrorDomain, code: Int(status))
        }
    }
}
