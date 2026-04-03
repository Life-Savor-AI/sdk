# lifesavor-assistant-sdk

Build assistant providers for the Life Savor agent using the `AssistantProvider` trait.

Assistant providers manage collections of assistant definitions — structured configurations that combine system prompts, tool bindings, guardrail rules, model preferences, and handoff logic. The SDK provides builders for both provider scaffolds and individual assistant definitions.

## Target Trait

[`AssistantProvider`](https://docs.rs/lifesavor-agent/latest/lifesavor_agent/providers/assistant_provider/trait.AssistantProvider.html) — defines `load`, `list`, and `resolve` methods for assistant definition management.

## Prerequisites

- Rust toolchain **1.75+** (edition 2021)
- Access to the `lifesavor-agent` crate (path dependency or published version)
- Familiarity with `async-trait` and `tokio`

## Quickstart

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
lifesavor-assistant-sdk = { path = "../SDK/assistant" }
tokio = { version = "1", features = ["full"] }
```

Build a minimal assistant definition:

```rust
use lifesavor_assistant_sdk::prelude::*;
use lifesavor_assistant_sdk::builder::AssistantDefinitionBuilder;

fn main() -> lifesavor_assistant_sdk::error::Result<()> {
    let definition = AssistantDefinitionBuilder::new()
        .id("my-assistant")
        .display_name("My Assistant")
        .system_prompt_template("You are a helpful assistant for {{domain}}.")
        .variable("domain", "customer support")
        .build()?;

    println!("Assistant '{}' created", definition.display_name);
    Ok(())
}
```

## Feature Flags

| Flag | Description |
|------|-------------|
| `analytics` | Developer Portal analytics reporting |

All features are disabled by default.

## Examples

- [`examples/local_fs_provider/`](examples/local_fs_provider/) — Load assistant definitions from a local directory
- [`examples/assistant_definition/`](examples/assistant_definition/) — Sample definitions in JSON and TOML formats
- [`examples/validation/`](examples/validation/) — Validate definitions with `validate_definition`
- [`examples/sandbox_compliance/`](examples/sandbox_compliance/) — Sandbox constraint demonstration

## Documentation

- [Getting Started](docs/GETTING_STARTED.md) — Build a minimal provider from scratch
- [Deployment Guide](docs/DEPLOYMENT.md) — Compile, deploy, and verify your provider
- [Compatibility](COMPATIBILITY.md) — SDK ↔ agent version mapping
- [Changelog](CHANGELOG.md) — Release history

## Architecture

This SDK is a thin re-export layer over the `lifesavor-agent` crate. Types like `ProviderManifest`, `ErrorChain`, and `AssistantDefinition` are the identical Rust types from the agent — no duplication, no drift.

See the [pluggable integration architecture spec](../../.kiro/specs/agent-pluggable-integrations/) for detailed design context.

## License

[MIT](LICENSE)
