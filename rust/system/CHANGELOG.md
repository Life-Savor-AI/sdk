# Changelog — lifesavor-system-sdk

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - Unreleased

### Changed

- **BREAKING**: `SystemComponentType::VectorStore` renamed to `MemoryStore` across all types and serialization
- **BREAKING**: `ProviderType::VectorStore` renamed to `MemoryStore`
- **BREAKING**: Per-crate `ComponentDeclaration` replaced with unified type from `lifesavor-agent-types`
- Dynamic bridge dispatch replaces hard-coded `dispatch_to_component` match statement
- `stubs.rs` deprecated in favor of registry-based dispatch

### Added

- `ComponentDeclaration` unified type (re-exported from `lifesavor-agent-types`)
- `SystemComponent::tool_schemas()` method for self-describing component operations
- `SystemComponent::declaration()` method for component metadata
- Multi-instance registry support with `register_with_instance_id()`, `get_components_by_type()`, `get_component_by_instance_id()`
- Instance-qualified MCP tool naming: `system.<type>.<instance_id>.<operation>`
- JSON-RPC external component support via `ExternalComponentProxy`
- `bridge_validation` helpers for operation and parameter validation
- Health reporting types: `ComponentMetrics`, `HealthSummary`, `ResourceUsage`

### Notes

- Path dependencies must be replaced with version dependencies before publishing to crates.io
- See `../PUBLISHING.md` for the full publishing workflow
- Publish `lifesavor-agent-types` first, then this crate

## [0.2.0] - 2026-04-07

### Changed

- **BREAKING**: SDK now depends on `lifesavor-agent-types` instead of `lifesavor-agent`. Component crates no longer transitively pull in the agent runtime.
- **BREAKING**: Removed `SystemComponentRegistry`, `SystemComponentBridge`, `BridgeRateLimiter`, `ProcessSandbox` from public re-exports (these are agent runtime types).
- **BREAKING**: `CredentialManager` struct replaced with `CredentialResolver` trait in prelude and re-exports.
- **BREAKING**: `SystemComponent` trait error type changed from `AgentError` to `Box<dyn std::error::Error + Send + Sync>`.
- **BREAKING**: Feature flags `tts`, `stt`, `file-storage`, `messaging`, `calendar`, `device-control`, `cache` removed (shared types are now always available from `agent-types`).
- All shared types now sourced from `lifesavor-agent-types` crate.
- SDK-owned modules (`builder`, `health`, `error`, `testing`, `security_surface`, `build_config`, `component_manifest`) updated to use `lifesavor_agent_types` import paths.

### Added

- `bridge_validation` module with `validate_operation` and `extract_required_param` helpers.
- Health reporting types: `ComponentMetrics`, `HealthSummary`, `ResourceUsage`, `MetricsCollector`, `ComponentHealthReporter` trait.

## [0.1.0] - 2025-01-01

### Added

- Initial release of the System SDK
- Re-exports of `SystemComponent`, `SystemComponentType`, `ComponentHealthStatus`, `SystemComponentInfo`
- Re-exports of `SystemComponentBridge` types (`BridgeRequest`, `BridgeResponse`, `BridgeError`)
- Re-exports of `StreamingEnvelope`, `ErrorChain`, `CredentialManager`, manifest types
- `SystemComponentBuilder` for ergonomic component construction
- `SystemSdkError` with `From` conversions and `into_error_context()`
- `HealthCheckBuilder` supporting `HttpGet`, `ConnectionTest`, `ProcessAlive` methods
- `MockAgentContext` test harness for isolated component testing
- `SecuritySurfaceReport` generation from provider manifests
- `BuildConfigBuilder` and `ComponentManifestBuilder` for Developer Portal integration
- `span_with_context` tracing helper
- `AnalyticsReporter` (behind `analytics` feature flag)
- Feature flags: `tts`, `stt`, `file-storage`, `messaging`, `calendar`, `device-control`, `cache`
- Examples: `tts_component`, `cache_component`, `bridge_consumer`, `sandbox_compliance`
- Templates: `lifesavor-build.yml`, `component-manifest.toml`, `README.md`
