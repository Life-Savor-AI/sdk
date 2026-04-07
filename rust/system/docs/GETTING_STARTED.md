# Getting Started with the System SDK

This guide walks you through building a minimal system component, deploying it to a local agent, and testing it.

## 1. Create a New Project

```bash
cargo new my-component --lib
cd my-component
```

Add the SDK dependency to `Cargo.toml`:

```toml
[dependencies]
lifesavor-system-sdk = { path = "../../../sdk/rust/system" }
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
tracing = "0.1"
```

That's it — no need to depend on `lifesavor-agent` directly. All shared types (traits, enums, structs) come through the SDK.

## 2. Implement Your Component

Create `src/lib.rs`:

```rust
use lifesavor_system_sdk::prelude::*;
use lifesavor_system_sdk::builder::SystemComponentBuilder;
use lifesavor_system_sdk::error::Result;

pub fn build_component() -> Result<Box<dyn SystemComponent>> {
    SystemComponentBuilder::new("my-cache", SystemComponentType::Cache)
        .on_initialize(|| {
            Box::pin(async {
                tracing::info!("Cache component initializing");
                Ok(())
            })
        })
        .on_health_check(|| {
            Box::pin(async {
                ComponentHealthStatus::Healthy
            })
        })
        .on_shutdown(|| {
            Box::pin(async {
                tracing::info!("Cache component shutting down");
                Ok(())
            })
        })
        .build()
}
```

## 3. Write Tests

Use the `MockAgentContext` from the testing module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use lifesavor_system_sdk::testing::MockAgentContext;

    #[tokio::test]
    async fn test_component_lifecycle() {
        let component = build_component().unwrap();
        let mut ctx = MockAgentContext::new();

        ctx.register(component);
        ctx.initialize_all().await.unwrap();

        let health = ctx.health_check_all().await;
        assert!(health.iter().all(|s| matches!(s, ComponentHealthStatus::Healthy)));

        ctx.shutdown_all().await.unwrap();
    }
}
```

## 4. Deploy to a Local Agent

Create a provider manifest (`manifest.toml`):

```toml
provider_type = "system"
instance_name = "my-cache"
sdk_version = "0.1.0"

[connection]
endpoint = "in-process"

[auth]
strategy = "none"

[health_check]
method = "process_alive"
interval_seconds = 30
timeout_seconds = 5
```

Copy the manifest to the agent's provider directory:

```bash
cp manifest.toml ~/.lifesavor/config/providers/my-cache.toml
```

System components run in-process with the agent — they are NOT sandboxed like third-party providers.

## 5. Run in Sandbox (for Testing)

While system components run in-process in production, you can use the `SandboxRunner` from the Skill SDK to test bridge interactions:

```bash
cargo run --bin sandbox-runner -- --manifest manifest.toml
```

Or use the Developer CLI:

```bash
lifesavor-dev test --manifest manifest.toml
```

## Next Steps

- See [examples/](../examples/) for complete working examples
- Read the [Deployment Guide](DEPLOYMENT.md) for production deployment details
- Check [COMPATIBILITY.md](../COMPATIBILITY.md) for version compatibility
