# Changelog — lifesavor-assistant-sdk

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-01-01

### Added

- Initial release of the Assistant SDK
- Re-exports of `AssistantProvider`, `AssistantDefinition`, `AssistantSummary`, `ResolvedAssistant`
- Re-exports of `ToolBinding`, `GuardrailRule`, `HandoffConfig`, `validate_definition`, `substitute_variables`
- Re-exports of `ProviderManifest`, `ErrorChain`, `CredentialManager`, manifest types
- `AssistantDefinitionBuilder` with required field validation and template variable checking
- `AssistantProviderBuilder` for scaffold generation with manifest validation
- `AssistantSdkError` with `From` conversions and `into_error_context()`
- `HealthCheckBuilder` supporting `HttpGet`, `ConnectionTest`, `ProcessAlive` methods
- `MockAssistantStore` test harness for isolated provider testing
- `SecuritySurfaceReport` generation from provider manifests
- `BuildConfigBuilder` and `ComponentManifestBuilder` for Developer Portal integration
- `span_with_context` tracing helper
- `AnalyticsReporter` (behind `analytics` feature flag)
- Examples: `local_fs_provider`, `assistant_definition`, `validation`, `sandbox_compliance`
- Templates: `lifesavor-build.yml`, `component-manifest.toml`, `README.md`
