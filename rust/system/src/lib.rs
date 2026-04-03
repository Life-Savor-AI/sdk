// lifesavor-system-sdk
//!
//! The System SDK for building first-party system components on the Life Savor
//! agent. System components implement the [`SystemComponent`] trait and run
//! in-process with privileged access to agent internals — TTS, STT, cache,
//! file storage, messaging, calendar, device control, and more.
//!
//! # Architecture
//!
//! This crate is a **thin re-export layer** over the
//! [`lifesavor-agent`](../lifesavor_agent/index.html) crate. It does not
//! duplicate agent internals; instead it depends on the agent as a library and
//! re-exports the public surface needed by system component developers. This
//! guarantees type identity — a [`ProviderManifest`] from this SDK is the
//! exact same Rust type as one from any other Life Savor SDK.
//!
//! # Target Trait
//!
//! The primary trait is [`SystemComponent`], defined in the agent's
//! `system_components` module. Implementations provide `initialize`,
//! `health_check`, and `shutdown` lifecycle hooks plus a component name and
//! [`SystemComponentType`].
//!
//! # Development Workflow
//!
//! 1. **Build** — use [`builder::SystemComponentBuilder`] to construct a
//!    component from closures without manually implementing the trait.
//! 2. **Test** — use [`testing::MockAgentContext`] to exercise the full
//!    `initialize → health_check → shutdown` lifecycle in isolation, with no
//!    running agent required.
//! 3. **Deploy** — compile the binary, place a
//!    [`ProviderManifest`] TOML in the agent's `config_dir/providers/`
//!    directory, and the agent will hot-reload it.
//!
//! # Key Modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`prelude`] | Commonly used types for `use lifesavor_system_sdk::prelude::*` |
//! | [`builder`] | [`SystemComponentBuilder`](builder::SystemComponentBuilder) for guided construction |
//! | [`health`] | [`HealthCheckBuilder`](health::HealthCheckBuilder) matching manifest config |
//! | [`mod@error`] | [`SystemSdkError`](error::SystemSdkError) with `into_error_context()` |
//! | [`testing`] | [`MockAgentContext`](testing::MockAgentContext) for isolated testing |
//! | [`security_surface`] | [`SecuritySurfaceReport`](security_surface::SecuritySurfaceReport) generation |
//! | [`build_config`] | [`BuildConfigBuilder`](build_config::BuildConfigBuilder) for `lifesavor-build.yml` |
//! | [`component_manifest`] | [`ComponentManifestBuilder`](component_manifest::ComponentManifestBuilder) for portal manifests |
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use lifesavor_system_sdk::prelude::*;
//!
//! let component = SystemComponentBuilder::new("my-tts", SystemComponentType::Tts)
//!     .on_initialize(|| Box::pin(async { Ok(()) }))
//!     .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
//!     .on_shutdown(|| Box::pin(async { Ok(()) }))
//!     .build()
//!     .expect("valid component");
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
// Re-exports: System Component types (Req 2.1)
// ---------------------------------------------------------------------------

/// Core system component trait and types from the agent crate.
pub use lifesavor_agent::system_components::{
    SystemComponent,
    SystemComponentType,
    ComponentHealthStatus,
    SystemComponentInfo,
    SystemComponentRegistry,
};

// ---------------------------------------------------------------------------
// Re-exports: Bridge types (Req 2.2)
// ---------------------------------------------------------------------------

/// Bridge protocol types for sandboxed provider ↔ system component communication.
pub use lifesavor_agent::system_components::{
    SystemComponentBridge,
    BridgeRequest,
    BridgeResponse,
    SystemCallRequest,
    SystemCallResponse,
    BridgeRateLimit,
    BridgeRateLimiter,
};

/// Structured error returned by the bridge.
pub use lifesavor_agent::system_components::bridge::BridgeError;

// ---------------------------------------------------------------------------
// Re-exports: Streaming envelope (Req 2.4)
// ---------------------------------------------------------------------------

/// Unified streaming envelope for WebSocket message framing.
pub use lifesavor_agent::streaming::{
    StreamingEnvelope,
    StreamStatus,
    StreamMetadata,
};

// ---------------------------------------------------------------------------
// Re-exports: Error chain (Req 2.5, 9.1)
// ---------------------------------------------------------------------------

/// Structured error chain types for cross-subsystem error reporting.
pub use lifesavor_agent::error::{
    ErrorChain,
    ErrorContext,
    Subsystem,
};

// ---------------------------------------------------------------------------
// Re-exports: Manifest types (Req 6.1)
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
        "system_sdk",
        correlation_id = %correlation_id,
        user_id = user_id.unwrap_or(""),
        instance_id = %instance_id,
    )
}
