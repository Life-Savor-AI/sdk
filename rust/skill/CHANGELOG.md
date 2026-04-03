# Changelog — lifesavor-skill-sdk

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-01-01

### Added

- Initial release of the Skill SDK
- Re-exports of `SkillProvider`, `ToolSchema`, `SkillCapabilityDescriptor`, `McpTransport`
- Re-exports of `ExecutionLifecycleEvent`, `SkillProviderError`, `HealthStatus`, `EnforcementContext`
- Re-exports of `BridgeRequest`, `BridgeResponse`, `SandboxConfig`, `ProcessSandbox`
- Re-exports of `ProviderManifest`, `ErrorChain`, `CredentialManager`, manifest types
- `SkillProviderBuilder` with JSON stdin/stdout scaffold and manifest validation
- `ToolSchemaBuilder` with JSON Schema input validation
- `SandboxComplianceChecker` for local sandbox constraint verification
- `SkillSdkError` with `From` conversions and `into_error_context()`
- `HealthCheckBuilder` supporting `HttpGet`, `ConnectionTest`, `ProcessAlive` methods
- `MockSandbox` test harness for isolated sandbox testing
- `SecuritySurfaceReport` generation from provider manifests
- `BuildConfigBuilder` and `ComponentManifestBuilder` for Developer Portal integration
- `span_with_context` tracing helper
- `AnalyticsReporter` (behind `analytics` feature flag)
- `sandbox-runner` binary for local sandbox testing
- Feature flags: `mcp`
- Examples: `json_stdio_skill`, `mcp_skill`, `bridge_access`, `sandbox_compliance`
- Templates: `lifesavor-build.yml`, `component-manifest.toml`, `README.md`
