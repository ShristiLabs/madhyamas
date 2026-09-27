import XCTest
@testable import MadhyamasCore

/// iOS-side coverage for the 3-consecutive-407 breaker ported from
/// MadhyamasVpnService.kt:171-186 (no dedicated Android test exists).
final class CircuitBreakerTests: XCTestCase {

    func testStartsNotTripped() {
        XCTAssertFalse(CircuitBreaker().tripped)
    }

    func testTripsAfterThreeConsecutiveRejections() {
        var breaker = CircuitBreaker()
        breaker.recordRejection()
        XCTAssertFalse(breaker.tripped)
        breaker.recordRejection()
        XCTAssertFalse(breaker.tripped)
        breaker.recordRejection()
        XCTAssertTrue(breaker.tripped)
    }

    func testSuccessResetsTheCounter() {
        var breaker = CircuitBreaker()
        breaker.recordRejection()
        breaker.recordRejection()
        breaker.recordSuccess()
        breaker.recordRejection()
        XCTAssertFalse(breaker.tripped, "a success clears the streak")
    }

    func testRejectionsAfterTrippingKeepItTripped() {
        var breaker = CircuitBreaker()
        for _ in 0..<5 { breaker.recordRejection() }
        XCTAssertTrue(breaker.tripped)
    }

    func testSuccessAfterTrippingUntrips() {
        var breaker = CircuitBreaker()
        for _ in 0..<3 { breaker.recordRejection() }
        XCTAssertTrue(breaker.tripped)
        breaker.recordSuccess()
        XCTAssertFalse(breaker.tripped)
    }

    func testCodableRoundTripPreservesState() throws {
        var breaker = CircuitBreaker()
        breaker.recordRejection()
        breaker.recordRejection()
        let data = try JSONEncoder().encode(breaker)
        let decoded = try JSONDecoder().decode(CircuitBreaker.self, from: data)
        XCTAssertEqual(breaker, decoded)
        XCTAssertFalse(decoded.tripped)
    }
}
