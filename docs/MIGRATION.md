# Migration Guide

## Migrating to the New Developer API

The developer API has been extracted to a dedicated service at `api.developer.lifesavor.ai`.

### What Changed

- **API Base URL**: `api.lifesavor.ai` → `api.developer.lifesavor.ai`
- **CLI**: Updated to use new base URL with automatic fallback
- **Frontend**: Points directly to new service

### Migration Timeline

1. **Now**: Both old and new URLs work (90-day proxy period)
2. **After 90 days**: Old URLs return HTTP 410 Gone with redirect info

### CLI Update

Update your CLI to the latest version:

```bash
cargo install --git https://github.com/lifesavorai/lsai-cli.git --force
```

The CLI automatically falls back to the old URL if the new one is unreachable.

### API Clients

Update your API base URL:

```javascript
// Before
const API_URL = 'https://api.lifesavor.ai/api/v3/developer';

// After
const API_URL = 'https://api.developer.lifesavor.ai/api/v3/developer';
```

### Webhooks

Webhook signatures continue to use the same HMAC-SHA256 algorithm. No changes needed for webhook consumers.

### Breaking Changes

None. All API endpoints maintain the same paths and request/response formats.

## Migrating System SDK from 0.1.0 to 0.2.0

SDK 0.2.0 inverts the dependency graph. The SDK now depends on `lifesavor-agent-types` instead of `lifesavor-agent`, so component crates no longer compile the entire agent runtime.

### What Changed

- **Dependency**: `lifesavor-agent` removed from SDK and component `Cargo.toml` files. All shared types come through `lifesavor-system-sdk` which depends on `lifesavor-agent-types`.
- **Removed re-exports**: `SystemComponentRegistry`, `SystemComponentBridge`, `BridgeRateLimiter`, `ProcessSandbox` are no longer available from the SDK (they are agent runtime types).
- **Credential types**: `CredentialManager` struct replaced with `CredentialResolver` trait. Components should program against the trait.
- **Error type**: `SystemComponent` trait methods now return `Result<(), Box<dyn std::error::Error + Send + Sync>>` instead of `Result<(), AgentError>`.
- **Feature flags**: `tts`, `stt`, `file-storage`, `messaging`, `calendar`, `device-control`, `cache` flags removed. All shared types are always available.

### Migration Steps

1. Remove `lifesavor-agent` from your component's `Cargo.toml`:

   ```diff
   [dependencies]
   lifesavor-system-sdk = { path = "../../../sdk/rust/system" }
   - lifesavor-agent = { path = "../../../../agents/cross-platform/source", features = ["tts"] }
   ```

2. Replace any `use lifesavor_agent::` imports with `use lifesavor_system_sdk::`:

   ```diff
   - use lifesavor_agent::system_components::{SystemComponent, SystemComponentType};
   + use lifesavor_system_sdk::{SystemComponent, SystemComponentType};
   ```

3. If you referenced `CredentialManager`, switch to `CredentialResolver`:

   ```diff
   - use lifesavor_system_sdk::CredentialManager;
   + use lifesavor_system_sdk::CredentialResolver;
   ```

4. If your error handling used `AgentError`, switch to generic errors:

   ```diff
   - async fn initialize(&mut self) -> Result<(), AgentError> {
   + async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
   ```
