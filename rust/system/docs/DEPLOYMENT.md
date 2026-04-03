# Deployment Guide — System SDK

## Compile Binary

System components are compiled as part of the agent or as dynamic libraries:

```bash
cargo build --release
```

The output binary is at `target/release/<your-component>`.

## Place Manifest

Create a provider manifest and place it in the agent's config directory:

```bash
cp manifest.toml ~/.lifesavor/config/providers/<component-name>.toml
```

The manifest must include:

```toml
provider_type = "system"
instance_name = "<component-name>"
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

## Hot-Reload Detection

The agent watches `~/.lifesavor/config/providers/` for file changes. When a manifest is added, modified, or removed:

- **Added**: The agent loads and initializes the component
- **Modified**: The agent restarts the component with the new configuration
- **Removed**: The agent shuts down the component gracefully

No agent restart is required.

## Verify via Component Inventory

After deployment, verify your component is registered:

```bash
lifesavor-dev component inventory
```

Or query the agent's API:

```
GET /api/v1/components
```

Your component should appear with status `Healthy`.

## Health Checks

The agent runs health checks at the interval specified in your manifest. Monitor health status:

```bash
lifesavor-dev component health <component-name>
```

Health check methods:
- `process_alive` — Checks the component process is running
- `http_get` — Probes an HTTP endpoint
- `connection_test` — Tests service connectivity

If a health check exceeds the configured timeout, it returns `Unhealthy` rather than blocking.

System components use `ComponentHealthStatus`:
- `Healthy` — Operating normally
- `Degraded` — Partially functional
- `Unhealthy` — Not operational
- `Unknown` — Status cannot be determined

## Metrics and Error Chain

System components contribute to the agent's error chain using `Subsystem::Bridge`:

```rust
use lifesavor_system_sdk::prelude::*;

let ctx = SystemSdkError::InitFailed("timeout".into()).into_error_context();
// ctx.subsystem == Subsystem::Bridge
```

## Credential Configuration

If your component needs credentials, declare vault keys in the manifest:

```toml
vault_keys = ["my-api-key"]

[auth]
strategy = "vault"
vault_key = "my-api-key"
```

The `CredentialManager` enforces the vault key allowlist — only keys declared in `vault_keys` can be resolved.
