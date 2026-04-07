# lifesavor-system-sdk

Build first-party system components for the Life Savor agent using the `SystemComponent` trait.

System components are privileged, in-process modules that provide core agent capabilities such as TTS, STT, file storage, messaging, caching, and device control. Unlike third-party providers, system components run without sandbox restrictions and have direct access to agent internals.

## Target Trait

[`SystemComponent`](https://docs.rs/lifesavor-agent-types/latest/lifesavor_agent_types/system_component/trait.SystemComponent.html) — defines `initialize`, `health_check`, and `shutdown` lifecycle methods plus component name and type metadata.

## Prerequisites

- Rust toolchain **1.75+** (edition 2021)
- Access to the `lifesavor-system-sdk` crate (path dependency or published version)
- Familiarity with `async-trait` and `tokio`

## Quickstart

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
lifesavor-system-sdk = { path = "../SDK/system" }
tokio = { version = "1", features = ["full"] }
```

Build a minimal system component:

```rust
use lifesavor_system_sdk::prelude::*;
use lifesavor_system_sdk::builder::SystemComponentBuilder;

#[tokio::main]
async fn main() -> lifesavor_system_sdk::error::Result<()> {
    let component = SystemComponentBuilder::new("my-cache", SystemComponentType::Cache)
        .on_initialize(|| Box::pin(async { Ok(()) }))
        .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
        .on_shutdown(|| Box::pin(async { Ok(()) }))
        .build()?;

    println!("Component '{}' ready", component.name());
    Ok(())
}
```

## Feature Flags

| Flag | Description |
|------|-------------|
| `analytics` | Developer Portal analytics reporting |
| `llm` | Ollama LLM component types (requires agent crate) |

Most shared types (SystemComponent, bridge, streaming, manifest, etc.) are available without any feature flags since they come from `lifesavor-agent-types`.

## Examples

- [`examples/tts_component/`](examples/tts_component/) — Minimal TTS component with `StreamingEnvelope` audio output
- [`examples/cache_component/`](examples/cache_component/) — Cache component with get/set/delete operations
- [`examples/bridge_consumer/`](examples/bridge_consumer/) — Sandboxed skill accessing system components via `SystemComponentBridge`
- [`examples/sandbox_compliance/`](examples/sandbox_compliance/) — Sandbox constraint declaration and enforcement

## Documentation

- [Getting Started](docs/GETTING_STARTED.md) — Build a minimal component from scratch
- [Deployment Guide](docs/DEPLOYMENT.md) — Compile, deploy, and verify your component
- [Compatibility](COMPATIBILITY.md) — SDK ↔ agent version mapping
- [Changelog](CHANGELOG.md) — Release history

## Architecture

This SDK is a thin re-export layer over the `lifesavor-agent-types` crate. Types like `ProviderManifest`, `ErrorChain`, and `StreamingEnvelope` are the identical Rust types used by the agent — no duplication, no drift. Component crates depend only on this SDK, not on the agent runtime.

```
your-component → lifesavor-system-sdk → lifesavor-agent-types
```

See the [SDK architecture docs](../../docs/ARCHITECTURE.md) for the full dependency graph.

## License

[MIT](LICENSE)
