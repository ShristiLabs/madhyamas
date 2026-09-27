import Foundation
import Network

/// Errors opening the connection to the proxy. TLS failures never fall
/// back to plaintext (parity with the Android `ProxySockets`/`TcpRelay`).
public enum ProxyConnectionError: Error, Equatable {
    case tlsFailure(String)
    case unreachable(String)
}

/// Opens and starts an `NWConnection` to the Madhyamas proxy.
///
/// - `useTls` (QR payload `tls=1`, issue #110): `NWParameters.tls` with
///   the default trust evaluator and hostname verification against the
///   `hostPort` endpoint — the demo proxy's Let's Encrypt certificate is
///   trusted out of the box; a TLS failure is reported, never downgraded.
/// - plain `tcp` otherwise.
///
/// `connectTimeout` mirrors the Android 10s connect timeout via
/// `TCPConnection` options.
public func openProxyConnection(host: String, port: Int, useTls: Bool,
                                connectTimeout: TimeInterval = 10) -> NWConnection {
    let parameters: NWParameters = useTls ? .tls : .tcp
    if let tcpOptions = parameters.defaultProtocolStack.transportProtocol as? NWProtocolTCP.Options {
        tcpOptions.connectionTimeout = Int(connectTimeout.rounded())
        tcpOptions.noDelay = true
    }
    let endpoint = NWEndpoint.hostPort(host: NWEndpoint.Host(host),
                                       port: NWEndpoint.Port(rawValue: UInt16(clamping: port)) ?? 8888)
    let connection = NWConnection(to: endpoint, using: parameters)
    return connection
}

extension NWConnection {
    /// Starts the connection on the given queue and suspends until it is
    /// ready (or fails). TLS failures map to `ProxyConnectionError.tlsFailure`.
    func waitUntilReady(queue: DispatchQueue, host: String) async throws {
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            var resumed = false
            stateUpdateHandler = { state in
                switch state {
                case .ready:
                    if !resumed { resumed = true; continuation.resume() }
                case .failed(let error):
                    if !resumed {
                        resumed = true
                        var isUnreachable = false
                        if case .posix(let code) = error {
                            isUnreachable = code == .ECONNREFUSED || code == .ETIMEDOUT
                        }
                        if isUnreachable {
                            continuation.resume(throwing: ProxyConnectionError.unreachable(error.localizedDescription))
                        } else {
                            continuation.resume(throwing: ProxyConnectionError.tlsFailure(error.localizedDescription))
                        }
                    }
                case .cancelled:
                    if !resumed { resumed = true; continuation.resume(throwing: ProxyConnectionError.unreachable("cancelled")) }
                default:
                    break
                }
            }
            start(queue: queue)
        }
    }

    /// Awaitable send.
    func send(_ data: Data) async throws {
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            send(content: data, completion: .contentProcessed { error in
                if let error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume()
                }
            })
        }
    }

    /// Awaitable receive of up to 64 KiB (empty Data on EOF/error-close).
    func receive() async throws -> Data {
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Data, Error>) in
            receive(minimumIncompleteLength: 1, maximumLength: 64 * 1024) { content, _, isComplete, error in
                if let content, !content.isEmpty {
                    continuation.resume(returning: content)
                } else if let error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume(returning: Data())
                }
            }
        }
    }
}
