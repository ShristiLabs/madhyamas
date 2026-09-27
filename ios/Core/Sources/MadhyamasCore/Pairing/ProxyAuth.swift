import Foundation

/// Builds the `Proxy-Authorization` header the companion attaches to every
/// CONNECT it authors (issue #111) — verbatim port of the Android
/// `ProxyAuth`.
///
/// The device key (`mdy_dev_...`) travels as the Basic *username* with an
/// empty password: the Madhyamas proxy accepts the key in either Basic half
/// (enterprise auth.rs checks the username half first — the manual-apply /
/// iOS precedent), and `Basic base64(key + ":")` is the canonical form.
///
/// Because the companion *re-originates* every TCP connection to the proxy
/// and writes its own CONNECT request, an app's own proxy-auth headers
/// (sent by proxy-aware apps into the tunnel) can never reach the proxy's
/// CONNECT parser — the companion's credential wins by construction.
public enum ProxyAuth {

    public static let headerName = "Proxy-Authorization"

    public static func basicHeaderValue(deviceKey: String) -> String {
        let raw = Data("\(deviceKey):".utf8)
        return "Basic " + raw.base64EncodedString()
    }
}
