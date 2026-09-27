import Foundation
import Network
import NetworkExtension
import MadhyamasCore

/// Pipes one `NEAppProxyTCPFlow` to the Madhyamas proxy:
///
///   app flow ⇄ [FlowPipe] ⇄ NWConnection (TLS/plain) ⇄ proxy CONNECT tunnel
///
/// Steps: open the flow → dial the proxy (TLS per `config.useTls`, never
/// downgraded) → author the CONNECT for the flow's original destination →
/// on 200 relay bidirectionally; on 407 report `.rejected` and close.
final class FlowPipe {

    enum Outcome {
        case established
        case rejected
        case failed
        case unreachable
        case tlsError
    }

    private let flow: NEAppProxyTCPFlow
    private let config: ProxyConfig
    private let proxyAuthorization: String
    private let queue: DispatchQueue
    private let onOutcome: (Outcome) -> Void
    private let onClosed: (FlowPipe) -> Void
    private let connection: NWConnection

    private var closed = false

    init(flow: NEAppProxyTCPFlow, config: ProxyConfig, proxyAuthorization: String,
         queue: DispatchQueue, onOutcome: @escaping (Outcome) -> Void,
         onClosed: @escaping (FlowPipe) -> Void) {
        self.flow = flow
        self.config = config
        self.proxyAuthorization = proxyAuthorization
        self.queue = queue
        self.onOutcome = onOutcome
        self.onClosed = onClosed
        self.connection = openProxyConnection(host: config.proxyHost, port: config.proxyPort,
                                              useTls: config.useTls)
    }

    func start() {
        flow.open(withLocalEndpoint: nil) { [weak self] error in
            guard let self else { return }
            if error == nil {
                self.connectToProxy()
            } else {
                self.close()
            }
        }
    }

    func stop() {
        queue.async { [weak self] in
            self?.close()
        }
    }

    // MARK: - proxy side

    private func connectToProxy() {
        connection.stateUpdateHandler = { [weak self] state in
            guard let self else { return }
            switch state {
            case .ready:
                self.handshake()
            case .failed(let error):
                var outcome = FlowPipe.Outcome.unreachable
                if case .posix(let code) = error, code != .ECONNREFUSED, code != .ETIMEDOUT {
                    outcome = .tlsError
                }
                self.onOutcome(outcome)
                self.close()
            case .cancelled:
                self.close()
            default:
                break
            }
        }
        connection.start(queue: queue)
    }

    private func handshake() {
        let endpoint: (host: String, port: Int)
        if let resolved = Self.destination(of: flow) {
            endpoint = resolved
        } else {
            onOutcome(.failed)
            close()
            return
        }

        Task { [weak self] in
            guard let self else { return }
            let result = await ProxyTunnelHandshake.perform(connection: self.connection,
                                                            dstHost: endpoint.host,
                                                            dstPort: endpoint.port,
                                                            proxyAuthorization: self.proxyAuthorization)
            await withCheckedContinuation { (continuation: CheckedContinuation<Void, Never>) in
                self.queue.async {
                    switch result {
                    case .established:
                        self.onOutcome(.established)
                        self.pumpFlowToProxy()
                        self.pumpProxyToFlow()
                    case .rejected:
                        self.onOutcome(.rejected)
                        self.close()
                    case .failed:
                        self.onOutcome(.failed)
                        self.close()
                    case .ioError:
                        self.onOutcome(.unreachable)
                        self.close()
                    }
                    continuation.resume()
                }
            }
        }
    }

    // MARK: - relay

    /// Resolves the flow's original destination (the CONNECT target).
    /// iOS 18+: `remoteFlowEndpoint` (Swift NWEndpoint); earlier: the
    /// legacy ObjC `NWHostEndpoint`.
    static func destination(of flow: NEAppProxyTCPFlow) -> (host: String, port: Int)? {
        if #available(iOS 18.0, *) {
            if case Network.NWEndpoint.hostPort(let host, let port) = flow.remoteFlowEndpoint {
                return ("\(host)", Int(port.rawValue))
            }
            return nil
        }
        if let hostEndpoint = flow.remoteEndpoint as? NWHostEndpoint {
            // `port` arrives as an NSNumber-ish value; coerce via its
            // description so any numeric type works.
            if let port = Int("\(hostEndpoint.port)") {
                return (hostEndpoint.hostname, port)
            }
        }
        return nil
    }

    private func pumpFlowToProxy() {
        flow.readData { [weak self] data, error in
            guard let self else { return }
            if let data {
                if !data.isEmpty {
                    self.connection.send(content: data, completion: .contentProcessed { _ in
                        self.pumpFlowToProxy()
                    })
                    return
                }
                // Empty read: flow signalled EOF; half-close the proxy side.
                self.connection.send(content: nil, contentContext: .finalMessage,
                                     isComplete: true, completion: .contentProcessed { _ in })
            } else {
                self.close()
            }
        }
    }

    private func pumpProxyToFlow() {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 64 * 1024) { [weak self] content, _, _, error in
            guard let self else { return }
            if let content, !content.isEmpty {
                self.flow.write(content) { error in
                    if error != nil {
                        self.close()
                    } else {
                        self.pumpProxyToFlow()
                    }
                }
            } else {
                self.close()
            }
            if error != nil {
                self.close()
            }
        }
    }

    // MARK: - teardown

    /// `closeReadWithError:`/`closeWriteWithError:` are public ObjC API on
    /// NEAppProxyFlow but not surfaced to Swift by the clang importer in
    /// current SDKs, hence the selector dance.
    private func closeFlow() {
        _ = flow.perform(Selector("closeReadWithError:"), with: nil)
        _ = flow.perform(Selector("closeWriteWithError:"), with: nil)
    }

    private func close() {
        guard !closed else { return }
        closed = true
        connection.cancel()
        closeFlow()
        onClosed(self)
    }
}
