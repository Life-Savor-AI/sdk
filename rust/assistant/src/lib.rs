// lifesavor-assistant-sdk
//!
//! The Assistant SDK for building assistant provider integrations on the
//! Life Savor agent. Assistant providers implement the [`AssistantProvider`]
//! trait and manage declarative assistant definitions — loading, listing,
//! resolving, and validating them.
//!
//! # Architecture
//!
//! This crate is a **thin re-export layer** over the
//! [`lifesavor-agent`](../lifesavor_agent/index.html) crate. It does not
//! duplicate agent internals; instead it depends on the agent as a library and
//! re-exports the public surface needed by assistant provider developers. This
//! guarantees type identity — a [`ProviderManifest`] from this SDK is the
//! exact same Rust type as one from any other Life Savor SDK.
//!
//! # Target Trait
//!
//! The primary trait is [`AssistantProvider`], defined in the agent's
//! `providers::assistant_provider` module. Implementations provide `load`,
//! `list`, and `resolve` methods for managing [`AssistantDefinition`]
//! instances that declare prompt templates, tool bindings, guardrail rules,
//! and handoff configuration.
//!
//! # Development Workflow
//!
//! 1. **Build** — use [`builder::AssistantDefinitionBuilder`] to construct
//!    validated definitions, and [`builder::AssistantProviderBuilder`] to
//!    scaffold a provider from a [`ProviderManifest`].
//! 2. **Test** — use [`testing::MockAssistantStore`] to simulate definition
//!    storage with `load(id)` and `list()` without real filesystem access.
//! 3. **Deploy** — compile the binary, place a
//!    [`ProviderManifest`] TOML in the agent's `config_dir/providers/`
//!    directory, and the agent will hot-reload it.
//!
//! # Key Modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`prelude`] | Commonly used types for `use lifesavor_assistant_sdk::prelude::*` |
//! | [`builder`] | [`AssistantDefinitionBuilder`](builder::AssistantDefinitionBuilder) and [`AssistantProviderBuilder`](builder::AssistantProviderBuilder) |
//! | [`health`] | [`HealthCheckBuilder`](health::HealthCheckBuilder) matching manifest config |
//! | [`mod@error`] | [`AssistantSdkError`](error::AssistantSdkError) with `into_error_context()` |
//! | [`testing`] | [`MockAssistantStore`](testing::MockAssistantStore) for isolated testing |
//! | [`security_surface`] | [`SecuritySurfaceReport`](security_surface::SecuritySurfaceReport) generation |
//! | [`build_config`] | [`BuildConfigBuilder`](build_config::BuildConfigBuilder) for `lifesavor-build.yml` |
//! | [`component_manifest`] | [`ComponentManifestBuilder`](component_manifest::ComponentManifestBuilder) for portal manifests |
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use lifesavor_assistant_sdk::prelude::*;
//!
//! let definition = AssistantDefinitionBuilder::new()
//!     .id("my-assistant")
//!     .display_name("My Assistant")
//!     .system_prompt_template("You are {{role}}.")
//!     .variable("role", "a helpful assistant")
//!     .build()?;
//! ```

pub mod prelude;
pub mod builder;
pub mod health;
pub mod error;
pub mod testing;
pub mod security_surface;
pub mod build_config;
pub mod component_manifest;

#[cfg(feature = "analytics")]
pub mod analytics;

// ---------------------------------------------------------------------------
// Re-exports: Assistant Provider types (Req 4.1)
// ---------------------------------------------------------------------------

/// Core assistant provider trait and types from the agent crate.
pub use lifesavor_agent::providers::assistant_provider::{
    AssistantProvider,
    AssistantDefinition,
    AssistantSummary,
    ResolvedAssistant,
    AssistantProviderError,
    ToolBinding,
    GuardrailRule,
    HandoffConfig,
    validate_definition,
    substitute_variables,
};

// ---------------------------------------------------------------------------
// Re-exports: Manifest types (Req 4.2, 6.1)
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
// Re-exports: Health status (Req 7.5)
// ---------------------------------------------------------------------------

/// Health status enum returned by provider health checks.
pub use lifesavor_agent::providers::skill_provider::HealthStatus;

// ---------------------------------------------------------------------------
// Re-exports: Error chain (Req 4.6, 9.1)
// ---------------------------------------------------------------------------

/// Structured error chain types for cross-subsystem error reporting.
pub use lifesavor_agent::error::{
    ErrorChain,
    ErrorContext,
    Subsystem,
};

// ---------------------------------------------------------------------------
// Re-exports: Sandbox / Process types (Req 10.1)
// ---------------------------------------------------------------------------

/// Process sandbox types for child-process isolation.
pub use lifesavor_agent::process::{
    ProcessSandbox,
    SandboxViolation,
    SandboxViolationType,
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
        "assistant_sdk",
        correlation_id = %correlation_id,
        user_id = user_id.unwrap_or(""),
        instance_id = %instance_id,
    )
}
