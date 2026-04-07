# lifesavor-agent-types

Shared interface types for the Life Savor agent and SDK ecosystem.

## Purpose

This crate is the single source of truth for all types shared between the agent runtime and the SDK/component crates. Both the agent and the system SDK depend on this crate, ensuring type identity across the ecosystem.

```
lifesavor-system-sdk ──► lifesavor-agent-types ◄── lifesavor-agent
```

## Design Principles

- **Zero runtime dependencies** — no `tokio`, no agent-specific crates. Only serialization, error-handling, and async trait crates.
- **Shared types only** — traits, structs, enums, and parsing functions. No runtime logic, no dispatch, no state management.
- **Security boundary** — privileged agent types (`Vault`, `CredentialManager` struct, `ProcessSandbox`, `SystemComponentRegistry`) are excluded.

## Modules

| Module | Contents |
|--------|----------|
| `system_component` | `SystemComponent` trait, `SystemComponentType`, `ComponentHealthStatus`, `SystemComponentInfo` |
| `bridge` | `BridgeRequest`, `BridgeResponse`, `BridgeError`, `SystemCallRequest`, `SystemCallResponse`, `BridgeRateLimit` |
| `streaming` | `StreamingEnvelope`, `StreamStatus`, `StreamMetadata` |
| `error_chain` | `ErrorChain`, `ErrorContext`, `Subsystem` |
| `manifest` | `ProviderManifest`, all config types, `parse_manifest`, `validate_manifest` |
| `sandbox` | `SandboxViolation`, `SandboxViolationType` |
| `credential` | `CredentialResolver` trait, `ResolvedCredential`, `CredentialError` |

## Usage

Component developers should not depend on this crate directly. Use `lifesavor-system-sdk` instead, which re-exports everything you need.

For agent developers adding new shared types:

```toml
[dependencies]
lifesavor-agent-types = { path = "../agent" }
```

## Testing

All serializable types have property-based serde round-trip tests:

```bash
cargo test -p lifesavor-agent-types
```

## License

[MIT](LICENSE)
