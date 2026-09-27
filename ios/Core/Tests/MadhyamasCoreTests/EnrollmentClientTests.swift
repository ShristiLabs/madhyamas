import Foundation
import XCTest
@testable import MadhyamasCore

/// URLProtocol mock ≙ android MinimalHttpMock — serves canned statuses and
/// bodies, and records the last request for assertions.
final class MockURLProtocol: URLProtocol {
    nonisolated(unsafe) static var handler: ((URLRequest) throws -> (HTTPURLResponse, Data))?

    override class func canInit(with request: URLRequest) -> Bool { true }
    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        guard let handler = MockURLProtocol.handler else {
            client?.urlProtocol(self, didFailWithError: URLError(.badServerResponse))
            return
        }
        do {
            let (response, data) = try handler(request)
            client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: data)
            client?.urlProtocolDidFinishLoading(self)
        } catch {
            client?.urlProtocol(self, didFailWithError: error)
        }
    }

    override func stopLoading() {}
}

/// Mirrors android EnrollmentClientTest (12 cases).
final class EnrollmentClientTests: XCTestCase {

    private func mockedClient() -> URLEnrollmentClient {
        let config = URLSessionConfiguration.ephemeral
        config.protocolClasses = [MockURLProtocol.self]
        return URLEnrollmentClient(session: URLSession(configuration: config))
    }

    private func httpResponse(_ url: URL, status: Int) -> HTTPURLResponse {
        HTTPURLResponse(url: url, statusCode: status, httpVersion: "HTTP/1.1", headerFields: nil)!
    }

    override func tearDown() {
        MockURLProtocol.handler = nil
        super.tearDown()
    }

    // ---------- URL building ----------

    func testEnrollUrlAppendsPath() {
        XCTAssertEqual("http://host:3001/api/devices/enroll",
                       URLEnrollmentClient.enrollUrl("http://host:3001/api"))
    }

    func testEnrollUrlTrimsTrailingSlashAndSpaces() {
        XCTAssertEqual("http://host/api/devices/enroll",
                       URLEnrollmentClient.enrollUrl(" http://host/api/ "))
    }

    // ---------- status mapping ----------

    func testSuccessParsesDeviceAndKey() async {
        let body = #"{"device":{"id":"9a7b5012","name":"demo-laptop"},"key":"mdy_dev_89a38cdcaab2429c"}"#
        MockURLProtocol.handler = { request in
            XCTAssertEqual("POST", request.httpMethod)
            XCTAssertEqual("application/json", request.value(forHTTPHeaderField: "Content-Type"))
            let url = try XCTUnwrap(request.url)
            XCTAssertEqual("http://x/api/devices/enroll", url.absoluteString)
            return (self.httpResponse(url, status: 200), Data(body.utf8))
        }
        let result = await mockedClient().enroll(apiBaseUrl: "http://x/api", token: "mdy_enroll_t")
        XCTAssertEqual(.success(PairedCredential(key: "mdy_dev_89a38cdcaab2429c",
                                                 deviceId: "9a7b5012",
                                                 deviceName: "demo-laptop")), result)
    }

    func testRequestBodyCarriesOnlyTheToken() async throws {
        var capturedBody: Data?
        MockURLProtocol.handler = { request in
            capturedBody = request.httpBody ?? request.httpBodyStream.map { stream -> Data in
                stream.open()
                defer { stream.close() }
                var data = Data()
                let bufSize = 4096
                let buf = UnsafeMutablePointer<UInt8>.allocate(capacity: bufSize)
                defer { buf.deallocate() }
                while stream.hasBytesAvailable {
                    let n = stream.read(buf, maxLength: bufSize)
                    if n <= 0 { break }
                    data.append(buf, count: n)
                }
                return data
            }
            return (self.httpResponse(try XCTUnwrap(request.url), status: 401), Data())
        }
        _ = await mockedClient().enroll(apiBaseUrl: "http://x/api", token: "mdy_enroll_t")
        let json = try XCTUnwrap(capturedBody).isEmpty ? nil
            : try? JSONSerialization.jsonObject(with: capturedBody!) as? [String: String]
        XCTAssertEqual(["token": "mdy_enroll_t"], json)
    }

    func test400MapsToMalformedToken() async {
        MockURLProtocol.handler = { req in (self.httpResponse(req.url!, status: 400), Data()) }
        let result = await mockedClient().enroll(apiBaseUrl: "http://x/api", token: "t")
        XCTAssertEqual(.failure(.malformedToken), result)
    }

    func test401MapsToInvalidToken() async {
        MockURLProtocol.handler = { req in (self.httpResponse(req.url!, status: 401), Data()) }
        let result = await mockedClient().enroll(apiBaseUrl: "http://x/api", token: "t")
        XCTAssertEqual(.failure(.invalidToken), result)
    }

    func test500MapsToServerError() async {
        MockURLProtocol.handler = { req in (self.httpResponse(req.url!, status: 500), Data()) }
        let result = await mockedClient().enroll(apiBaseUrl: "http://x/api", token: "t")
        XCTAssertEqual(.failure(.serverError), result)
    }

    func test302MapsToServerError() async {
        MockURLProtocol.handler = { req in (self.httpResponse(req.url!, status: 302), Data()) }
        let result = await mockedClient().enroll(apiBaseUrl: "http://x/api", token: "t")
        XCTAssertEqual(.failure(.serverError), result)
    }

    func testTransportErrorMapsToNetwork() async {
        MockURLProtocol.handler = { _ in throw URLError(.notConnectedToInternet) }
        let result = await mockedClient().enroll(apiBaseUrl: "http://x/api", token: "t")
        XCTAssertEqual(.failure(.network), result)
    }

    // ---------- response parsing ----------

    func testParseResponseRejectsNonDeviceKey() {
        XCTAssertEqual(.failure(.badResponse),
                       URLEnrollmentClient.parseResponse(Data(#"{"key":"wrong"}"#.utf8)))
    }

    func testParseResponseRejectsGarbageJson() {
        XCTAssertEqual(.failure(.badResponse),
                       URLEnrollmentClient.parseResponse(Data("not json".utf8)))
    }

    func testParseResponseToleratesMissingDeviceObject() {
        let result = URLEnrollmentClient.parseResponse(Data(#"{"key":"mdy_dev_k"}"#.utf8))
        XCTAssertEqual(.success(PairedCredential(key: "mdy_dev_k", deviceId: nil, deviceName: nil)), result)
    }

    func testParseResponseBlanksDeviceFieldsToNil() {
        let result = URLEnrollmentClient.parseResponse(
            Data(#"{"device":{"id":"","name":"  "},"key":"mdy_dev_k"}"#.utf8))
        XCTAssertEqual(.success(PairedCredential(key: "mdy_dev_k", deviceId: nil, deviceName: nil)), result)
    }
}
