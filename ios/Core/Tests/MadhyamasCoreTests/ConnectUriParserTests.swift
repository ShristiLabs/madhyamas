import XCTest
@testable import MadhyamasCore

/// Full validation matrix for the madhyamas://connect deep link (issue
/// #111) — mirrors android ConnectUriParserTest (28 cases).
final class ConnectUriParserTests: XCTestCase {

    private func ok(_ link: String, file: StaticString = #filePath, line: UInt = #line) throws -> ConnectPayload {
        let result = ConnectUriParser.parse(link)
        guard case .ok(let payload) = result else {
            XCTFail("expected Ok for \(link), got \(result)", file: file, line: line)
            throw XCTSkip("")
        }
        return payload
    }

    private func err(_ link: String?, file: StaticString = #filePath, line: UInt = #line) -> String {
        let result = ConnectUriParser.parse(link)
        guard case .error(let reason) = result else {
            XCTFail("expected Error for \(String(describing: link)), got \(result)", file: file, line: line)
            return ""
        }
        return reason
    }

    // ---------- happy paths ----------

    func testParsesFullTokenLink() throws {
        let payload = try ok(
            "madhyamas://connect?host=proxy.example.com&port=8888&tls=1"
                + "&token=mdy_enroll_0123456789abcdef&name=Hari%27s%20Pixel"
                + "&ca=http://proxy.example.com:3001/api/cert/ca"
                + "&api=http://proxy.example.com:3001/api"
        )
        XCTAssertEqual("proxy.example.com", payload.host)
        XCTAssertEqual(8888, payload.port)
        XCTAssertTrue(payload.tls)
        XCTAssertEqual("Hari's Pixel", payload.name)
        XCTAssertEqual("mdy_enroll_0123456789abcdef", payload.token)
        XCTAssertNil(payload.key)
        XCTAssertTrue(payload.isEnrollment)
        XCTAssertEqual("http://proxy.example.com:3001/api/cert/ca", payload.caUrl)
        XCTAssertEqual("http://proxy.example.com:3001/api", payload.apiUrl)
    }

    func testParsesKeyLinkManualMode() throws {
        let payload = try ok(
            "madhyamas://connect?host=10.0.0.5&port=18888&tls=0&key=mdy_dev_aaaaaaaaaaaaaaaa"
                + "&api=https://10.0.0.5:3001/api"
        )
        XCTAssertFalse(payload.tls)
        XCTAssertEqual("mdy_dev_aaaaaaaaaaaaaaaa", payload.key)
        XCTAssertNil(payload.token)
        XCTAssertFalse(payload.isEnrollment)
        XCTAssertNil(payload.name)
        XCTAssertNil(payload.caUrl)
    }

    func testTlsDefaultsToFalseWhenAbsent() throws {
        let payload = try ok("madhyamas://connect?host=h&port=8888&key=mdy_dev_k")
        XCTAssertFalse(payload.tls)
    }

    func testEmptyNameBecomesNil() throws {
        let payload = try ok("madhyamas://connect?host=h&port=1&key=mdy_dev_k&name=%20%20")
        XCTAssertNil(payload.name)
    }

    func testSchemeAndLinkHostAreCaseInsensitive() throws {
        let payload = try ok("MADHYAMAS://Connect?host=h&port=1&key=mdy_dev_k")
        XCTAssertEqual("h", payload.host)
    }

    func testPortBoundariesAccepted() throws {
        XCTAssertEqual(1, try ok("madhyamas://connect?host=h&port=1&key=mdy_dev_k").port)
        XCTAssertEqual(65535, try ok("madhyamas://connect?host=h&port=65535&key=mdy_dev_k").port)
    }

    // ---------- error arms ----------

    func testRejectsEmptyLink() {
        XCTAssertTrue(err("").contains("Empty"))
    }

    func testRejectsNilLink() {
        XCTAssertTrue(err(nil).contains("Empty"))
    }

    func testRejectsMalformedUri() {
        XCTAssertFalse(err("madhyamas://connect?host=%zz&port=1&key=mdy_dev_k").isEmpty)
    }

