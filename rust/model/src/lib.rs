// lifesavor-model-sdk
//!
//! The Model SDK for building LLM provider integrations on the Life Savor
//! agent. LLM providers implement the [`LlmProvider`] trait and run as
//! child processes or in-process backends for streaming chat completion,
//! model listing, embedding generation, and hot/cold model management.
//!
//! # Architecture
//!
//! This crate is a **thin re-export layer** over the
//! [`lifesavor-agent`](../lifesavor_agent/index.html) crate. It does not
//! duplicate agent internals; instead it depends on the agent as a library and
//! re-exports the public surface needed by LLM provider developers. This
//! guarantees type identity — a [`ProviderManifest`] from this SDK is the
//! exact same Rust type as one from any other Life Savor SDK.
//!
//! # Target Trait
//!
//! The primary trait is [`LlmProvider`], defined in the agent's
//! `providers::llm_provider` module. Implementations provide
//! `chat_completion_stream`, `list_models`, `model_load_status`,
//! `generate_embedding`, `capability_descriptor`, and `resolve_model_alias`.
//!
//! # Development Workflow
//!
//! 1. **Build** — use [`builder::ModelProviderBuilder`] to construct a
//!    provider scaffold from a [`ProviderManifest`]. The scaffold provides
//!    `unimplemented!()` stubs you fill in incrementally.
//! 2. **Test** — use [`testing::MockRegistry`] to simulate provider
//!    registration, health monitoring, and routing without a running agent.
//! 3. **Deploy** — compile the binary, place a
//!    [`ProviderManifest`] TOML in the agent's `config_dir/providers/`
//!    directory, and the agent will hot-reload it.
//!
//! # Key Modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`prelude`] | Commonly used types for `use lifesavor_model_sdk::prelude::*` |
//! | [`builder`] | [`ModelProviderBuilder`](builder::ModelProviderBuilder) for guided construction |
//! | [`health`] | [`HealthCheckBuilder`](health::HealthCheckBuilder) matching manifest config |
//! | [`mod@error`] | [`ModelSdkError`](error::ModelSdkError) with `into_error_context()` |
//! | [`testing`] | [`MockRegistry`](testing::MockRegistry) for isolated testing |
//! | [`security_surface`] | [`SecuritySurfaceReport`](security_surface::SecuritySurfaceReport) generation |
//! | [`build_config`] | [`BuildConfigBuilder`](build_config::BuildConfigBuilder) for `lifesavor-build.yml` |
//! | [`component_manifest`] | [`ComponentManifestBuilder`](component_manifest::ComponentManifestBuilder) for portal manifests |
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use lifesavor_model_sdk::prelude::*;
//!
//! let manifest = parse_manifest_file("provider-manifest.toml")?;
//! let provider = ModelProviderBuilder::new(manifest)?.build();
//! ```

/// Convenience re-exports of the most commonly used SDK types.
pub mod prelude;
/// `ModelProviderBuilder` for scaffold `LlmProvider` implementations from a manifest.
pub mod builder;
/// `ComponentHealthReporter` for health status tracking and reporting.
pub mod health;
/// SDK-specific error types and conversions.
pub mod error;
/// Test utilities and mock providers for component testing.
pub mod testing;
/// `SecuritySurfaceReport` generation from provider manifests.
pub mod security_surface;
/// Build configuration helpers for `lifesavor-build.yml` integration.
pub mod build_config;
/// `component-manifest.toml` parsing and validation.
pub mod component_manifest;

#[cfg(feature = "analytics")]
pub mod analytics;

// ---------------------------------------------------------------------------
// Re-exports: LLM Provider types (Req 3.1)
// ---------------------------------------------------------------------------

/// Core LLM provider trait and types from the agent crate.
pub use lifesavor_agent::providers::llm_provider::{
    LlmProvider,
    ChatRequest,
    ModelInfo,
    CapabilityDescriptor,
    ModelCapability,
    ModelLocality,
    PricingTier,
    LatencyClass,
    ToolDefinition,
};

// ---------------------------------------------------------------------------
// Re-exports: Inference types (Req 1.6, 2.5, 3.6)
// ---------------------------------------------------------------------------

/// Inference error (including `AuthenticationFailed`, `RateLimited`,
/// `ProviderUnavailable` variants — Req 2.5), metrics, token events, and
/// model load status from the agent crate's inference engine.
pub use lifesavor_agent::inference::{
    CancellableInference,
    InferenceError,
    InferenceMetrics,
    ModelLoadStatus,
    TokenEvent,
    content_type,
};

/// Extended chat message type with optional `images`, `tool_calls`, and
/// `tool_call_id` fields (Req 1.6).
pub use lifesavor_agent::inference::ChatMessage;

/// Function call response type returned by models that support tool use
/// (Req 1.6).
pub use lifesavor_agent::inference::inference_queue::ToolCall;

// ---------------------------------------------------------------------------
// Re-exports: Health status (Req 7.5)
// ---------------------------------------------------------------------------

/// Health status enum returned by provider health checks.
pub use lifesavor_agent::providers::skill_provider::HealthStatus;

// ---------------------------------------------------------------------------
// Re-exports: Manifest types (Req 3.2, 6.1)
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
// Re-exports: Model Profile Integration types (Req 58)
// ---------------------------------------------------------------------------

/// Model profile types for displaying installed components grouped by
/// provider type in the web app's model profile settings.
pub use lifesavor_agent::registry::{
    ModelProviderType,
    ModelProfileEntry,
    ModelProfile,
};

// ---------------------------------------------------------------------------
// Re-exports: Streaming envelope (Req 3.3)
// ---------------------------------------------------------------------------

/// Unified streaming envelope for WebSocket message framing.
pub use lifesavor_agent::streaming::{
    StreamingEnvelope,
    StreamStatus,
    StreamMetadata,
};

// ---------------------------------------------------------------------------
// Re-exports: Error chain (Req 3.5, 9.1)
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
// Re-exports: RAG provider types (Req 42.1 – 42.8)
// ---------------------------------------------------------------------------

/// RAG provider trait and request/response types for knowledge base access.
/// Model components use these types to interact with the agent's knowledge
/// store via JSON-RPC `rag.search` and `rag.upsert` methods.
pub use lifesavor_agent::skills::rag_provider::{
    RagProvider,
    RagSearchRequest,
    RagUpsertRequest,
    RagGetRequest,
    RagForgetRequest,
    RagResult,
    RagProviderStatus,
};

/// JSON-RPC parameter types for RAG methods.
pub use lifesavor_agent::jsonrpc::{
    RagSearchParams,
    RagUpsertParams,
};

/// JSON-RPC parameter and result types for vault decrypt (Req 17.4).
pub use lifesavor_agent::jsonrpc::{
    VaultDecryptParams,
    VaultDecryptResult,
    PII_VAULT_SYSTEM_PROMPT,
    content_contains_vault_tags,
};

/// JSON-RPC parameter and result types for inter-component `tools/call`
/// (Req 23.1 – 23.7, 24.1 – 24.6).
pub use lifesavor_agent::jsonrpc::{
    ToolsCallParams,
    ToolsCallResult,
    ToolsCallError,
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
        "model_sdk",
        correlation_id = %correlation_id,
        user_id = user_id.unwrap_or(""),
        instance_id = %instance_id,
    )
}
