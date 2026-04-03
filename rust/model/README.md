# lifesavor-model-sdk

Build LLM provider integrations for the Life Savor agent using the `LlmProvider` trait.

Model providers connect the agent to language model backends (Ollama, OpenAI-compatible APIs, AWS Bedrock, etc.) and expose capabilities like chat completion streaming, model listing, embedding generation, and hot/cold model management.

## Target Trait

[`LlmProvider`](https://docs.rs/lifesavor-agent/latest/lifesavor_agent/providers/llm_provider/trait.LlmProvider.html) — defines `chat_completion_stream`, `list_models`, `model_load_status`, `generate_embedding`, `capability_descriptor`, and `resolve_model_alias`.

## Prerequisites

- Rust toolchain **1.75+** (edition 2021)
- Access to the `lifesavor-agent` crate (path dependency or published version)
- Familiarity with `async-trait` and `tokio`

## Quickstart

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
lifesavor-model-sdk = { path = "../SDK/model" }
tokio = { version = "1", features = ["full"] }
```

Build a minimal model provider scaffold:

```rust
use lifesavor_model_sdk::prelude::*;
use lifesavor_model_sdk::builder::ModelProviderBuilder;

fn main() -> lifesavor_model_sdk::error::Result<()> {
    let manifest = parse_manifest(r#"
        provider_type = "llm"
        instance_name = "my-ollama"
        sdk_version = "0.1.0"

        [connection]
        endpoint = "http://localhost:11434"

        [auth]
        strategy = "none"

        [health_check]
        method = "http_get"
        interval_seconds = 30
        timeout_seconds = 5
    "#)?;

    let provider = ModelProviderBuilder::new(manifest)?.build();
    println!("Model provider scaffold created");
    Ok(())
}
```

## Feature Flags

| Flag | Description |
|------|-------------|
| `bedrock` | AWS Bedrock provider re-exports |
| `openai` | OpenAI-compatible provider re-exports |
| `analytics` | Developer Portal analytics reporting |

All features are disabled by default. The core `LlmProvider` trait and Ollama types are always available.

## Examples

- [`examples/ollama_provider/`](examples/ollama_provider/) — Minimal Ollama LLM provider with streaming
- [`examples/mock_provider/`](examples/mock_provider/) — Mock provider with canned responses for testing
- [`examples/hot_cold_management/`](examples/hot_cold_management/) — Hot/cold model management with `ModelLoadStatus`
- [`examples/sandbox_compliance/`](examples/sandbox_compliance/) — Sandbox constraint demonstration

## Documentation

- [Getting Started](docs/GETTING_STARTED.md) — Build a minimal provider from scratch
- [Deployment Guide](docs/DEPLOYMENT.md) — Compile, deploy, and verify your provider
- [Compatibility](COMPATIBILITY.md) — SDK ↔ agent version mapping
- [Changelog](CHANGELOG.md) — Release history

## Architecture

This SDK is a thin re-export layer over the `lifesavor-agent` crate. Types like `ProviderManifest`, `ErrorChain`, and `StreamingEnvelope` are the identical Rust types from the agent — no duplication, no drift.

See the [pluggable integration architecture spec](../../.kiro/specs/agent-pluggable-integrations/) for detailed design context.

## License

[MIT](LICENSE)
