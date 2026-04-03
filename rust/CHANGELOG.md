# Changelog — Life Savor Rust SDK Suite

Aggregated notable changes across all four SDK crates. For per-crate details, see each crate's own `CHANGELOG.md`.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-01-01

### Added

#### All SDKs
- Re-exports of agent crate traits, shared types, error types, and manifest structures
- Per-SDK `thiserror`-based error enums with `From` conversions and `into_error_context()`
- `HealthCheckBuilder` supporting `HttpGet`, `ConnectionTest`, `ProcessAlive` methods
- `SecuritySurfaceReport` generation from provider manifests (JSON and Markdown output)
- `BuildConfigBuilder` and `ComponentManifestBuilder` for Developer Portal integration
- `span_with_context` tracing helper for correlation ID propagation
- `AnalyticsReporter` (behind `analytics` feature flag) for Developer Portal event reporting
- Starter templates: `lifesavor-build.yml`, `component-manifest.toml`, `README.md`
- Per-crate documentation: `README.md`, `COMPATIBILITY.md`, `CHANGELOG.md`, `docs/GETTING_STARTED.md`, `docs/DEPLOYMENT.md`

#### lifesavor-system-sdk
- `SystemComponentBuilder` for ergonomic component construction
- `MockAgentContext` test harness for isolated lifecycle testing
- Feature flags: `tts`, `stt`, `file-storage`, `messaging`, `calendar`, `device-control`, `cache`
- Examples: `tts_component`, `cache_component`, `bridge_consumer`, `sandbox_compliance`

#### lifesavor-model-sdk
- `ModelProviderBuilder` for scaffold generation with manifest type validation
- `MockRegistry` test harness for provider registration and routing
- Feature flags: `bedrock`, `openai`
- Examples: `ollama_provider`, `mock_provider`, `hot_cold_management`, `sandbox_compliance`

#### lifesavor-assistant-sdk
- `AssistantDefinitionBuilder` with required field validation and template variable checking
- `AssistantProviderBuilder` for scaffold generation with manifest type validation
- `MockAssistantStore` test harness for definition storage simulation
- Examples: `local_fs_provider`, `assistant_definition`, `validation`, `sandbox_compliance`

#### lifesavor-skill-sdk
- `SkillProviderBuilder` with JSON stdin/stdout scaffold and manifest type validation
- `ToolSchemaBuilder` with JSON Schema input validation
- `SandboxComplianceChecker` for local sandbox constraint verification
- `MockSandbox` test harness for sandbox restriction simulation
- `sandbox-runner` binary for local sandbox testing without a running agent
- Feature flag: `mcp`
- Examples: `json_stdio_skill`, `mcp_skill`, `bridge_access`, `sandbox_compliance`
