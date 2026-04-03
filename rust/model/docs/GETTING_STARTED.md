# Getting Started with the Model SDK

This guide walks you through building a minimal LLM provider, deploying it to a local agent, and testing it.

## 1. Create a New Project

```bash
cargo new my-llm-provider --lib
cd my-llm-provider
```

Add the SDK dependency to `Cargo.toml`:

```toml
[dependencies]
lifesavor-model-sdk = { path = "../SDK/model" }
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
tracing = "0.1"
serde_json = "1.0"
```

## 2. Define Your Provider Manifest

Create `manifest.toml`:

```toml
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
```

## 3. Build a Provider Scaffold

```rust
use lifesavor_model_sdk::prelude::*;
use lifesavor_model_sdk::builder::ModelProviderBuilder;
use lifesavor_model_sdk::error::Result;

pub fn create_provider(manifest: ProviderManifest) -> Result<impl LlmProvider> {
    let provider = ModelProviderBuilder::new(manifest)?.build();
    Ok(provider)
}
```

The scaffold provides stub implementations for all `LlmProvider` methods. Replace them incrementally:

- `chat_completion_stream` — Stream token responses
- `list_models` — Return available models
- `model_load_status` — Report hot/cold/loading state
- `generate_embedding` — Produce vector embeddings
- `capability_descriptor` — Declare provider capabilities

## 4. Write Tests

Use the `MockRegistry` from the testing module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use lifesavor_model_sdk::testing::MockRegistry;

    #[tokio::test]
    async fn test_provider_registration() {
        let manifest = parse_manifest(include_str!("../manifest.toml")).unwrap();
        let provider = create_provider(manifest.clone()).unwrap();

        let mut registry = MockRegistry::new();
        registry.register(manifest, provider);

        assert!(registry.has_provider("my-ollama"));
    }
}
```

## 5. Deploy to a Local Agent

Copy the manifest to the agent's provider directory:

```bash
cp manifest.toml ~/.lifesavor/config/providers/my-ollama.toml
```

Build your provider binary:

```bash
cargo build --release
```

The agent detects new manifests via hot-reload and starts the provider process.

## 6. Run in Sandbox

Child-process-based providers are sandboxed. Declare your requirements in the manifest:

```toml
[sandbox]
allowed_env_vars = ["OLLAMA_HOST"]
allowed_paths = ["/tmp/ollama-cache"]
max_output_bytes = 10485760
```

Test sandbox compliance locally with the Developer CLI:

```bash
lifesavor-dev sandbox-test --manifest manifest.toml
```

## Next Steps

- See [examples/](../examples/) for complete working examples
- Read the [Deployment Guide](DEPLOYMENT.md) for production deployment details
- Check [COMPATIBILITY.md](../COMPATIBILITY.md) for version compatibility
