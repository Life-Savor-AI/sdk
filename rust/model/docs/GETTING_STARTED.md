# Getting Started — Model Component Development

This guide walks you through creating a new model component for the Life Savor agent, from scaffolding to local installation.

## Overview

Model components are standalone Rust crates that implement the `LlmProvider` trait (or `SystemComponent` for Voice/TTS). Each component lives in its own private GitHub repository (`lifesavorai/component-model-<component-id>`) and is mounted as a git submodule at `developer/components/models/<component-id>/`.

The agent ships with zero built-in model providers — all model support is delivered through installable marketplace components.

## Prerequisites

- Rust toolchain **1.75+** (edition 2021)
- `lifesavor-model-sdk` v0.5.0 (path or published dependency)
- `lifesavor-system-sdk` v0.5.0 (for bridge, manifest, health, MCP types)
- Familiarity with `async-trait`, `tokio`, and `serde`

## 1. Scaffold a New Component

Create a new crate using the sarcinator CLI:

```bash
sarcinator repo-setup model my-model
```

This creates a private repo `lifesavorai/component-model-my-model` with a standalone `Cargo.toml`, `README.md`, `.gitignore`, and `LICENSE`.

Alternatively, scaffold manually:

```bash
cargo new lifesavor-my-model --lib
cd lifesavor-my-model
```

### Cargo.toml

Use pinned dependency versions consistent with all other components (see `DEPENDENCY_VERSIONS.md`):

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
tracing = "0.1"
reqwest = { version = "0.12", features = ["json", "stream"] }
async-trait = "0.1"
thiserror = "2.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }

[dev-dependencies]
proptest = "1.0"
```

### Module Layout

Every model component follows this standard structure:

```
src/
├── lib.rs              # Module declarations and re-exports
├── config.rs           # Runtime configuration with JSON Schema validation
├── bridge.rs           # Bridge request dispatcher with error codes
├── provider.rs         # LlmProvider trait implementation
├── health.rs           # ComponentHealthReporter implementation
├── mcp.rs              # MCP tool definitions for agent tool registry
├── rate_limiter.rs     # Token bucket rate limiter
├── logging.rs          # Structured tracing log functions
├── security_surface.rs # SecuritySurfaceReport generation
├── api_client.rs       # (API/BYOK only) Vendor-specific HTTP client
├── runtime.rs          # (Local only) NativeRuntime integration
└── native_runtime_adapter.rs  # (Local only) Runtime adapter
```

## 2. Implement `LlmProvider`

The core trait every LLM component must implement:

```rust
use lifesavor_model_sdk::prelude::*;
use async_trait::async_trait;
use tokio::sync::mpsc;

pub struct MyModelProvider {
    config: MyModelConfig,
}

#[async_trait]
impl LlmProvider for MyModelProvider {
    async fn chat_completion_stream(
        &self,
        request: &ChatRequest,
        tx: mpsc::Sender<TokenEvent>,
    ) -> Result<InferenceMetrics, InferenceError> {
        // Stream tokens through `tx`, return metrics when done.
        // The implementation depends on your provider pattern:
        //   - API: POST to service-api gateway, consume SSE stream
        //   - Local: delegate to NativeRuntime
        //   - BYOK: call vendor API directly with user's key
        todo!("Implement streaming inference")
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, InferenceError> {
        Ok(vec![ModelInfo {
            name: "my-model".to_string(),
            // ...
        }])
    }

    async fn model_load_status(&self, model: &str) -> Result<ModelLoadStatus, InferenceError> {
        Ok(ModelLoadStatus::Ready)
    }

    async fn generate_embedding(
        &self,
        text: &str,
        model: &str,
    ) -> Result<Vec<f32>, InferenceError> {
        Err(InferenceError::RequestFailed("Embeddings not supported".into()))
    }

    fn capability_descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            models: vec![ModelCapability {
                name: "my-model".to_string(),
                locality: ModelLocality::Local, // or Remote for API/BYOK
                context_window: 4096,
                pricing_tier: PricingTier::Free,
                latency_class: LatencyClass::Medium,
                load_status: ModelLoadStatus::Ready,
                features: vec!["text_generation".into(), "chat".into()],
            }],
            features: vec!["text_generation".into(), "chat".into()],
            locality: Locality::Local,
        }
    }

    fn resolve_model_alias(&self, alias: &str) -> String {
        alias.to_string()
    }
}
```

## 3. Implement Supporting Modules

### config.rs — Runtime Configuration

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyModelConfig {
    pub model_name: String,
    pub max_tokens: u32,
    pub temperature: f64,
}

impl Default for MyModelConfig {
    fn default() -> Self {
        Self {
            model_name: "my-model".to_string(),
            max_tokens: 2048,
            temperature: 0.7,
        }
    }
}

impl MyModelConfig {
    pub fn config_schema() -> Value {
        serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "model_name": { "type": "string" },
                "max_tokens": { "type": "integer", "minimum": 1 },
                "temperature": { "type": "number", "minimum": 0.0, "maximum": 2.0 }
            }
        })
    }

    pub fn apply_config(&mut self, value: Value) -> Result<(), String> {
        *self = serde_json::from_value(value).map_err(|e| e.to_string())?;
        Ok(())
    }
}
```

