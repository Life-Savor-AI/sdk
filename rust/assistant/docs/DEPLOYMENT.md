# Deployment Guide — Assistant SDK

## Compile Binary

Build your assistant provider as a release binary:

```bash
cargo build --release
```

The output binary is at `target/release/<your-provider>`.

## Place Manifest

Create a provider manifest and place it in the agent's config directory:

```bash
cp manifest.toml ~/.lifesavor/config/providers/<provider-name>.toml
```

The manifest must include:

```toml
provider_type = "assistant"
instance_name = "<provider-name>"
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

## Hot-Reload Detection

The agent watches `~/.lifesavor/config/providers/` for file changes. When a manifest is added, modified, or removed:

- **Added**: The agent registers and starts the provider
- **Modified**: The agent restarts the provider with the new configuration
- **Removed**: The agent deregisters and stops the provider

No agent restart is required.

## Verify via Component Inventory

After deployment, verify your provider is registered:

```bash
lifesavor-dev component inventory
```

Or query the agent's API:

```
GET /api/v1/providers
```

Your provider should appear with status `Healthy`.

## Health Checks

The agent runs health checks at the interval specified in your manifest. Monitor health status:

```bash
lifesavor-dev component health <provider-name>
```

Health check methods:
- `process_alive` — Checks the provider process is running
- `http_get` — Probes an HTTP endpoint
- `connection_test` — Tests service connectivity

If a health check exceeds the configured timeout, it returns a failure status rather than blocking.

## Metrics and Error Chain

Assistant providers contribute to the agent's error chain using `Subsystem::Provider`:

```rust
use lifesavor_assistant_sdk::prelude::*;

let ctx = AssistantSdkError::NotFound("bot-123".into()).into_error_context();
// ctx.subsystem == Subsystem::Provider
```

## Credential Configuration

If your provider needs credentials, declare vault keys in the manifest:

```toml
vault_keys = ["assistant-api-key"]

[auth]
strategy = "vault"
vault_key = "assistant-api-key"
```

Supported credential sources:
- Vault key reference
- Environment variable
- AWS Secrets Manager ARN
- File path

The `CredentialManager` enforces the vault key allowlist — only keys declared in `vault_keys` can be resolved.
