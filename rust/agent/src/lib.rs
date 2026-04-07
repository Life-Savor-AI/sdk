//! Shared interface types for the Life Savor agent and SDK ecosystem.
//!
//! This crate is the single source of truth for all types shared between
//! the agent runtime and the system SDK. It has no runtime dependencies
//! on tokio or agent-specific crates.

pub mod system_component;
pub mod bridge;
pub mod streaming;
pub mod error_chain;
pub mod manifest;
pub mod sandbox;
pub mod credential;
