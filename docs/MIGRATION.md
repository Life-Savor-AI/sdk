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
