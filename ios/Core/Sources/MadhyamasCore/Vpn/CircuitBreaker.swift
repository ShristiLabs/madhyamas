import Foundation

/// 3-consecutive-407 fail-fast state machine — port of the Android VPN
/// service's `authRejected` logic (MadhyamasVpnService.kt:171-186).
///
/// After three consecutive rejected CONNECTs the provider stops dialing
/// the proxy for new flows (they fail immediately) until a success resets
/// the counter. Persisted via `ConfigStore` so it survives provider
/// restarts.
public struct CircuitBreaker: Codable, Equatable {
    public let threshold: Int
    public var consecutiveRejections: Int

    public init(threshold: Int = 3, consecutiveRejections: Int = 0) {
        self.threshold = threshold
        self.consecutiveRejections = consecutiveRejections
    }

    public var tripped: Bool { consecutiveRejections >= threshold }

    public mutating func recordRejection() {
        consecutiveRejections += 1
    }

    public mutating func recordSuccess() {
        consecutiveRejections = 0
    }
}
