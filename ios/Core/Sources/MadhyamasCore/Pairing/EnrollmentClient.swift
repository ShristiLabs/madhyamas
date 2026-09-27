import Foundation

/// Outcome of redeeming an enrollment token (issue #111). On success the
/// server returns the long-lived `mdy_dev_` key exactly once.
public enum EnrollmentFailure: Equatable {
    /// Server rejected the token: unknown, expired, already used, or
    /// revoked (indistinguishable by design).
    case invalidToken
    /// Token not shaped like an enrollment credential (HTTP 400).
    case malformedToken
    /// Any other non-200 server response.
    case serverError
    /// Connection/timeout failure.
    case network
    /// 200 with an unparseable body or a key that is not mdy_dev_-shaped.
    case badResponse
}

public enum EnrollmentResult: Equatable {
    case success(PairedCredential)
    case failure(EnrollmentFailure)
}

/// Redeems the QR's single-use `mdy_enroll_` token via the public enroll
/// endpoint from issue #106. The request body carries ONLY the token.
public protocol EnrollmentClient {
    func enroll(apiBaseUrl: String, token: String) async -> EnrollmentResult
}

/// `EnrollmentClient` over `URLSession`. Follows the `api` base URL from
/// the QR payload verbatim (http or https). No retries — a rejected token
/// is never retried with the same credential.
public final class URLEnrollmentClient: EnrollmentClient {

    private let session: URLSession

    public init(session: URLSession? = nil) {
        if let session {
            self.session = session
        } else {
            let config = URLSessionConfiguration.ephemeral
            // Android used connect 10s / read 20s; URLSession exposes a
            // per-request idle timeout (≈ read) and an overall resource
            // timeout. 20s/30s preserves the same envelope.
            config.timeoutIntervalForRequest = 20
            config.timeoutIntervalForResource = 30
            self.session = URLSession(configuration: config)
        }
    }

    public func enroll(apiBaseUrl: String, token: String) async -> EnrollmentResult {
        let url: URL
        do {
            url = try URL(string: Self.enrollUrl(apiBaseUrl)).orThrow()
        } catch {
            return .failure(.badResponse)
        }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.httpBody = try? JSONEncoder().encode(["token": token])

        let data: Data
        let response: URLResponse
        do {
            (data, response) = try await session.data(for: request)
        } catch {
            return .failure(.network)
        }
        guard let http = response as? HTTPURLResponse else {
            return .failure(.serverError)
        }
        switch http.statusCode {
        case 200:
            return Self.parseResponse(data)
        case 400:
            return .failure(.malformedToken)
        case 401:
            return .failure(.invalidToken)
        default:
            return .failure(.serverError)
        }
    }

    /// `http://host:3001/api` -> `http://host:3001/api/devices/enroll`.
    public static func enrollUrl(_ apiBaseUrl: String) -> String {
        let trimmed = apiBaseUrl.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.trimmingCharacters(in: CharacterSet(charactersIn: "/")) + "/devices/enroll"
    }

    /// Parses the `DeviceWithKey` response:
    /// `{"device":{"id","name",...},"key":"mdy_dev_..."}`.
    /// A 200 body whose key is not mdy_dev_-shaped is treated as a bad
    /// response (defensive — never store an unexpected credential).
    public static func parseResponse(_ body: Data) -> EnrollmentResult {
        guard let json = (try? JSONSerialization.jsonObject(with: body)) as? [String: Any] else {
            return .failure(.badResponse)
        }
        guard let key = json["key"] as? String,
              key.hasPrefix(ConnectUriParser.deviceKeyPrefix) else {
            return .failure(.badResponse)
        }
        let device = json["device"] as? [String: Any]
        func nonBlank(_ field: String) -> String? {
            guard let v = device?[field] as? String, !v.trimmingCharacters(in: .whitespaces).isEmpty else { return nil }
            return v
        }
        return .success(PairedCredential(key: key, deviceId: nonBlank("id"), deviceName: nonBlank("name")))
    }
}

private extension Optional {
    func orThrow() throws -> Wrapped {
        guard let self else { throw URLError(.badURL) }
        return self
    }
}
