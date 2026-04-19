# Deployment Guide — Component Lifecycle

This guide covers the full lifecycle of a model component: manifest creation, registry registration, health monitoring, and inference routing.

## Lifecycle Overview

```
1. Create Manifest  →  2. Install Component  →  3. Registry Discovery
       ↓                                               ↓
4. Health Monitoring  ←──────────────────────  5. Status: Active
       ↓
6. Inference Routing  →  7. Metrics Collection
```

## 1. Create the Provider Manifest

The provider manifest (`provider-manifest.toml`) declares your component's identity, connection details, authentication, health check configuration, and capabilities.

```toml
provider_type = "llm"
instance_name = "my-model"
sdk_version = "0.5.0"
priority = 50
locality = "local"                    # "local" or "remote"
depends_on = []
vault_keys = []

[connection]
endpoint = "native://my-model"        # or gateway URL for API providers

[auth]
strategy = "none"                     # "none", "vault", "gateway"

[health_check]
method = "process_alive"              # "http_get", "connection_test", "process_alive"
interval_seconds = 30
timeout_seconds = 5

[capabilities]
features = ["text_generation", "chat"]

[cost_limits]
max_daily_cost_usd = 0.0              # 0 for local/BYOK, set for API providers

[sandbox]
allowed_env_vars = []
allowed_paths = []
max_output_bytes = 10485760
```

### Manifest Fields

| Field | Required | Description |
|-------|----------|-------------|
| `provider_type` | Yes | `"llm"` for LlmProvider, `"system"` for SystemComponent |
| `instance_name` | Yes | Unique identifier for this provider instance |
| `sdk_version` | Yes | Must match agent's `AGENT_SDK_VERSION` (currently `"0.5.0"`) |
| `priority` | Yes | Higher = preferred when multiple providers serve the same model |
| `locality` | Yes | `"local"` for NativeRuntime/Voice, `"remote"` for API/BYOK |
| `connection.endpoint` | Yes | `"native://<id>"` for local, gateway URL for API, vendor URL for BYOK |
| `auth.strategy` | Yes | `"none"` (local), `"gateway"` (API), `"vault"` (BYOK) |
| `health_check.method` | Yes | How the agent probes health |
| `vault_keys` | BYOK only | Allowlist of vault keys the component may access |
| `model_aliases` | Optional | Map of alias → physical model name |

## 2. Install the Component

### Local Development

```bash
lifesavor-dev component install ./path/to/my-component
```

This:
1. Copies the component crate to the agent's component directory
2. Deploys the provider manifest to `~/.lifesavor/config/providers/`
3. Triggers the `IntegrationRegistry` to discover the new manifest
4. Skips code signature verification (development mode)

### Marketplace Installation

For production distribution, components are published to the marketplace:

```bash
sarcinator marketplace register ./path/to/my-component --dry-run  # Validate first
sarcinator marketplace register ./path/to/my-component             # Submit
```

The agent's `MarketplaceInstaller` downloads, verifies the code signature (based on `TrustPolicy`), and installs the component.

## 3. Registry Discovery

When a manifest is detected, the `IntegrationRegistry` processes it:

1. **Parse** — reads and validates the TOML manifest
2. **Version check** — verifies `sdk_version` matches `AGENT_SDK_VERSION`
3. **Register** — creates a `ProviderEntry` with status `Pending`
4. **Initialize** — calls the component's initialization logic
5. **Activate** — transitions status to `Active` on success

```rust
pub enum ProviderStatus {
    Pending,        // Manifest loaded, not yet initialized
    Active,         // Healthy and serving requests
    Degraded,       // 1-2 consecutive health check failures
    Unhealthy,      // 3+ consecutive health check failures
    Deregistered,   // Removed from registry
}
```

The registry supports hot-reload: adding, modifying, or removing a manifest file triggers the corresponding registration action without an agent restart.

### Registry Methods

| Method | Description |
|--------|-------------|
| `register(manifest)` | Add a new provider to the registry |
| `deregister(instance_name)` | Remove a provider |
| `query_by_type(ProviderType)` | Find providers by type (Llm, System, etc.) |
| `query_by_capability(feature)` | Find providers that support a specific feature |
| `load_manifests(config_dir)` | Scan directory for manifest files |

## 4. Health Monitoring

The `HealthMonitor` runs periodic health checks at the interval specified in each manifest.

### Health Check Methods

