import Foundation

/// Parsed `madhyamas://connect` QR payload (issue #111).
///
/// Payload shape (built by the web Devices panel, issue #106):
/// `madhyamas://connect?host=H&port=P&tls=0|1&token=mdy_enroll_...` or
/// `&key=mdy_dev_...`, plus `&name=`, `&ca=`, `&api=` (URL-encoded).
///
/// Exactly one of `token` (enrollment exchange) and `key` (manual mode)
/// is non-nil.
public struct ConnectPayload: Equatable {
    public let host: String
    public let port: Int
    public let tls: Bool
    public let name: String?
    public let caUrl: String?
    public let apiUrl: String?
    public let token: String?
    public let key: String?

    public init(host: String, port: Int, tls: Bool, name: String?,
                caUrl: String?, apiUrl: String?, token: String?, key: String?) {
        self.host = host
        self.port = port
        self.tls = tls
        self.name = name
        self.caUrl = caUrl
        self.apiUrl = apiUrl
        self.token = token
        self.key = key
    }

    public var isEnrollment: Bool { token != nil }
}

public enum ConnectParseResult: Equatable {
    case ok(ConnectPayload)
    case error(String)
}

/// Pure parser for the `madhyamas://connect` deep link — a verbatim port
/// of the Android `ConnectUriParser` (android/.../pairing/ConnectUriParser.kt).
///
/// Malformed or incomplete links produce `.error` with a
/// user-presentable reason — never an exception, never a crash.
/// Validation mirrors the server contract from issues #104/#106/#110:
///  - `host` non-blank, `port` in 1...65535, `tls` strictly "0" or "1"
///  - exactly one credential: `token` (`mdy_enroll_` prefix, redeemed via
///    the enroll API) or `key` (`mdy_dev_` prefix, stored directly)
///  - token links must carry `api` (the base URL of the enroll endpoint)
///  - `ca`/`api`, when present, must be http(s) URLs
public enum ConnectUriParser {

    public static let scheme = "madhyamas"
    public static let linkHost = "connect"

    public static let enrollmentTokenPrefix = "mdy_enroll_"
    public static let deviceKeyPrefix = "mdy_dev_"

    public static func parse(_ raw: String?) -> ConnectParseResult {
        guard let raw, !raw.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return .error("Empty link")
        }
        guard let url = URL(string: raw) else {
            return .error("Malformed link")
        }
        guard let urlScheme = url.scheme?.lowercased(), urlScheme == scheme else {
            return .error("Not a Madhyamas connect link")
        }
        guard let urlHost = url.host?.lowercased(), urlHost == linkHost else {
            return .error("Not a Madhyamas connect link")
        }
        // java.net.URI rejects links with invalid percent-escapes (`%zz`);
        // Foundation tolerates them, so police it ourselves for parity.
        guard !hasInvalidPercentEscape(raw) else {
            return .error("Malformed link")
        }

        let rawQuery = URLComponents(string: raw)?.percentEncodedQuery
        let params = parseQuery(rawQuery)

        let host = (params["host"] ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        if host.isEmpty {
            return .error("Link is missing the proxy host")
        }

        guard let portText = params["port"]?.trimmingCharacters(in: .whitespacesAndNewlines),
              !portText.isEmpty else {
            return .error("Link is missing the proxy port")
        }
        guard let port = Int(portText), (1...65535).contains(port) else {
            return .error("Proxy port out of range (1-65535)")
        }

        let tlsText = params["tls"] ?? "0"
        let tls: Bool
        switch tlsText {
        case "0": tls = false
        case "1": tls = true
        default: return .error("tls must be 0 or 1")
        }

        func trimmedNonEmpty(_ key: String) -> String? {
            guard let v = params[key]?.trimmingCharacters(in: .whitespacesAndNewlines), !v.isEmpty else { return nil }
            return v
        }
        let token = trimmedNonEmpty("token")
        let key = trimmedNonEmpty("key")
        switch (token, key) {
        case (.some, .some):
            return .error("Link carries both a token and a key")
        case (nil, nil):
            return .error("Link carries no credential (token or key)")
        case (.some(let t), nil) where !t.hasPrefix(enrollmentTokenPrefix):
            return .error("Unexpected enrollment token format")
        case (nil, .some(let k)) where !k.hasPrefix(deviceKeyPrefix):
            return .error("Unexpected device key format")
        default:
            break
        }

        // A token link must say where to redeem it.
        if token != nil, (params["api"] ?? "").trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return .error("Token links must include the API URL")
        }

        let name = trimmedNonEmpty("name")
        var caUrl: String? = nil
        if let ca = trimmedNonEmpty("ca") {
            guard isHttpUrl(ca) else { return .error("ca parameter is not an http(s) URL") }
            caUrl = ca
        }
        var apiUrl: String? = nil
        if let api = trimmedNonEmpty("api") {
            guard isHttpUrl(api) else { return .error("api parameter is not an http(s) URL") }
            apiUrl = api
        }

        return .ok(ConnectPayload(host: host, port: port, tls: tls, name: name,
                                  caUrl: caUrl, apiUrl: apiUrl, token: token, key: key))
    }

    /// Manual `&`/`=` query parser with URL-decoding (and `+`-as-space,
    /// matching java.net.URLDecoder). Invalid escapes fall back to the raw
    /// text instead of failing.
    static func parseQuery(_ rawQuery: String?) -> [String: String] {
        guard let rawQuery, !rawQuery.isEmpty else { return [:] }
        var params: [String: String] = [:]
        for pair in rawQuery.split(separator: "&", omittingEmptySubsequences: true) {
            let s = String(pair)
            guard let eq = s.firstIndex(of: "="), eq != s.startIndex else { continue }
            let name = String(s[s.startIndex..<eq])
            let rawValue = String(s[s.index(after: eq)...])
            params[name] = decode(rawValue)
        }
        return params
    }

    static func decode(_ value: String) -> String {
        let plusToSpace = value.replacingOccurrences(of: "+", with: " ")
        return plusToSpace.removingPercentEncoding ?? value
    }

    static func isHttpUrl(_ value: String) -> Bool {
        guard let url = URL(string: value), let scheme = url.scheme?.lowercased() else { return false }
        return scheme == "http" || scheme == "https"
    }

    /// True when the string contains a `%` not followed by two hex digits.
    static func hasInvalidPercentEscape(_ value: String) -> Bool {
        let chars = Array(value.unicodeScalars)
        var i = 0
        while i < chars.count {
            if chars[i] == "%" {
                guard i + 2 < chars.count,
                      isHexDigit(chars[i + 1]), isHexDigit(chars[i + 2]) else {
                    return true
                }
                i += 3
            } else {
                i += 1
            }
        }
        return false
    }

    private static func isHexDigit(_ scalar: Unicode.Scalar) -> Bool {
        (0x30...0x39).contains(scalar.value)
            || (0x41...0x46).contains(scalar.value)
            || (0x61...0x66).contains(scalar.value)
    }
}
