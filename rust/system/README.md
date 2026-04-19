# lifesavor-system-sdk

Build first-party system components for the Life Savor agent using the `SystemComponent` trait.

System components are privileged, in-process modules that provide core agent capabilities such as TTS/Voice (speech-to-text and text-to-speech), file storage, messaging, caching, and device control. Unlike third-party providers, system components run without sandbox restrictions and have direct access to agent internals.

Voice/TTS model components (Whisper, XTTS-v2, Bark, StyleTTS2) implement `SystemComponent` rather than `LlmProvider` since they process audio rather than text. See the [Voice/TTS Development](#voicetts-component-development) section below.

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

Build a minimal system component with config closures:

```rust
use lifesavor_system_sdk::prelude::*;
use lifesavor_system_sdk::builder::SystemComponentBuilder;
use serde_json::json;
use std::sync::{Arc, RwLock};

#[tokio::main]
async fn main() -> lifesavor_system_sdk::error::Result<()> {
    let config = Arc::new(RwLock::new(json!({"max_items": 1000})));

    let component = SystemComponentBuilder::new("my-cache", SystemComponentType::Cache)
        .on_initialize(|| Box::pin(async { Ok(()) }))
        .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
        .on_shutdown(|| Box::pin(async { Ok(()) }))
        .on_config_schema(|| Some(json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "max_items": { "type": "integer", "minimum": 1 }
            }
        })))
        .on_current_config({
            let cfg = config.clone();
            move || Some(cfg.read().unwrap().clone())
        })
        .on_apply_config({
            let cfg = config.clone();
            move |value| {
                *cfg.write().unwrap() = value;
                Ok(())
            }
        })
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
- [`examples/external_component/`](examples/external_component.rs) — External component connecting via JSON-RPC (auth, register, handle bridge requests)
- [`examples/memory_store_component/`](examples/memory_store_component.rs) — MemoryStore with store/search/delete and `tool_schemas()`/`declaration()`

## Documentation

- [Getting Started](docs/GETTING_STARTED.md) — Build a minimal component from scratch
- [Component Checklist](docs/COMPONENT_CHECKLIST.md) — Full checklist of required artifacts for production components
- [Deployment Guide](docs/DEPLOYMENT.md) — Compile, deploy, and verify your component
- [Compatibility](COMPATIBILITY.md) — SDK ↔ agent version mapping
- [Changelog](CHANGELOG.md) — Release history

## Project Structure

A production system component follows this file layout:

```
my-component/
├── Cargo.toml              # Crate metadata + lifesavor-system-sdk dependency
├── marketplace.toml        # Marketplace listing metadata
├── permissions.toml        # Access control declarations
├── README.md               # Architecture, operations, config, error codes
├── src/
│   ├── lib.rs              # Module declarations and re-exports
│   ├── component.rs        # SystemComponentBuilder lifecycle wiring
│   ├── bridge.rs           # Bridge request handler + standard ops
│   ├── mcp.rs              # MCP tool definitions for tool registry
│   ├── config.rs           # config_schema / current_config / apply_config
│   ├── health.rs           # Real provider probe + ComponentHealthReporter
│   ├── logging.rs          # Structured logging with credential masking
│   ├── rate_limiter.rs     # Token bucket rate limiter (if applicable)
│   └── test_support.rs     # Mock provider (#[cfg(test)])
└── tests/
    └── properties.rs       # Property-based tests (proptest)
```

Use the [scaffold template](templates/component/) and its `generate.sh` script to create a new component with all files pre-populated. See the [Component Checklist](docs/COMPONENT_CHECKLIST.md) for details on each artifact.

## Architecture

This SDK is a thin re-export layer over the `lifesavor-agent-types` crate. Types like `ProviderManifest`, `ErrorChain`, and `StreamingEnvelope` are the identical Rust types used by the agent — no duplication, no drift. Component crates depend only on this SDK, not on the agent runtime.

Components can be **compiled-in** (linked into the agent binary) or **external** (any language, connected via JSON-RPC 2.0). Both types implement the same `SystemComponent` interface and participate in the same bridge dispatch, health checks, and tool schema registry.

```
Compiled-in:  your-component → lifesavor-system-sdk → lifesavor-agent-types
External:     your-component → JSON-RPC → agent → ExternalComponentProxy → registry
```

The agent supports **multi-instance** components — multiple instances of the same `SystemComponentType` can coexist, each with a unique instance ID (e.g., `memory_store:sqlite-vec-rag`, `memory_store:mempalace-personal`).

---

## Voice/TTS Component Development

Voice and TTS model components use the System SDK because they process audio rather than text. There are four Voice/TTS components in the platform:

| Component | Type | Operation | Description |
|-----------|------|-----------|-------------|
| `whisper-large-v3` | ASR | `transcribe` | Audio → text transcription |
| `xtts-v2` | TTS | `synthesize` | Text → speech synthesis |
| `bark` | TTS | `synthesize` | Text → speech synthesis |
| `styletts2` | TTS | `synthesize` | Text → speech synthesis |

### SystemComponent Trait for Voice

Voice components implement the `SystemComponent` trait:

```rust
use lifesavor_system_sdk::prelude::*;
use async_trait::async_trait;

pub struct MyVoiceComponent {
    config: VoiceConfig,
}

#[async_trait]
impl SystemComponent for MyVoiceComponent {
    fn name(&self) -> &str { "my-voice-model" }

    fn component_type(&self) -> SystemComponentType {
        SystemComponentType::Voice
    }

    async fn initialize(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Load the audio model, initialize inference backend
        Ok(())
    }

    async fn health_check(&self) -> ComponentHealthStatus {
        ComponentHealthStatus::Healthy
    }

    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Release model resources
        Ok(())
    }
}
```

### Bridge Operations

Voice components expose audio-specific bridge operations instead of LLM operations:

- **`transcribe`** (ASR) — accepts audio input (WAV/MP3/FLAC), returns transcribed text
- **`synthesize`** (TTS) — accepts text input, returns audio data

### MCP Tool Registration

Each Voice component registers MCP tools prefixed with its model ID:

```rust
// ASR: whisper-large-v3.transcribe
// TTS: xtts-v2.synthesize, bark.synthesize, styletts2.synthesize
```

### Audio Streaming

Voice components use `StreamingEnvelope` with `content_type: "audio"` for WebSocket streaming of audio output. This integrates with the agent's multimodal streaming support.

### Marketplace Metadata

Voice components use `category = "Voice"` in `marketplace.toml`:

```toml
[component]
id = "whisper-large-v3"
category = "Voice"

[model]
specialization = "asr"  # or "tts"
```

### Provider Manifest

```toml
provider_type = "system"
instance_name = "whisper-large-v3"
sdk_version = "0.5.0"
locality = "local"

[connection]
endpoint = "native://whisper-large-v3"

[auth]
strategy = "none"

[health_check]
method = "process_alive"
interval_seconds = 30
timeout_seconds = 5
```

### Key Differences from LLM Components

| Aspect | LLM Component | Voice/TTS Component |
|--------|--------------|-------------------|
| SDK | `lifesavor-model-sdk` | `lifesavor-system-sdk` |
| Trait | `LlmProvider` | `SystemComponent` |
| Operations | `chat`, `generate`, `embeddings` | `transcribe`, `synthesize` |
| Input/Output | Text ↔ Text | Audio ↔ Text |
| Category | `"Llm"` | `"Voice"` |
| Provider Type | `"llm"` | `"system"` |

See the [Model SDK Provider Patterns](../model/docs/PROVIDER_PATTERNS.md) guide for the full TTS/Voice pattern documentation with code examples.

## License

[MIT](LICENSE)
