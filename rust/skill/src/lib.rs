// lifesavor-skill-sdk
//!
//! The Skill SDK for building skill provider integrations on the Life Savor
//! agent. Skill providers implement the [`SkillProvider`] trait and run as
//! sandboxed child processes communicating via JSON stdin/stdout or MCP
//! protocols.
//!
//! # Architecture
//!
//! This crate is a **thin re-export layer** over the
//! [`lifesavor-agent`](../lifesavor_agent/index.html) crate. It does not
//! duplicate agent internals; instead it depends on the agent as a library and
//! re-exports the public surface needed by skill developers. This guarantees
//! type identity — a [`ProviderManifest`] from this SDK is the exact same
//! Rust type as one from any other Life Savor SDK.
//!
//! # Target Trait
//!
//! The primary trait is [`SkillProvider`], defined in the agent's
//! `providers::skill_provider` module. Implementations provide `invoke`,
//! `list_tools`, `health_check`, and `capability_descriptor` methods. Skills
//! declare their tools via [`ToolSchema`] and run inside a
//! [`ProcessSandbox`] that restricts filesystem access, environment
//! variables, and resource consumption.
//!
//! # Development Workflow
//!
//! 1. **Build** — use [`builder::SkillProviderBuilder`] to scaffold a
//!    provider from a [`ProviderManifest`] and a set of
//!    [`ToolSchema`] definitions (constructed via
//!    [`builder::ToolSchemaBuilder`]).
//! 2. **Test** — use [`testing::MockSandbox`] to simulate sandbox
//!    restrictions (env var allowlists, filesystem checks, output size
//!    limits) without a running agent. Use the `sandbox_runner` binary for
//!    end-to-end sandbox testing.
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
// Re-exports: Skill Provider types (Req 5.1)
// ---------------------------------------------------------------------------

/// Core skill provider trait and types from the agent crate.
pub use lifesavor_agent::providers::skill_provider::{
    SkillProvider,
    ToolSchema,
    SkillCapabilityDescriptor,
    ExecutionLifecycleEvent,
    SkillProviderError,
    HealthStatus,
    EnforcementContext,
};

/// MCP transport type (feature-gated behind `mcp`).
#[cfg(feature = "mcp")]
pub use lifesavor_agent::providers::skill_provider::McpTransport;

// ---------------------------------------------------------------------------
// Re-exports: Bridge types (Req 5.3)
// ---------------------------------------------------------------------------

/// Bridge protocol types for sandboxed skill ↔ system component communication.
pub use lifesavor_agent::system_components::{
    BridgeRequest,
    BridgeResponse,
    SystemCallRequest,
    SystemCallResponse,
};

/// Structured error returned by the bridge.
pub use lifesavor_agent::system_components::bridge::BridgeError;

// ---------------------------------------------------------------------------
// Re-exports: Sandbox / Process types (Req 5.6, 10.1)
// ---------------------------------------------------------------------------

/// Process sandbox types for child-process isolation.
pub use lifesavor_agent::process::{
    ProcessSandbox,
    SandboxViolation,
    SandboxViolationType,
};

// ---------------------------------------------------------------------------
// Re-exports: Manifest types (Req 5.2, 6.1)
// ---------------------------------------------------------------------------

/// Provider manifest and validation types.
pub use lifesavor_agent::registry::manifest::{
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
// Re-exports: Error chain (Req 9.1)
// ---------------------------------------------------------------------------

/// Structured error chain types for cross-subsystem error reporting.
pub use lifesavor_agent::error::{
    ErrorChain,
    ErrorContext,
    Subsystem,
};

// ---------------------------------------------------------------------------
// Re-exports: Credential management (Req 8.1, 8.2)
// ---------------------------------------------------------------------------

/// Credential resolution types.
pub use lifesavor_agent::providers::credential_manager::{
    CredentialManager,
    ResolvedCredential,
    CredentialError,
};

// ---------------------------------------------------------------------------
// Re-exports: Tracing macros (Req 23.1)
// ---------------------------------------------------------------------------

/// Tracing macros for structured logging and instrumentation.
pub use tracing::{info, warn, error, debug, trace, instrument};

/// Tracing span type for manual span management.
pub use tracing::Span;

// ---------------------------------------------------------------------------
// Tracing helper (Req 23.2)
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
