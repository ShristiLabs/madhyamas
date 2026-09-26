//! Interception, breakpoints, and traffic modification

mod block_list;
mod breakpoint;
mod handler;
mod mock;
mod regex_cache;
mod rewrite;
mod throttle;
mod types;

pub use block_list::{matches_pattern, BlockListEntry, BlockListManager, BlockListStats};
pub use breakpoint::{
    BreakpointDecision, BreakpointManager, BreakpointRule, BreakpointState, PausedTraffic,
};
pub use handler::{InterceptAction, InterceptHandler};
pub use mock::{
    ConditionalResponse, MockCollection, MockExpiration, MockHitRecord, MockHitStats, MockManager,
    MockPreviewResult, MockResponse, MockRule, MockRuleVersion, MockTemplates, MockTestResult,
    ProbabilisticResponse, RequestCondition, ResponseConfig,
};
pub use rewrite::{RewriteAction, RewriteDirection, RewriteManager, RewriteRule, RewriteTemplates};
pub use throttle::{ThrottleManager, ThrottleProfile};
pub use types::*;

/// Whether a device-scoped rule applies to a request attributed to
/// `request_device` (issue #109).
///
/// - A global rule (`None` scope) applies to **all** traffic, including
///   device-attributed and unattributed requests.
/// - A rule scoped to device X applies **only** to requests attributed to
///   device X; every other device (and unattributed traffic) skips it.
///
/// This is the single match-time predicate shared by every intercept
/// manager; it is a cheap `Option` comparison with no lookups.
pub fn device_scope_applies(rule_device: Option<&str>, request_device: Option<&str>) -> bool {
    match rule_device {
        None => true,
        Some(rule) => Some(rule) == request_device,
    }
}

#[cfg(test)]
mod tests {
    use super::device_scope_applies;

    #[test]
    fn global_rule_applies_to_every_including_unattributed() {
        // None-scoped (user-global) rules keep their pre-#109 behavior:
        // they match every request, attributed or not.
        assert!(device_scope_applies(None, None));
        assert!(device_scope_applies(None, Some("device-x")));
        assert!(device_scope_applies(None, Some("device-y")));
    }

    #[test]
    fn scoped_rule_applies_only_to_its_device() {
        assert!(device_scope_applies(Some("device-x"), Some("device-x")));
        assert!(!device_scope_applies(Some("device-x"), Some("device-y")));
        // Unattributed traffic never sees device-scoped rules.
        assert!(!device_scope_applies(Some("device-x"), None));
    }
}
