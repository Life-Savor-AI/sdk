# Changelog — lifesavor-agent-types

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-07

### Added

- Initial release of the shared interface types crate
- `SystemComponentType` enum with all component variants including `MemoryStore` (renamed from `VectorStore`)
- `ProviderType` enum with `MemoryStore` variant (renamed from `VectorStore`)
- `ComponentDeclaration` unified type shared across all SDK crates
- `SystemComponent` trait with `tool_schemas()` and `declaration()` optional methods
- `BridgeRequest` and `BridgeResponse` types for skill ↔ component communication
- `ErrorChain` type for structured error propagation with subsystem, code, message, and correlation ID
- `ToolSchema` type for self-describing component operations
- `ProviderManifest` and related manifest types for component/skill configuration
- `SandboxConfig` and sandbox-related types
- `CredentialResolver` async trait for credential management
- `StreamingEnvelope` for streaming response support
- Property-based serde round-trip tests for all serializable types
- Zero runtime dependencies (no tokio, no agent-specific crates)

### Notes

- This crate is the root of the SDK dependency tree and must be published to crates.io first
- See `../PUBLISHING.md` for the full publishing workflow
