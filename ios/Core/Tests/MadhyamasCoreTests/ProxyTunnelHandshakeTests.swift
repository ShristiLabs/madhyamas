import XCTest
@testable import MadhyamasCore

/// Mirrors android ProxyTunnelHandshakeTest (request bytes, status
/// parsing, line-reader edge cases).
final class ProxyTunnelHandshakeTests: XCTestCase {

    // ---------- buildRequest ----------

    func testBuildRequestWithoutAuthorization() {
        let request = ProxyTunnelHandshake.buildRequest(dstHost: "example.com", dstPort: 443,
                                                        proxyAuthorization: nil)
        let expected = "CONNECT example.com:443 HTTP/1.1\r\n"
            + "Host: example.com:443\r\n"
            + "Proxy-Connection: keep-alive\r\n"
            + "\r\n"
        XCTAssertEqual(expected, request)
    }

    func testBuildRequestWithAuthorization() {
        let request = ProxyTunnelHandshake.buildRequest(
            dstHost: "10.0.0.5", dstPort: 8080,
            proxyAuthorization: ProxyAuth.basicHeaderValue(deviceKey: "mdy_dev_k")
        )
        XCTAssertTrue(request.contains("CONNECT 10.0.0.5:8080 HTTP/1.1\r\n"))
        XCTAssertTrue(request.contains("Host: 10.0.0.5:8080\r\n"))
        XCTAssertTrue(request.contains("Proxy-Authorization: \(ProxyAuth.basicHeaderValue(deviceKey: "mdy_dev_k"))\r\n"))
        XCTAssertTrue(request.hasSuffix("Proxy-Connection: keep-alive\r\n\r\n"))
    }

    // ---------- statusCode ----------

    func testStatusCodeParsesStandardLine() {
        XCTAssertEqual(200, ProxyTunnelHandshake.statusCode("HTTP/1.1 200 Connection Established"))
        XCTAssertEqual(407, ProxyTunnelHandshake.statusCode("HTTP/1.1 407 Proxy Authentication Required"))
    }

    func testStatusCodeTrimsLeadingSpace() {
        XCTAssertEqual(200, ProxyTunnelHandshake.statusCode("  HTTP/1.0 200 OK"))
    }

    func testStatusCodeReturnsNilForTooFewParts() {
        XCTAssertNil(ProxyTunnelHandshake.statusCode("garbage"))
        XCTAssertNil(ProxyTunnelHandshake.statusCode(""))
    }

    func testStatusCodeReturnsNilForNonNumeric() {
        XCTAssertNil(ProxyTunnelHandshake.statusCode("HTTP/1.1 abc OK"))
    }

    // ---------- LineParser (readLine semantics incl. lone-CR tolerance) ----------

    private func line(from bytes: [UInt8], file: StaticString = #filePath, line: UInt = #line) -> String? {
        var parser = LineParser()
        for b in bytes.dropLast() {
            if parser.feed(b) != nil {
                XCTFail("premature line", file: file, line: line)
            }
        }
        return parser.feed(bytes.last!)
    }

    func testLineParserReadsCRLFTerminatedLine() {
        XCTAssertEqual("HTTP/1.1 200 Connection Established",
                       line(from: Array("HTTP/1.1 200 Connection Established\r\n".utf8)))
    }

    func testLineParserToleratesLoneCR() {
        // CR followed by a non-LF byte: CR and that byte join the line,
        // which then continues (parity with android readLine).
        XCTAssertEqual("a\rbc", line(from: Array("a\rbc\r\n".utf8)))
    }

    func testLineParserEmptyLine() {
        XCTAssertEqual("", line(from: Array("\r\n".utf8)))
    }

    func testLineParserFinishReturnsPartialLineAtEof() {
        var parser = LineParser()
        for b in Array("partial".utf8) {
            XCTAssertNil(parser.feed(b))
        }
        XCTAssertEqual("partial", parser.finish())
    }

    func testLineParserFinishNilWhenEmptyAtEof() {
        var parser = LineParser()
        XCTAssertNil(parser.finish())
    }

    func testLineParserFinishAppendsTrailingCR() {
        var parser = LineParser()
        for b in Array("ab".utf8) {
            XCTAssertNil(parser.feed(b))
        }
        _ = parser.feed(0x0D) // pending CR, then EOF
        XCTAssertEqual("ab\r", parser.finish())
    }

    // ---------- ISO-8859-1 byte fidelity ----------

    func testIsoLatin1BytesForHighBytes() {
        // Bytes > 0x7F map 1:1 (ISO-8859-1), not UTF-8 multi-byte.
        // Built from a scalar so the source file's encoding can't skew it.
        let latin1 = String(UnicodeScalar(0xE9)!) // é
        XCTAssertEqual([0xE9], latin1.isoLatin1Bytes)
    }
}
