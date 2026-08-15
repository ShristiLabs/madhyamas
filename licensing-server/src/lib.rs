//! Madhyamas licensing server library.
//!
//! This crate provides the core licensing server functionality: Ed25519
//! license signing, PostgreSQL database layer, and axum API handlers. The
//! `main.rs` binary wraps these into a standalone server application.

pub mod api;
pub mod auth;
pub mod db;
pub mod license;
