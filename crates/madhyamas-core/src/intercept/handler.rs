//! Unified intercept handler trait.
//!
//! All intercept managers ([`MockManager`], [`BreakpointManager`],
//! [`RewriteManager`], [`ThrottleManager`]) participate in the proxy
//! request/response pipeline. Historically each was invoked through a
//! separate `if let Some(manager) = self.X_manager { ... }` block in
//! [`crate::proxy::pipeline::Pipeline`], with the short-circuit response
//! handling (store entry, broadcast, write to client, record metrics)
//! duplicated for mocks and breakpoints.
//!
//! The [`InterceptHandler`] trait gives every manager a uniform
//! `on_request` / `on_response` surface. Managers that don't participate
//! in a given direction rely on the default no-op implementations.
//!
//! # Pipeline integration
//!
//! The pipeline invokes each attached handler in [`InterceptHandler::priority`]
//! order (ascending — lower numbers run first). A handler may:
//!
//! - Mutate the request/response in place (e.g. [`RewriteManager`]).
//! - Apply a side effect (e.g. [`ThrottleManager`] sleeps for latency).
//! - Short-circuit the pipeline by returning [`InterceptAction::Respond`]
//!   (e.g. [`MockManager`] and [`BreakpointManager`]).
//! - Abort the request by returning [`InterceptAction::Abort`]
//!   (e.g. [`BreakpointManager`]).
//!
//! The pipeline centralizes the "store entry → broadcast → write to client
//! → record metrics" sequence for any handler that returns
//! [`InterceptAction::Respond`], eliminating the duplicated response
//! shipping code that previously existed in both the mock and breakpoint
//! branches.

use crate::traffic::{RequestData, ResponseData};

/// Action returned by an intercept handler after processing a
/// request or response.
#[derive(Debug)]
pub enum InterceptAction {
    /// Continue processing. The request/response may have been modified
    /// in place by the handler.
    Continue,
    /// Short-circuit the pipeline and return `response` to the client.
    /// The pipeline stores the traffic entry, broadcasts it, writes the
    /// response bytes to the client stream, and records metrics.
    Respond(ResponseData),
    /// Abort the request entirely. No response is sent to the client.
    Abort,
}

/// Unified intercept handler trait.
///
/// Implemented by every manager that participates in the proxy pipeline.
/// Methods have default no-op implementations so each manager only
/// overrides the directions it cares about.
#[async_trait::async_trait]
pub trait InterceptHandler: Send + Sync {
    /// Human-readable name for logging and diagnostics.
    fn name(&self) -> &'static str;

    /// Processing priority. Handlers are invoked in ascending order
    /// (lower number = earlier). Default: `100`.
    fn priority(&self) -> u32 {
        100
    }

    /// Process an outgoing request before it is forwarded upstream.
    /// May mutate the request in place. Default: no-op (`Continue`).
    async fn on_request(&self, _request: &mut RequestData) -> InterceptAction {
        InterceptAction::Continue
    }

    /// Process an incoming response after it is received from upstream.
    /// May mutate the response in place. Default: no-op (`Continue`).
    async fn on_response(
        &self,
        _request: &RequestData,
        _response: &mut ResponseData,
    ) -> InterceptAction {
        InterceptAction::Continue
    }
}

// ============================================================================
// Trait implementations for each manager
// ============================================================================

use super::{
    BreakpointDecision, BreakpointManager, InterceptDirection, MockManager, RewriteManager,
    ThrottleManager,
};

#[async_trait::async_trait]
impl InterceptHandler for RewriteManager {
    fn name(&self) -> &'static str {
        "rewrite"
    }

    fn priority(&self) -> u32 {
        // Rewrites run first so subsequent handlers (mock/breakpoint) see
        // the rewritten request.
        10
    }

    async fn on_request(&self, request: &mut RequestData) -> InterceptAction {
        self.rewrite_request(request);
        InterceptAction::Continue
    }

    async fn on_response(
        &self,
        request: &RequestData,
        response: &mut ResponseData,
    ) -> InterceptAction {
        self.rewrite_response(request, response);
        InterceptAction::Continue
    }
}

#[async_trait::async_trait]
impl InterceptHandler for MockManager {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn priority(&self) -> u32 {
        // Mocks run after rewrites but before breakpoints so a matching
        // mock short-circuits before the user is prompted.
        20
    }

    async fn on_request(&self, request: &mut RequestData) -> InterceptAction {
        if let Some(mock) = self.find_matching_mock(request) {
            tracing::debug!("Mock matched: {} for {}", mock.name, request.url);
            // Build the mock response (honors configured delay).
            let response = build_mock_response(&mock.response()).await;
            return InterceptAction::Respond(response);
        }
        InterceptAction::Continue
    }
}

