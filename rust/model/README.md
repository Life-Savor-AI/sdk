# lifesavor-model-sdk

Build LLM provider components for the Life Savor agent using the `LlmProvider` trait.

The agent ships with zero built-in model providers — all model support is delivered through installable marketplace components. Each component is a standalone Rust crate in its own repository, implementing one of four provider patterns.

## Provider Patterns

| Pattern | Use Case | Examples |
|---------|----------|---------|
| **API Gateway** | Commercial models routed through service-api for centralized billing | gpt-4o, claude-3-5-sonnet, gemini-1-5-pro |
| **Local/NativeRuntime** | Open-source models on the embedded PyTorch + ONNX dual-runtime | llama-3-8b, tinyllama-1-1b, phi-3-mini |
| **BYOK** | Commercial models with user-supplied API keys, no platform billing | gpt-4o-byok, claude-3-5-sonnet-byok |
| **TTS/Voice** | Audio models implementing `SystemComponent` instead of `LlmProvider` | whisper-large-v3, xtts-v2, bark |

## Target Trait

[`LlmProvider`](https://docs.rs/lifesavor-agent/latest/lifesavor_agent/providers/llm_provider/trait.LlmProvider.html) — defines `chat_completion_stream`, `list_models`, `model_load_status`, `generate_embedding`, `capability_descriptor`, and `resolve_model_alias`.

## Prerequisites

- Rust toolchain **1.75+** (edition 2021)
- Access to the `lifesavor-agent` crate (path dependency or published version)
- Familiarity with `async-trait` and `tokio`

## Quickstart

Add the dependency to your component's `Cargo.toml`:

```toml
[package]
name = "lifesavor-my-model"
version = "0.1.0"
edition = "2021"

[dependencies]
lifesavor-model-sdk = "0.5.0"
lifesavor-system-sdk = "0.5.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
async-trait = "0.1"

[dev-dependencies]
proptest = "1.0"
```

Implement the `LlmProvider` trait:

```rust
use lifesavor_model_sdk::prelude::*;
use async_trait::async_trait;
use tokio::sync::mpsc;

pub struct MyProvider { /* config, state */ }

#[async_trait]
impl LlmProvider for MyProvider {
    async fn chat_completion_stream(
        &self,
        request: &ChatRequest,
        tx: mpsc::Sender<TokenEvent>,
    ) -> Result<InferenceMetrics, InferenceError> {
        // Stream tokens through tx, return metrics
        todo!()
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, InferenceError> { todo!() }
    async fn model_load_status(&self, model: &str) -> Result<ModelLoadStatus, InferenceError> { todo!() }
    async fn generate_embedding(&self, text: &str, model: &str) -> Result<Vec<f32>, InferenceError> { todo!() }

    fn capability_descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            models: vec![],
            features: vec!["text_generation".into(), "chat".into()],
            locality: Locality::Local,
        }
    }

    fn resolve_model_alias(&self, alias: &str) -> String { alias.to_string() }
}
```

Install locally:

```bash
lifesavor-dev component install ./path/to/my-component
lifesavor-dev component list
```

## Component Structure

Every model component follows this standard layout:

```
my-component/
├── Cargo.toml                  # Standalone crate with pinned dependencies
├── marketplace.toml            # Marketplace listing metadata
├── permissions.toml            # Access control declarations
├── model-deps.json             # Model weights, checksums, hardware requirements
├── component-manifest.toml     # Component identity and SDK version
├── orchestration.yaml          # Post-install hooks
├── lifesavor-build.yml         # CI/CD pipeline with code signing
├── README.md                   # Model docs, capabilities, config, quick-start
├── LICENSE
├── src/
│   ├── lib.rs                  # Module declarations and re-exports
│   ├── config.rs               # Runtime configuration with JSON Schema
│   ├── bridge.rs               # Bridge request dispatcher with error codes
│   ├── provider.rs             # LlmProvider trait implementation
│   ├── health.rs               # ComponentHealthReporter
│   ├── mcp.rs                  # MCP tool definitions
│   ├── rate_limiter.rs         # Token bucket rate limiter
│   ├── logging.rs              # Structured tracing
│   ├── security_surface.rs     # SecuritySurfaceReport generation
│   ├── api_client.rs           # (API/BYOK) Vendor HTTP client
│   ├── runtime.rs              # (Local) NativeRuntime integration
│   └── native_runtime_adapter.rs # (Local) Runtime adapter
├── examples/
│   └── provider-manifest.toml  # Example provider manifest
└── tests/
    ├── config_properties.rs    # Config round-trip property tests
    ├── bridge_properties.rs    # Bridge dispatch property tests
    └── ...                     # Additional property tests
```

## Key Types

| Type | Description |
|------|-------------|
| `LlmProvider` | Core trait for all LLM components |
| `ChatRequest` | Inference request with messages, tools, and options |
| `ChatMessage` | Message with optional `images`, `tool_calls`, `tool_call_id` |
| `ToolCall` | Function call response from tool-use models |
| `ToolDefinition` | Function declaration passed to tool-use models |
| `TokenEvent` | Streaming token with execution ID and index |
| `InferenceMetrics` | Token counts, TTFT, and duration |
| `InferenceError` | Error variants including `AuthenticationFailed`, `RateLimited`, `ProviderUnavailable` |
| `CapabilityDescriptor` | Model capabilities, features, and locality |
| `ProviderManifest` | Component identity, connection, auth, health config |
| `CredentialManager` | Vault-based credential resolution (BYOK) |
| `ModelLoadStatus` | Hot/Warm/Cold/Loading state for local models |

## Feature Flags

| Flag | Description |
|------|-------------|
| `analytics` | Developer Portal analytics reporting |

All features are disabled by default. The core `LlmProvider` trait is always available.

## Examples

- [`examples/native_provider/`](examples/native_provider/) — Local model provider with NativeRuntime streaming
- [`examples/mock_provider/`](examples/mock_provider/) — Mock provider for testing
- [`examples/hot_cold_management/`](examples/hot_cold_management/) — Hot/Warm/Cold model state management
- [`examples/sandbox_compliance/`](examples/sandbox_compliance/) — Sandbox constraint demonstration

## Documentation

- [Getting Started](docs/GETTING_STARTED.md) — Component development workflow: scaffold, implement, test, deploy
- [Provider Patterns](docs/PROVIDER_PATTERNS.md) — Four provider patterns with Rust code examples
- [Deployment Guide](docs/DEPLOYMENT.md) — Manifest → registry → health → routing lifecycle
- [Migration Guide](docs/MIGRATION.md) — Migrating from built-in providers to components
- [Compatibility](COMPATIBILITY.md) — SDK ↔ agent version mapping
- [Changelog](CHANGELOG.md) — Release history

## Architecture

This SDK is a thin re-export layer over the `lifesavor-agent` crate. Types like `ProviderManifest`, `ErrorChain`, and `StreamingEnvelope` are the identical Rust types from the agent — no duplication, no drift.

The 55 model components span four provider patterns across seven categories (Foundation, Open/Self-Hosted, Lightweight/Edge, Coding, Reasoning, Experimental, TTS/Voice). Each component lives in its own private repository (`lifesavorai/component-model-<id>`) and is mounted as a git submodule at `developer/components/models/<id>/`.

See the [pluggable integration architecture spec](../../.kiro/specs/agent-pluggable-integrations/) for detailed design context.

## License

[MIT](LICENSE)
