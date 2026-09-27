import Foundation

/// The paired device credential: the long-lived `mdy_dev_...` key plus
/// display metadata returned by the enroll endpoint (or carried by the QR
/// payload in manual-key mode).
public struct PairedCredential: Equatable, Codable {
    public let key: String
    public let deviceId: String?
    public let deviceName: String?

    public init(key: String, deviceId: String?, deviceName: String?) {
        self.key = key
        self.deviceId = deviceId
        self.deviceName = deviceName
    }
}

/// Storage abstraction for the device credential. On iOS the Keychain is
/// the implementation (already encrypted at rest — unlike Android's
/// SharedPreferences, no AES-GCM sealing layer is needed); tests use an
/// in-memory fake.
public protocol SecretStore: AnyObject {
    /// Persists the device key plus metadata. Overwrites any previous
    /// credential (single-device pairing, like the Android app).
    func saveCredential(_ credential: PairedCredential) throws
    func loadCredential() -> PairedCredential?
    func clearCredential() throws
}

/// Enrollment request/response bodies for `POST {api}/devices/enroll`.
/// The request schema (`EnrollDeviceRequest` server-side) has exactly one
/// field: the token.
struct EnrollRequestBody: Codable {
    let token: String
}

/// `{"device":{"id","name",...},"key":"mdy_dev_..."}` — decoded leniently
/// via JSONSerialization in `URLEnrollmentClient.parseResponse` (the
/// server object carries more fields than we consume).
public struct EnrollResponse {
    public let deviceId: String?
    public let deviceName: String?
    public let key: String
}