### bridge.rs — Request Dispatcher

```rust
pub const MY_MODEL_UNREACHABLE: i32 = -33001;
pub const MY_MODEL_TIMEOUT: i32 = -33002;
pub const UNKNOWN_OPERATION: i32 = -33099;

pub fn dispatch(operation: &str, params: serde_json::Value) -> Result<serde_json::Value, (i32, String)> {
    match operation {
        "chat" | "generate" | "embeddings" | "component_status" => {
            // Route to the appropriate handler
            todo!()
        }
        _ => Err((UNKNOWN_OPERATION, format!("Unknown operation: {}", operation))),
    }
}
```

### health.rs — Health Reporting

```rust
pub struct MyModelHealthReporter {
    pub consecutive_failures: u32,
}

impl MyModelHealthReporter {
    pub fn health_status(&self) -> &str {
        match self.consecutive_failures {
            0 => "Healthy",
            1..=2 => "Degraded",
            _ => "Unhealthy",
        }
    }

    pub fn record_success(&mut self) { self.consecutive_failures = 0; }
    pub fn record_failure(&mut self) { self.consecutive_failures += 1; }
}
```

## 4. Create Metadata Files

Every component requires these metadata files at the crate root:

### marketplace.toml

```toml
[component]
id = "my-model"
name = "My Model"
version = "0.1.0"
category = "Llm"
description = "My custom model component"
author = "lifesavorai"

[model]
family = "custom"
parameter_count = "7B"
context_window = 4096
specialization = "general"

[capabilities]
features = ["text_generation", "chat"]
```

### permissions.toml

```toml
[publish]
topics = ["inference.completed"]

[mcp]
tools = ["my-model.chat", "my-model.generate"]
```

### model-deps.json

```json
{
  "model_source": "huggingface",
  "repo_id": "organization/my-model",
  "files": ["model.safetensors", "tokenizer.json", "config.json"],
  "checksums": {},
  "min_ram_gb": 4
}
```

### component-manifest.toml

```toml
[component]
id = "my-model"
type = "model"
sdk_version = "0.5.0"
```

### Provider Manifest (examples/my-model-provider.toml)

```toml
provider_type = "llm"
instance_name = "my-model"
sdk_version = "0.5.0"
priority = 50
locality = "local"

[connection]
endpoint = "native://my-model"

[auth]
strategy = "none"

[health_check]
method = "process_alive"
interval_seconds = 30
timeout_seconds = 5
```

## 5. Write Tests

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = MyModelConfig::default();
        assert_eq!(config.model_name, "my-model");
        assert_eq!(config.max_tokens, 2048);
    }

    #[test]
    fn test_bridge_unknown_operation() {
        let result = dispatch("nonexistent", serde_json::json!({}));
        assert!(result.is_err());
        let (code, _) = result.unwrap_err();
        assert_eq!(code, UNKNOWN_OPERATION);
    }
}
```

### Property-Based Tests

Create `tests/config_properties.rs`:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn config_round_trip(
        max_tokens in 1u32..10000,
        temperature in 0.0f64..2.0,
    ) {
        let config = MyModelConfig {
            model_name: "my-model".to_string(),
            max_tokens,
            temperature,
        };
        let json = serde_json::to_value(&config).unwrap();
        let restored: MyModelConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.max_tokens, restored.max_tokens);
    }
}
```

## 6. Build and Test

```bash
cargo build
cargo test
cargo clippy -- -D warnings
```

Each component must compile and pass all tests independently from its own repository root.

## 7. Install Locally

Use the developer CLI to install your component on a local agent:

```bash
lifesavor-dev component install ./path/to/lifesavor-my-model
```

This copies the component to the agent's component directory, deploys the manifest, and triggers registry discovery. Local installs skip code signature verification.

Verify installation:

```bash
lifesavor-dev component list
```

Other useful commands:

```bash
lifesavor-dev component reload my-model    # Hot-reload after code changes
lifesavor-dev component uninstall my-model  # Remove the component
```

## Next Steps

- Read [Provider Patterns](PROVIDER_PATTERNS.md) for detailed guidance on each of the four provider patterns
- Read [Deployment](DEPLOYMENT.md) for the full manifest → registry → health → routing lifecycle
- Read [Migration](MIGRATION.md) if you're migrating from the old built-in providers
- Check existing components under `developer/components/models/` for reference implementations
