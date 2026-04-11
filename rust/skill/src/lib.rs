// lifesavor-skill-sdk
//!
//! The Skill SDK for building skill provider integrations on the Life Savor
//! agent. Skill providers implement the [`SkillProvider`] trait and run as
//! sandboxed child processes communicating via JSON stdin/stdout or MCP
//! protocols.
//!
//! # Architecture
//!
//! This crate is a **thin re-export layer** over
//! [`lifesavor-agent-types`](../lifesavor_agent_types/index.html). It depends
//! only on the lightweight shared types crate — not the full agent binary —
//! keeping build times fast and the transitive dependency count low (< 50
//! crates). Type identity is guaranteed: a [`ProviderManifest`] from this SDK
//! is the exact same Rust type as one from any other Life Savor SDK.
//!
//! # Target Trait
//!
//! The primary trait is [`SkillProvider`], defined in
//! `lifesavor-agent-types`. Implementations provide `invoke`, `list_tools`,
//! `health_check`, and `capability_descriptor` methods. Skills declare their
//! tools via [`ToolSchema`] and can be tested with [`testing::MockSandbox`]
//! for sandbox compliance.
//!
//! # Development Workflow
//!
//! 1. **Build** — use [`builder::SkillProviderBuilder`] to scaffold a
//!    provider from a [`ProviderManifest`] and a set of
//!    [`ToolSchema`] definitions (constructed via
//!    [`builder::ToolSchemaBuilder`]).
//! 2. **Test** — use [`testing::MockSandbox`] to simulate sandbox
//!    restrictions (env var allowlists, filesystem checks, output size
//!    limits) without a running agent. Use the `sandbox_runner` binary
//!    (requires `agent-runtime` feature) for end-to-end sandbox testing.
//! 3. **Deploy** — compile the binary, place a
//!    [`ProviderManifest`] TOML in the agent's `config_dir/providers/`
//!    directory, and the agent will hot-reload it.
//!
//! # Key Modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`prelude`] | Commonly used types for `use lifesavor_skill_sdk::prelude::*` |
//! | [`builder`] | [`SkillProviderBuilder`](builder::SkillProviderBuilder) and [`ToolSchemaBuilder`](builder::ToolSchemaBuilder) |
//! | [`health`] | [`HealthCheckBuilder`](health::HealthCheckBuilder) matching manifest config |
//! | [`mod@error`] | [`SkillSdkError`](error::SkillSdkError) with `into_error_context()` |
//! | [`testing`] | [`MockSandbox`](testing::MockSandbox) for isolated testing |
//! | [`security_surface`] | [`SecuritySurfaceReport`](security_surface::SecuritySurfaceReport) generation |
//! | [`build_config`] | [`BuildConfigBuilder`](build_config::BuildConfigBuilder) for `lifesavor-build.yml` |
//! | [`component_manifest`] | [`ComponentManifestBuilder`](component_manifest::ComponentManifestBuilder) for portal manifests |
//! | [`sandbox_compliance`] | [`SandboxComplianceChecker`](sandbox_compliance::SandboxComplianceChecker) for constraint validation |
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use lifesavor_skill_sdk::prelude::*;
//!
//! let tool = ToolSchemaBuilder::new()
//!     .name("greet")
//!     .description("Returns a greeting")
//!     .input_schema(serde_json::json!({"type": "object"}))
//!     .build()?;
//!
//! let manifest = parse_manifest_file("provider-manifest.toml")?;
//! let provider = SkillProviderBuilder::new(manifest)?.tool(tool).build();
//! ```

pub mod prelude;
pub mod builder;
pub mod health;
pub mod error;
pub mod testing;
pub mod security_surface;
pub mod build_config;
pub mod component_manifest;
pub mod sandbox_compliance;

#[cfg(feature = "analytics")]
pub mod analytics;

// ---------------------------------------------------------------------------
// Re-exports: Skill Provider types from lifesavor-agent-types
// ---------------------------------------------------------------------------

/// Core skill provider trait and types.
pub use lifesavor_agent_types::skill_provider::{
    SkillProvider,
    SkillCapabilityDescriptor,
    ExecutionLifecycleEvent,
    SkillProviderError,
    HealthStatus,
    SkillExecutionResult,
};

/// Tool schema type from the unified component declaration module.
pub use lifesavor_agent_types::component_declaration::ToolSchema;

/// MCP transport type (feature-gated behind `mcp`).
#[cfg(feature = "mcp")]
pub use lifesavor_agent_types::skill_provider::McpTransport;

// ---------------------------------------------------------------------------
// Re-exports: Bridge types from lifesavor-agent-types
// ---------------------------------------------------------------------------

/// Bridge protocol types for sandboxed skill ↔ system component communication.
pub use lifesavor_agent_types::bridge::{
    BridgeRequest,
    BridgeResponse,
    BridgeError,
    SystemCallRequest,
    SystemCallResponse,
};

// ---------------------------------------------------------------------------
// Re-exports: Sandbox types from lifesavor-agent-types
// ---------------------------------------------------------------------------

/// Sandbox violation types for process isolation reporting.
pub use lifesavor_agent_types::sandbox::{
    SandboxViolation,
    SandboxViolationType,
};

/// Process sandbox (requires `agent-runtime` feature for full agent support).
#[cfg(feature = "agent-runtime")]
pub use lifesavor_agent::process::ProcessSandbox;

// ---------------------------------------------------------------------------
// Re-exports: Manifest types from lifesavor-agent-types
// ---------------------------------------------------------------------------

/// Provider manifest and validation types.
pub use lifesavor_agent_types::manifest::{
    ProviderManifest,
    ProviderType,
    ConnectionConfig,
    AuthConfig,
    CredentialSource,
    HealthCheckConfig,
    HealthCheckMethod,
    CostLimits,
    SandboxConfig,
    Locality,
    CapabilityOverrides,
    ManifestValidationError,
    parse_manifest,
    parse_manifest_file,
    validate_manifest,
};

// ---------------------------------------------------------------------------
// Re-exports: Error chain from lifesavor-agent-types
// ---------------------------------------------------------------------------

/// Structured error chain types for cross-subsystem error reporting.
pub use lifesavor_agent_types::error_chain::{
    ErrorChain,
    ErrorContext,
    Subsystem,
};

// ---------------------------------------------------------------------------
// Re-exports: Credential types from lifesavor-agent-types
// ---------------------------------------------------------------------------

/// Credential resolution trait and types.
pub use lifesavor_agent_types::credential::{
    CredentialResolver,
    ResolvedCredential,
    CredentialError,
};

/// Concrete credential manager (requires `agent-runtime` feature).
#[cfg(feature = "agent-runtime")]
pub use lifesavor_agent::providers::credential_manager::CredentialManager;

// ---------------------------------------------------------------------------
// Re-exports: Tracing macros
// ---------------------------------------------------------------------------

/// Tracing macros for structured logging and instrumentation.
pub use tracing::{info, warn, error, debug, trace, instrument};

/// Tracing span type for manual span management.
pub use tracing::Span;

// ---------------------------------------------------------------------------
// Tracing helper
// ---------------------------------------------------------------------------

/// Create a tracing span pre-populated with correlation context fields.
pub fn span_with_context(
    correlation_id: &str,
    user_id: Option<&str>,
    instance_id: &str,
) -> tracing::Span {
    tracing::info_span!(
        "skill_sdk",
        correlation_id = %correlation_id,
        user_id = user_id.unwrap_or(""),
        instance_id = %instance_id,
    )
}