| Method | Use Case | How It Works |
|--------|----------|-------------|
| `http_get` | API/BYOK providers | Probes an HTTP health endpoint |
| `connection_test` | Remote services | Tests TCP connectivity |
| `process_alive` | Local providers | Checks the provider process is running |

### Health State Transitions

```
0 consecutive failures  →  Healthy (Active)
1-2 consecutive failures →  Degraded
3+ consecutive failures  →  Unhealthy
```

Each health check result triggers `record_success()` or `record_failure()`, which returns a `HealthTransition` if the status changed.

### Monitoring Commands

```bash
lifesavor-dev component list                    # All components with status
lifesavor-dev component health my-model         # Detailed health for one component
```

### Health Reporter

Each component implements a `ComponentHealthReporter` that tracks:

- `model_loaded` — whether the model is in memory
- `total_inference_count` — lifetime inference count
- `avg_inference_latency_ms` — rolling average latency

## 5. Inference Routing

The `InferenceBridge` delegates inference requests to `LlmProvider` instances resolved from the `IntegrationRegistry`.

### Selection Strategies

| Strategy | Behavior |
|----------|----------|
| `HighestPriority` | Use the highest-priority `Active` provider |
| `ByName` | Match by model alias or capability descriptor name |

### Routing by Provider Type

The `modelRouter` resolves the selected model's provider type and builds a routing plan:

| Provider Type | Route |
|---------------|-------|
| `lifesavor_hosted` | → service-api gateway → vendor adapter → vendor API |
| `byok_cloud` | → component → vendor API directly (user's key) |
| `user_hosted_agent` | → component → NativeRuntime (local inference) |

### Capabilities Snapshot

When models are loaded or unloaded, the agent rebuilds a `CapabilitiesSnapshot` and broadcasts it over WebSocket. This enables fleet coordination — other agents and the web app can discover available models in real time.

## 6. Metrics Collection

Every completed inference returns `InferenceMetrics`:

```rust
pub struct InferenceMetrics {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub ttft_ms: u64,       // time to first token
    pub duration_ms: u64,   // total inference duration
}
```

The agent's `TokenUsageTracker` records per-model token consumption (input + output) per day in SQLite. For API providers, the gateway also records billing via `fmMeter.computeCost()` + `tokenLedger.appendEvent()`.

`input_tokens + output_tokens` = total billable token count (billing authority via `InferenceCompletedEvent.total_tokens`).

## 7. Verify Deployment

After installation, verify your component is working:

```bash
# Check registration status
lifesavor-dev component list

# Check health
lifesavor-dev component health my-model

# Test inference (if the agent exposes a test endpoint)
curl -X POST http://localhost:8080/api/v1/chat \
  -H "Content-Type: application/json" \
  -d '{"model": "my-model", "messages": [{"role": "user", "content": "Hello"}]}'
```

## Build Artifact Cleanup

After verifying a component, clean build artifacts to manage disk space:

```bash
cargo clean -p lifesavor-my-model
```

This removes compiled artifacts for the specific component without affecting other crates.

## Credential Configuration

### API Providers (lifesavor_hosted)

No credential configuration needed — the gateway retrieves vendor API keys from the vault.

### BYOK Providers

Declare vault keys in the manifest and configure the user's API key in the agent vault:

```toml
vault_keys = ["openai-api-key"]

[auth]
strategy = "vault"
vault_key = "openai-api-key"
```

The `CredentialManager` enforces the vault key allowlist — only keys declared in `vault_keys` can be resolved.

### Supported Credential Sources

| Source | Description |
|--------|-------------|
| `Vault` | Agent vault key reference (recommended for BYOK) |
| `Env` | Environment variable |
| `File` | File path |
| `AwsSecretsManager` | AWS Secrets Manager ARN |
| `None` | No credentials (local models, API gateway) |

## Security

### Code Signing

Marketplace-distributed components are signed. The agent verifies signatures based on `TrustPolicy`:

| Policy | Behavior |
|--------|----------|
| `Strict` | Reject unsigned components |
| `Warn` | Log warning, proceed with unsigned |
| `Disabled` | Skip verification |

Local installs via `lifesavor-dev component install` skip signature verification.

### Security Surface Report

Each component generates a `SecuritySurfaceReport` from its manifest, declaring:
- Network endpoints accessed
- Vault keys required
- File system paths used
- Environment variables read

This report is available for security auditing via `lifesavor-dev component security <component-id>`.
