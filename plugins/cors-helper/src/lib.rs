//! Example plugin: adds CORS headers to every response.
//!
//! Build: `cargo build --target wasm32-unknown-unknown --example cors_helper --release`
//! Then copy the resulting `.wasm` to your plugin dir as `plugin.wasm`.

#![no_std]
extern crate alloc;

use madhyamas_plugin_sdk::{log, log_level, register_plugin, Context, Outcome, Plugin};

#[derive(Default)]
struct CorsHelper;

impl Plugin for CorsHelper {
    fn on_load(&mut self, _ctx: &mut Context) -> Outcome {
        log(log_level::INFO, "cors-helper loaded");
        Outcome::pass()
    }

    fn on_response(&mut self, ctx: &mut Context) -> Outcome {
        if let Some(resp) = ctx.response_mut() {
            resp.headers
                .insert("Access-Control-Allow-Origin".into(), "*".into());
            resp.headers.insert(
                "Access-Control-Allow-Methods".into(),
                "GET, POST, PUT, DELETE, OPTIONS".into(),
            );
            resp.headers.insert(
                "Access-Control-Allow-Headers".into(),
                "Content-Type, Authorization".into(),
            );
        }
        log(log_level::DEBUG, "cors-helper added CORS headers");
        Outcome::modified()
    }
}

register_plugin!(CorsHelper);
