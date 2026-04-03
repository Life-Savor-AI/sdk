# Getting Started with the Skill SDK

This guide walks you through building a minimal skill, deploying it to a local agent, and testing it in the sandbox.

## 1. Create a New Project

```bash
cargo new my-skill
cd my-skill
```

Add the SDK dependency to `Cargo.toml`:

```toml
[dependencies]
lifesavor-skill-sdk = { path = "../SDK/skill" }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
```

## 2. Define Your Tool Schema

```rust
use lifesavor_skill_sdk::builder::ToolSchemaBuilder;

fn create_tool() -> lifesavor_skill_sdk::error::Result<lifesavor_skill_sdk::prelude::ToolSchema> {
    ToolSchemaBuilder::new()
        .name("translate")
        .description("Translates text between languages")
        .input_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "text": { "type": "string" },
                "target_lang": { "type": "string" }
            },
            "required": ["text", "target_lang"]
        }))
        .build()
}
```

## 3. Build a Skill Provider

```rust
use lifesavor_skill_sdk::prelude::*;
use lifesavor_skill_sdk::builder::SkillProviderBuilder;

fn main() -> lifesavor_skill_sdk::error::Result<()> {
    let tool = create_tool()?;

    let manifest = parse_manifest(include_str!("../manifest.toml"))?;
    let provider = SkillProviderBuilder::new(manifest)?
        .tool(tool)
        .build();

    // The scaffold handles JSON stdin/stdout protocol.
    // Replace the invoke handler with your logic.
    Ok(())
}
```

## 4. Write Tests

Use the `MockSandbox` from the testing module:

```rust
#[cfg(test)]
mod tests {
    use lifesavor_skill_sdk::testing::MockSandbox;
    use lifesavor_skill_sdk::prelude::*;

    #[test]
    fn test_sandbox_compliance() {
        let config = SandboxConfig {
            allowed_env_vars: vec!["API_KEY".into()],
            allowed_paths: vec!["/tmp/skill-data".into()],
            max_output_bytes: Some(1048576),
            ..Default::default()
        };

        let sandbox = MockSandbox::new(config);
        assert!(sandbox.check_env_var("API_KEY"));
        assert!(!sandbox.check_env_var("SECRET_TOKEN"));
    }
}
```

## 5. Deploy to a Local Agent

Create a provider manifest (`manifest.toml`):

```toml
provider_type = "skill"
instance_name = "my-translator"
sdk_version = "0.1.0"

[connection]
command = "./target/release/my-skill"
transport = "json_stdio"

[auth]
strategy = "none"

[health_check]
method = "process_alive"
interval_seconds = 30
timeout_seconds = 5

[sandbox]
allowed_env_vars = ["API_KEY"]
allowed_paths = ["/tmp/skill-data"]
max_output_bytes = 1048576
```

Copy the manifest and build:

```bash
cargo build --release
cp manifest.toml ~/.lifesavor/config/providers/my-translator.toml
```

## 6. Run in Sandbox

Test your skill locally using the `SandboxRunner`:

```bash
cargo run -p lifesavor-skill-sdk --bin sandbox-runner -- --manifest manifest.toml
```

For MCP skills, add the `--mcp` flag:

```bash
cargo run -p lifesavor-skill-sdk --bin sandbox-runner -- --manifest manifest.toml --mcp
```

Or use the Developer CLI:

```bash
lifesavor-dev sandbox-test --manifest manifest.toml
```

The sandbox runner spawns your skill as a child process with the same `ProcessSandbox` restrictions the agent applies, reporting any violations.

## Next Steps

- See [examples/](../examples/) for complete working examples
- Read the [Deployment Guide](DEPLOYMENT.md) for production deployment details
- Check [COMPATIBILITY.md](../COMPATIBILITY.md) for version compatibility