#[async_trait::async_trait]
impl InterceptHandler for BreakpointManager {
    fn name(&self) -> &'static str {
        "breakpoint"
    }

    fn priority(&self) -> u32 {
        // Breakpoints run after mocks so the user is only prompted for
        // traffic that isn't already mocked.
        30
    }

    async fn on_request(&self, request: &mut RequestData) -> InterceptAction {
        if let Some(rule) = self.check_request(request) {
            tracing::debug!("Breakpoint hit: {} for {}", rule.name, request.url);
            // The pipeline creates the traffic entry before invoking
            // handlers, so we use a synthetic entry id here. The real
            // entry id is passed via the pipeline's breakpoint-specific
            // path (see `Pipeline::process_request`).
            //
            // This trait method is used by the unified handler loop; the
            // pipeline's dedicated breakpoint branch handles entry-id
            // wiring for the pause-and-wait flow.
            let entry_id = uuid::Uuid::new_v4().to_string();
            let decision = self
                .pause_and_wait(
                    entry_id,
                    InterceptDirection::Request,
                    request.clone(),
                    None,
                    rule.id.clone(),
                )
                .await;
            return breakpoint_decision_to_action(decision);
        }
        InterceptAction::Continue
    }

    async fn on_response(
        &self,
        request: &RequestData,
        response: &mut ResponseData,
    ) -> InterceptAction {
        if let Some(rule) = self.check_response(request, response) {
            tracing::debug!(
                "Breakpoint hit on response: {} for {}",
                rule.name,
                request.url
            );
            let entry_id = uuid::Uuid::new_v4().to_string();
            let decision = self
                .pause_and_wait(
                    entry_id,
                    InterceptDirection::Response,
                    request.clone(),
                    Some(response.clone()),
                    rule.id.clone(),
                )
                .await;
            return breakpoint_decision_to_action_response(decision, response);
        }
        InterceptAction::Continue
    }
}

#[async_trait::async_trait]
impl InterceptHandler for ThrottleManager {
    fn name(&self) -> &'static str {
        "throttle"
    }

    fn priority(&self) -> u32 {
        // Throttle applies latency right before forwarding, so it runs
        // last among request handlers.
        40
    }

    async fn on_request(&self, _request: &mut RequestData) -> InterceptAction {
        self.apply_latency().await;
        InterceptAction::Continue
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Build a [`ResponseData`] from a [`super::MockResponse`], honoring the
/// configured delay. This is the same logic as
/// `Pipeline::build_mock_response` but available without a `Pipeline`
/// reference so the trait implementation is self-contained.
async fn build_mock_response(mock_response: &super::MockResponse) -> ResponseData {
    if let Some(delay_ms) = mock_response.delay_ms {
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
    }
    ResponseData {
        status_code: mock_response.status_code,
        status_message: None,
        headers: mock_response.headers.clone(),
        body: mock_response.body_bytes(),
        content_type: mock_response.headers.get("Content-Type").cloned(),
        duration_ms: mock_response.delay_ms.unwrap_or(0),
        http_version: None,
    }
}

/// Convert a [`BreakpointDecision`] (request-side) into an [`InterceptAction`].
fn breakpoint_decision_to_action(decision: BreakpointDecision) -> InterceptAction {
    match decision {
        BreakpointDecision::Continue => InterceptAction::Continue,
        BreakpointDecision::Abort => InterceptAction::Abort,
        BreakpointDecision::Modify { .. } => {
            // Modifications are applied by the pipeline's breakpoint branch
            // (it has access to the entry id). When reached via the trait,
            // we treat it as continue — the dedicated pipeline path handles
            // the full modify flow.
            InterceptAction::Continue
        }
        BreakpointDecision::Respond {
            status_code,
            headers,
            body,
        } => InterceptAction::Respond(ResponseData {
            status_code,
            status_message: None,
            headers,
            body: body.map(|b| b.into_bytes()),
            content_type: Some("application/json".to_string()),
            duration_ms: 0,
            http_version: None,
        }),
    }
}

/// Convert a [`BreakpointDecision`] (response-side) into an [`InterceptAction`],
/// applying any modifications to the response in place.
fn breakpoint_decision_to_action_response(
    decision: BreakpointDecision,
    response: &mut ResponseData,
) -> InterceptAction {
    match decision {
        BreakpointDecision::Continue => InterceptAction::Continue,
        BreakpointDecision::Abort => InterceptAction::Abort,
        BreakpointDecision::Modify { modifications } => {
            BreakpointManager::apply_response_modifications(response, &modifications);
            InterceptAction::Continue
        }
        BreakpointDecision::Respond { .. } => {
            // Already have the response, just continue.
            InterceptAction::Continue
        }
    }
}
