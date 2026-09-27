import Foundation
import Network
import XCTest
@testable import MadhyamasCore

/// End-to-end pairing over real loopback sockets — mirrors android
/// PairingEndToEndTest: mock enroll API → stored credential → CONNECT over
/// a real socket carries the expected Proxy-Authorization header.
final class PairingEndToEndTests: XCTestCase {

    func testEnrollThenConnectCarriesAuthorizationHeader() async throws {
        let (listener, port) = try await startListener()
        defer { listener.cancel() }

        // 1. Enrollment against the mock API over real HTTP.
        let enrollClient = URLEnrollmentClient() // default session, real request
        let enrolled = await enrollClient.enroll(apiBaseUrl: "http://127.0.0.1:\(port)/api",
                                                 token: "mdy_enroll_0123456789abcdef")
        guard case .success(let credential) = enrolled else {
            return XCTFail("enrollment failed: \(enrolled)")
        }
        XCTAssertEqual("mdy_dev_89a38cdcaab2429c9f27f888552d723b", credential.key)

        // 2. CONNECT through the same listener with the credential.
        let connection = openProxyConnection(host: "127.0.0.1", port: Int(port), useTls: false)
        defer { connection.cancel() }
        try await connection.waitUntilReady(queue: DispatchQueue.global(), host: "127.0.0.1")

        let proxyAuth = ProxyAuth.basicHeaderValue(deviceKey: credential.key)
        let result = await ProxyTunnelHandshake.perform(connection: connection,
                                                        dstHost: "example.com", dstPort: 443,
                                                        proxyAuthorization: proxyAuth)
        XCTAssertEqual(TunnelResult.established, result)

        // 3. Data relayed through the established tunnel (echo server).
        try await connection.send(Data("ping".utf8))
        let echoed = try await connection.receive()
        XCTAssertEqual("ping", String(data: echoed, encoding: .utf8))

        // 4. The listener recorded the exact header the companion authored.
        let recorded = try await RecordedRequests.shared.connectRequest()
        XCTAssertTrue(recorded.contains("CONNECT example.com:443 HTTP/1.1"),
                      "unexpected CONNECT request: \(recorded)")
        XCTAssertTrue(recorded.contains("Proxy-Authorization: \(proxyAuth)\r\n"),
                      "missing proxy authorization in: \(recorded)")
    }

    // ---------- mock server ----------

    actor RequestBox {
        var connectRequestText: String?
        func set(_ value: String) { connectRequestText = value }
        func get() -> String? { connectRequestText }
    }

    final class RecordedRequests {
        static let shared = RecordedRequests()
        let box = RequestBox()
        func connectRequest() async throws -> String {
            for _ in 0..<50 {
                if await box.get() != nil { break }
                try await Task.sleep(nanoseconds: 100_000_000)
            }
            guard let recorded = await box.get() else {
                throw NSError(domain: "e2e", code: 1,
                              userInfo: [NSLocalizedDescriptionKey: "CONNECT never arrived at the mock proxy"])
            }
            return recorded
        }
    }

    private func startListener() async throws -> (NWListener, UInt16) {
        let box = RecordedRequests.shared.box
        let params = NWParameters.tcp
        params.allowLocalEndpointReuse = true
        let listener = try NWListener(using: params, on: .any)

        listener.newConnectionHandler = { [weak self] connection in
            connection.start(queue: .global())
            self?.serve(connection: connection, recorded: box)
        }
        listener.start(queue: .global())

        let port: UInt16 = try await withCheckedThrowingContinuation { continuation in
            listener.stateUpdateHandler = { state in
                switch state {
                case .ready:
                    continuation.resume(returning: listener.port?.rawValue ?? 0)
                case .failed(let error):
                    continuation.resume(throwing: error)
                default:
                    break
                }
            }
        }
        return (listener, port)
    }

    /// Serves two kinds of connection: the enroll POST (HTTP JSON) and the
    /// proxy CONNECT (asserted + echoed) — told apart by the first bytes.
    private func serve(connection: NWConnection, recorded: RequestBox) {
        var buffer = Data()
        var mode: Mode = .undetermined

        enum Mode { case undetermined, enroll, connect }

        func receiveNext() {
            connection.receive(minimumIncompleteLength: 1, maximumLength: 64 * 1024) { content, _, _, error in
                if let content { buffer.append(content) }
                if error != nil || content == nil { connection.cancel(); return }

                let text = String(data: buffer, encoding: .utf8) ?? ""
                if mode == .undetermined {
                    mode = text.hasPrefix("CONNECT ") ? .connect : .enroll
                }
                if mode == .enroll {
                    // Enroll HTTP request: wait for the full body per Content-Length.
                    if let headerEnd = text.range(of: "\r\n\r\n") {
                        let headers = String(text[..<headerEnd.lowerBound])
                        let headerByteCount = headers.utf8.count + 4
                        let lowerHeaders = headers.lowercased()
                        var length = 0
                        if let r = lowerHeaders.range(of: "content-length: ") {
                            length = Int(lowerHeaders[r.upperBound...].prefix(while: { $0.isNumber })) ?? 0
                        }
                        if buffer.count >= headerByteCount + length {
                            let body = #"{"device":{"id":"dev1","name":"e2e"},"key":"mdy_dev_89a38cdcaab2429c9f27f888552d723b"}"#
                            let response = "HTTP/1.1 200 OK\r\n"
                                + "Content-Length: \(body.utf8.count)\r\n"
                                + "Connection: close\r\n\r\n\(body)"
                            connection.send(content: Data(response.utf8), completion: .contentProcessed { _ in
                                connection.cancel()
                            })
                            buffer.removeAll()
                        }
                    }
                } else {
                    // Proxy connection: CONNECT until blank line, then 200 + echo.
                    if let headerEnd = text.range(of: "\r\n\r\n") {
                        Task { await recorded.set(String(text[..<headerEnd.lowerBound]) + "\r\n") }
                        let established = "HTTP/1.1 200 Connection Established\r\n\r\n"
                        connection.send(content: Data(established.utf8), completion: .contentProcessed { _ in })
                        let remainder = String(text[headerEnd.upperBound...])
                        if !remainder.isEmpty {
                            connection.send(content: Data(remainder.utf8), completion: .contentProcessed { _ in })
                        }
                        buffer = Data(text[headerEnd.upperBound...].utf8)
                        self.echoLoop(connection: connection)
                        return
                    }
                }
                receiveNext()
            }
        }
        receiveNext()
    }

    private func echoLoop(connection: NWConnection) {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 64 * 1024) { content, _, _, error in
            if let content, !content.isEmpty {
                connection.send(content: content, completion: .contentProcessed { _ in
                    self.echoLoop(connection: connection)
                })
            } else {
                connection.cancel()
            }
            if error != nil { connection.cancel() }
        }
    }
}
