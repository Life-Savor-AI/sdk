# Architecture

## Platform Overview

The Life Savor Developer Platform consists of:

- **Developer Portal** (`developer.lifesavor.ai`) — React/Vite frontend for component management
- **Developer API** (`api.developer.lifesavor.ai`) — Express.js backend service
- **Build Infrastructure** — CodeBuild projects for Linux, macOS, Windows, and GPU builds
- **Marketplace** — Component discovery and installation
- **CLI** (`lsai-cli`) — Rust command-line tool for developer workflows

## Component Lifecycle

```
Draft → Pending Build → Pending QA → Approved → Published
         ↓                ↓
     Build Failed      Rejected → Draft
```

## Build Pipeline

1. Code pushed to GitHub (or manual trigger via CLI/portal)
2. Build config (`lifesavor-build.yml`) validated against schema
3. Build secrets injected as environment variables
4. Source code cloned (SSH deploy key for system components)
5. Build executed in CodeBuild (language-specific container)
6. Security scanning (Semgrep SAST + dependency audit)
7. Artifact uploaded to S3 with SHA-256 checksum
8. Build status reported back to developer portal

## Data Flow

- Component metadata stored in `lifesavor_developer` PostgreSQL database
- Build artifacts stored in S3 with CloudFront CDN delivery
- Build logs stored in S3 at `builds/{BUILD_ID}/build.log`
- Analytics events anonymized (SHA-256 hashed user IDs)
- Webhook deliveries signed with HMAC-SHA256
