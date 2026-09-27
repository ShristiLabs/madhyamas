import Foundation
import Network

/// Coarse per-connection outcome surfaced to the tunnel provider (and from
/// there to the UI's status card) — issue #111.
public enum ConnectState: Equatable {
    /// CONNECT succeeded — traffic is being captured.
    case ok
    /// HTTP 407 — the proxy refused the device credential (revoked or wrong key).
    case rejected
    /// Proxy answered with another non-200 status.
    case failed
    /// Could not reach the proxy (connect/IO error).
    case unreachable
    /// TLS handshake or certificate verification failed on the proxy connection.
    case tlsError
}

/// Result of performing the HTTP CONNECT handshake with the proxy.
public enum TunnelResult: Equatable {
    case established
    case rejected
    case failed(String?)
    case ioError(String?)
}

/// Incremental CRLF-line reader with lone-CR tolerance — the exact byte
/// semantics of the Android `ProxyTunnelHandshake.readLine`, kept so the
/// wire format is unit-testable byte-for-byte.
public struct LineParser {
    private var bytes: [UInt8] = []
    private var pendingCR = false

    public init() {}

    /// Feeds one byte; returns a completed line (without CRLF) when the
    /// byte terminates one.
    public mutating func feed(_ byte: UInt8) -> String? {
        if pendingCR {
            pendingCR = false
            if byte == 0x0A {
                let line = String(bytes: bytes, encoding: .isoLatin1) ?? ""
                bytes.removeAll()
                return line
            }
            bytes.append(0x0D)
            bytes.append(byte)
        } else if byte == 0x0D {
            pendingCR = true
        } else {
            bytes.append(byte)
        }
        return nil
    }

    /// EOF: returns any accumulated (partial) line, mirroring the Android
    /// "non-empty remainder or null" behavior.
    public mutating func finish() -> String? {
        if pendingCR {
            bytes.append(0x0D)
            pendingCR = false
        }
        guard !bytes.isEmpty else { return nil }
        defer { bytes.removeAll() }
        return String(bytes: bytes, encoding: .isoLatin1)
    }
}

/// The proxy-side half of the tunnel setup — verbatim port of the Android
/// `ProxyTunnelHandshake` (android/.../vpn/ProxyTunnelHandshake.kt).
///
/// The companion AUTHORS this CONNECT request itself (it re-originates the
/// app's TCP connections to the proxy), so the `proxyAuthorization` value —
/// built from the Keychain-stored device credential — is the only
/// Proxy-Authorization the proxy ever sees on this connection.
///
/// A 407 response maps to `.rejected`: the caller closes the connection
/// and reports `ConnectState.rejected` — it never retries with the same
/// dead credential.
public enum ProxyTunnelHandshake {

    public static func buildRequest(dstHost: String, dstPort: Int,
                                    proxyAuthorization: String?) -> String {
        var request = "CONNECT \(dstHost):\(dstPort) HTTP/1.1\r\n"
        request += "Host: \(dstHost):\(dstPort)\r\n"
        if let proxyAuthorization {
            request += "Proxy-Authorization: \(proxyAuthorization)\r\n"
        }
        request += "Proxy-Connection: keep-alive\r\n"
        request += "\r\n"
        return request
    }

    /// `HTTP/1.1 200 Connection Established` -> 200; nil when unparseable.
    public static func statusCode(_ statusLine: String) -> Int? {
        let parts = statusLine.trimmingCharacters(in: .whitespaces).split(separator: " ")
        guard parts.count >= 2 else { return nil }
        return Int(parts[1])
    }

    /// Performs the handshake over an established (possibly TLS)
    /// `NWConnection` to the proxy: writes the CONNECT request, reads the
    /// status line, and on 200 consumes the remaining response headers up
    /// to the blank line.
    public static func perform(connection: NWConnection, dstHost: String, dstPort: Int,
                               proxyAuthorization: String?) async -> TunnelResult {
        let request = Data(buildRequest(dstHost: dstHost, dstPort: dstPort,
                                        proxyAuthorization: proxyAuthorization)
            .isoLatin1Bytes)
        do {
            try await connection.send(request)
        } catch {
            return .ioError(error.localizedDescription)
        }

        var parser = LineParser()
        var statusLine: String? = nil
        var headerParser = LineParser()
        var established = false

        while true {
            let chunk: Data
            do {
                chunk = try await connection.receive()
            } catch {
                return .ioError(error.localizedDescription)
            }
            guard !chunk.isEmpty else {
                return .ioError(nil)
            }
            for byte in chunk {
                if statusLine == nil {
                    if let line = parser.feed(byte) {
                        statusLine = line
                        guard let code = statusCode(line) else {
                            return .failed(line)
                        }
                        switch code {
                        case 200: continue // consume headers below
                        case 407: return .rejected
                        default: return .failed(line)
                        }
                    }
                } else {
                    if let line = headerParser.feed(byte) {
                        if line.isEmpty {
                            established = true
                            break
                        }
                    }
                }
            }
            if established { break }
        }
        return .established
    }
}

extension String {
    /// ISO-8859-1 bytes, matching the Android `toByteArray(ISO_8859_1)`.
    var isoLatin1Bytes: [UInt8] {
        unicodeScalars.map { scalar in
            UInt8(truncatingIfNeeded: scalar.value & 0xFF)
        }
    }
}
