# Getting Started with the Assistant SDK

This guide walks you through building a minimal assistant provider, deploying it to a local agent, and testing it.

## 1. Create a New Project

```bash
cargo new my-assistant-provider --lib
cd my-assistant-provider
```

Add the SDK dependency to `Cargo.toml`:

```toml
[dependencies]
lifesavor-assistant-sdk = { path = "../SDK/assistant" }
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
tracing = "0.1"
serde_json = "1.0"
```

## 2. Define an Assistant

Create an assistant definition using the builder:

```rust
use lifesavor_assistant_sdk::prelude::*;
use lifesavor_assistant_sdk::builder::AssistantDefinitionBuilder;

fn create_assistant() -> lifesavor_assistant_sdk::error::Result<AssistantDefinition> {
    AssistantDefinitionBuilder::new()
        .id("support-bot")
        .display_name("Support Bot")
        .system_prompt_template("You are a {{domain}} support assistant.")
        .variable("domain", "customer")
        .build()
}
```

## 3. Build a Provider

```rust
use lifesavor_assistant_sdk::builder::AssistantProviderBuilder;

pub fn create_provider(manifest: ProviderManifest) -> lifesavor_assistant_sdk::error::Result<impl AssistantProvider> {
    let provider = AssistantProviderBuilder::new(manifest)?.build();
    Ok(provider)
}
```

The scaffold provides stub implementations for `load`, `list`, and `resolve`. Replace them with your logic for loading definitions from your data source.

## 4. Write Tests

Use the `MockAssistantStore` from the testing module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use lifesavor_assistant_sdk::testing::MockAssistantStore;

    #[tokio::test]
    async fn test_definition_storage() {
        let def = create_assistant().unwrap();
        let mut store = MockAssistantStore::new();
        store.add(def);

        let loaded = store.load("support-bot").unwrap();
        assert_eq!(loaded.display_name, "Support Bot");

        let all = store.list();
        assert_eq!(all.len(), 1);
    }
}
```

## 5. Deploy to a Local Agent

Create a provider manifest (`manifest.toml`):

```toml
provider_type = "assistant"
instance_name = "my-assistants"
sdk_version = "0.1.0"

[connection]
endpoint = "./assistants/"

[auth]
strategy = "none"

[health_check]
method = "process_alive"
interval_seconds = 60
timeout_seconds = 5
```

Copy the manifest to the agent's provider directory:

```bash
cp manifest.toml ~/.lifesavor/config/providers/my-assistants.toml
```

## 6. Run in Sandbox

Child-process-based assistant providers are sandboxed. Declare your requirements:

```toml
[sandbox]
allowed_env_vars = []
allowed_paths = ["./assistants/"]
max_output_bytes = 1048576
```

Test sandbox compliance locally:

```bash
lifesavor-dev sandbox-test --manifest manifest.toml
```

## Next Steps

- See [examples/](../examples/) for complete working examples
- Read the [Deployment Guide](DEPLOYMENT.md) for production deployment details
- Check [COMPATIBILITY.md](../COMPATIBILITY.md) for version compatibility
