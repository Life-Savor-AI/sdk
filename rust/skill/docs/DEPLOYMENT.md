# Deployment Guide — Skill SDK

## Compile Binary

Build your skill as a release binary:

```bash
cargo build --release
```

The output binary is at `target/release/<your-skill>`.

## Place Manifest

Create a provider manifest and place it in the agent's config directory:

```bash
cp manifest.toml ~/.lifesavor/config/providers/<skill-name>.toml
```

The manifest must include:

```toml
provider_type = "skill"
instance_name = "<skill-name>"
sdk_version = "0.1.0"

[connection]
command = "./target/release/<your-skill>"
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

## Hot-Reload Detection

The agent watches `~/.lifesavor/config/providers/` for file changes. When a manifest is added, modified, or removed:

- **Added**: The agent registers and starts the skill process
- **Modified**: The agent restarts the skill with the new configuration
- **Removed**: The agent deregisters and stops the skill

No agent restart is required.

## Verify via Component Inventory

After deployment, verify your skill is registered:

```bash
lifesavor-dev component inventory
```

Or query the agent's API:

```
GET /api/v1/providers
```

Your skill should appear with status `Healthy` and its tools listed.

## Health Checks

The agent runs health checks at the interval specified in your manifest. Monitor health status:

```bash
lifesavor-dev component health <skill-name>
```

Health check methods:
- `process_alive` — Checks the skill process is running (recommended for skills)
- `http_get` — Probes an HTTP endpoint
- `connection_test` — Tests service connectivity

If a health check exceeds the configured timeout, it returns a failure status rather than blocking.

## Metrics and Error Chain

Skill providers contribute to the agent's error chain using `Subsystem::Provider`:

```rust
use lifesavor_skill_sdk::prelude::*;

let ctx = SkillSdkError::ExecutionFailed("tool timed out".into()).into_error_context();
// ctx.subsystem == Subsystem::Provider
```

## Credential Configuration

If your skill needs credentials, declare vault keys in the manifest:

```toml
vault_keys = ["my-api-key"]

[auth]
strategy = "vault"
vault_key = "my-api-key"
```

Supported credential sources:
- Vault key reference
- Environment variable
- AWS Secrets Manager ARN
- File path

The `CredentialManager` enforces the vault key allowlist — only keys declared in `vault_keys` can be resolved.

## Sandbox Configuration

Skills run as sandboxed child processes. The `[sandbox]` section in your manifest declares:

- `allowed_env_vars` — Environment variables the skill can access
- `allowed_paths` — Filesystem paths the skill can read/write
- `max_output_bytes` — Maximum stdout output size

Test sandbox compliance locally before deploying:

```bash
cargo run -p lifesavor-skill-sdk --bin sandbox-runner -- --manifest manifest.toml
```

Violations are reported as:
- `UndeclaredEnvVar` — Accessed an env var not in `allowed_env_vars`
- `DisallowedPath` — Accessed a path not under `allowed_paths`
- `OutputSizeExceeded` — Stdout output exceeded `max_output_bytes`
