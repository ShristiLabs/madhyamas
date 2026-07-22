//! Madhyamas CLI library
//!
//! Exposes the CLI command structure and API client for embedding
//! in the unified `madhyamas` binary.

pub mod commands;

pub use commands::{ApiClient, Commands};
