# Changelog — lifesavor-model-sdk

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-01-01

### Added

- Initial release of the Model SDK
- Re-exports of `LlmProvider`, `ChatRequest`, `ModelInfo`, `ModelLoadStatus`, `CapabilityDescriptor`
- Re-exports of `InferenceError`, `InferenceMetrics`, `TokenEvent`
- Re-exports of `ProviderManifest`, `StreamingEnvelope`, `ErrorChain`, `CredentialManager`
- `ModelProviderBuilder` for scaffold generation with manifest validation
- `ModelSdkError` with `From` conversions and `into_error_context()`
- `HealthCheckBuilder` supporting `HttpGet`, `ConnectionTest`, `ProcessAlive` methods
- `MockRegistry` test harness for isolated provider testing
- `SecuritySurfaceReport` generation from provider manifests
- `BuildConfigBuilder` and `ComponentManifestBuilder` for Developer Portal integration
- `span_with_context` tracing helper
- `AnalyticsReporter` (behind `analytics` feature flag)
- Feature flag: `analytics`
- Re-exports of extended `ChatMessage` (with `images`, `tool_calls`, `tool_call_id`), `ToolCall`, `ToolDefinition`
- Re-exports of new `InferenceError` variants: `AuthenticationFailed`, `RateLimited`, `ProviderUnavailable`
- Examples: `native_provider`, `mock_provider`, `hot_cold_management`, `sandbox_compliance`
- Templates: `lifesavor-build.yml`, `component-manifest.toml`, `README.md`
