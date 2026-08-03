//! Example plugin: logs every request method + host.
//!
//! Build: `cargo build --target wasm32-unknown-unknown --example request_logger --release`

#![no_std]
extern crate alloc;

use alloc::format;
use madhyamas_plugin_sdk::{log, log_level, register_plugin, Context, Outcome, Plugin};

#[derive(Default)]
struct RequestLogger;

impl Plugin for RequestLogger {
    fn on_load(&mut self, _ctx: &mut Context) -> Outcome {
        log(log_level::INFO, "request-logger loaded");
        Outcome::pass()
    }

    fn on_request(&mut self, ctx: &mut Context) -> Outcome {
        if let Some(req) = ctx.request() {
            let msg = format!("{} {}{}", req.method, req.host, req.path);
            log(log_level::INFO, &msg);
        }
        Outcome::pass()
    }
}

register_plugin!(RequestLogger);
