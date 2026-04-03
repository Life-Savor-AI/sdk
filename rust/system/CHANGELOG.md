# Changelog — lifesavor-system-sdk

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