    func testRejectsWrongScheme() {
        XCTAssertTrue(err("https://connect?host=h&port=1&key=mdy_dev_k").contains("Not a Madhyamas"))
    }

    func testRejectsWrongLinkHost() {
        XCTAssertTrue(err("madhyamas://pair?host=h&port=1&key=mdy_dev_k").contains("Not a Madhyamas"))
    }

    func testRejectsMissingHostParam() {
        XCTAssertTrue(err("madhyamas://connect?port=1&key=mdy_dev_k").contains("host"))
    }

    func testRejectsBlankHostParam() {
        XCTAssertTrue(err("madhyamas://connect?host=%20&port=1&key=mdy_dev_k").contains("host"))
    }

    func testRejectsMissingPort() {
        XCTAssertTrue(err("madhyamas://connect?host=h&key=mdy_dev_k").contains("port"))
    }

    func testRejectsNonNumericPort() {
        XCTAssertTrue(err("madhyamas://connect?host=h&port=abc&key=mdy_dev_k").contains("port"))
    }

    func testRejectsPortZero() {
        XCTAssertTrue(err("madhyamas://connect?host=h&port=0&key=mdy_dev_k").contains("range"))
    }

    func testRejectsPortAboveRange() {
        XCTAssertTrue(err("madhyamas://connect?host=h&port=65536&key=mdy_dev_k").contains("range"))
    }

    func testRejectsBadTlsValue() {
        XCTAssertTrue(err("madhyamas://connect?host=h&port=1&tls=2&key=mdy_dev_k").contains("tls"))
    }

    func testRejectsBothTokenAndKey() {
        XCTAssertTrue(
            err("madhyamas://connect?host=h&port=1&token=mdy_enroll_a&key=mdy_dev_b&api=http://x/api")
                .contains("both")
        )
    }

    func testRejectsNoCredential() {
        XCTAssertTrue(err("madhyamas://connect?host=h&port=1").contains("no credential"))
    }

    func testRejectsWrongTokenPrefix() {
        XCTAssertTrue(
            err("madhyamas://connect?host=h&port=1&token=mdy_dev_a&api=http://x/api")
                .contains("token format")
        )
    }

    func testRejectsWrongKeyPrefix() {
        XCTAssertTrue(err("madhyamas://connect?host=h&port=1&key=mdy_enroll_a").contains("key format"))
    }

    func testRejectsTokenWithoutApi() {
        XCTAssertTrue(err("madhyamas://connect?host=h&port=1&token=mdy_enroll_a").contains("API URL"))
    }

    func testRejectsNonHttpCa() {
        XCTAssertTrue(
            err("madhyamas://connect?host=h&port=1&key=mdy_dev_k&ca=ftp://example.com/ca")
                .contains("ca parameter")
        )
    }

    func testRejectsNonHttpApi() {
        XCTAssertTrue(
            err("madhyamas://connect?host=h&port=1&key=mdy_dev_k&api=ftp://example.com/api")
                .contains("api parameter")
        )
    }

    // ---------- query decoding ----------

    func testDecodesUrlEncodedValues() {
        let params = ConnectUriParser.parseQuery("a=one%20two&b=x%26y&c")
        XCTAssertEqual("one two", params["a"])
        XCTAssertEqual("x&y", params["b"])
        XCTAssertNil(params["c"])
    }

    func testParseQueryHandlesEmptyInput() {
        XCTAssertTrue(ConnectUriParser.parseQuery(nil).isEmpty)
        XCTAssertTrue(ConnectUriParser.parseQuery("").isEmpty)
    }

    func testParseNeverFailsOnGarbage() {
        for raw in [
            "madhyamas://",
            "madhyamas://connect",
            "madhyamas://connect?",
            "::::",
            "madhyamas://connect?=&=&",
            "madhyamas://connect?token=",
            "madhyamas://connect?host=h&port=99999999999999999999&key=mdy_dev_k",
        ] {
            switch ConnectUriParser.parse(raw) {
            case .ok, .error:
                break // must produce a result — never trap
            }
        }
    }
}
