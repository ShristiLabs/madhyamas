//! Example plugin: blocks requests to a configurable list of domains and
//! returns a 403. Demonstrates settings + short-circuit response.
//!
//! Build: `cargo build --target wasm32-unknown-unknown --example domain_blocker --release`

#![no_std]
extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use madhyamas_plugin_sdk::{log, log_level, register_plugin, Context, Outcome, Plugin};

#[derive(Default)]
struct DomainBlocker;

impl Plugin for DomainBlocker {
    fn on_request(&mut self, ctx: &mut Context) -> Outcome {
        // Read the blocked-domains setting (a JSON array of strings).
        let blocked: Vec<String> = ctx
            .setting("blocked_domains")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();

        if let Some(req) = ctx.request() {
            if blocked.iter().any(|d| &req.host == d) {
                log(
                    log_level::WARN,
                    &format!("blocking request to {}", req.host),
                );
                return Outcome::respond(403, "blocked by domain-blocker plugin");
            }
        }
        Outcome::pass()
    }
}

register_plugin!(DomainBlocker);
