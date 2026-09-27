import XCTest
@testable import MadhyamasCore

/// Mirrors android ProxyAuthTest.
final class ProxyAuthTests: XCTestCase {

    func testHeaderName() {
        XCTAssertEqual("Proxy-Authorization", ProxyAuth.headerName)
    }

    func testBasicHeaderValueIsCanonicalForm() {
        // base64("mdy_dev_k:") — key as Basic username, empty password.
        let expected = Data("mdy_dev_k:".utf8).base64EncodedString()
        XCTAssertEqual("Basic \(expected)", ProxyAuth.basicHeaderValue(deviceKey: "mdy_dev_k"))
    }

    func testBasicHeaderValueEncodesLongKey() {
        let key = "mdy_dev_89a38cdcaab2429c9f27f888552d723b"
        let expected = Data("\(key):".utf8).base64EncodedString()
        XCTAssertEqual("Basic \(expected)", ProxyAuth.basicHeaderValue(deviceKey: key))
    }
}
