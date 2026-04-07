# Life Savor Developer SDK

The Life Savor Developer SDK provides tools and libraries for building components (Models, Assistants, Skills, and System components) for the Life Savor platform.

## Quick Links

- [Getting Started](./GETTING_STARTED.md) — Set up your development environment
- [Architecture](./ARCHITECTURE.md) — Platform architecture overview
- [Build Configuration](./BUILD_CONFIG.md) — `lifesavor-build.yml` reference
- [Deploy Keys](./DEPLOY_KEYS.md) — SSH deploy key setup for system components
- [Security Scanning](./SECURITY_SCANNING.md) — Build security scanning details
- [Troubleshooting](./TROUBLESHOOTING.md) — Common issues and solutions
- [Migration Guide](./MIGRATION.md) — Migrating from previous SDK versions

## Installation

### CLI Tool

```bash
# macOS
brew install lifesavor/tap/lsai-cli

# Linux (Debian/Ubuntu)
curl -fsSL https://developer.lifesavor.ai/install.sh | sh

# From source
cargo install --git https://github.com/lifesavorai/lsai-cli.git
```

### Rust SDK

```toml
[dependencies]
lifesavor-sdk = { git = "https://github.com/lifesavorai/sdk-rust.git" }
```

## Authentication

```bash
lsai-cli auth login    # Opens browser for OAuth
lsai-cli auth status   # Check current auth state
```

## Component Types

| Type      | Description                      |
| --------- | -------------------------------- |
| Model     | AI/ML models for inference       |
| Assistant | Conversational AI assistants     |
| Skill     | Reusable capabilities and tools  |
| System    | Platform-level system components |

## Support

- [Developer Portal](https://developer.lifesavor.ai)
- [API Reference](https://developer.lifesavor.ai/documentation)
- [Support Cases](https://developer.lifesavor.ai/support)
