# Getting Started

## Prerequisites

- Node.js 20+ or Rust 1.75+ (depending on component type)
- Git
- `lsai-cli` installed

## 1. Create a Developer Account

Visit [developer.lifesavor.ai](https://developer.lifesavor.ai) and sign up with Google OAuth or email. Complete the developer agreement and profile setup.

## 2. Install the CLI

```bash
# Install lsai-cli
cargo install --git https://github.com/lifesavorai/lsai-cli.git

# Authenticate
lsai-cli auth login
```

## 3. Create a Component

```bash
# Create a new skill component
lsai-cli components create --name "my-skill" --type skill

# Or create via the web portal
```

## 4. Set Up Build Configuration

Create `lifesavor-build.yml` in your repository root:

```yaml
version: 1
component:
  type: skill
  name: my-skill
build:
  language: rust
  command: cargo build --release
  artifact: target/release/my-skill
```

## 5. Connect GitHub Repository

Link your GitHub repository through the developer portal. Builds trigger automatically on push to the default branch.

## 6. Trigger a Build

```bash
lsai-cli builds trigger --component-id <your-component-id>
```

## 7. Submit for QA Review

Once your build succeeds, submit for QA review through the portal. After approval, publish to the marketplace.

## Local Development

```bash
# Clone your component repo
git clone <your-repo-url>

# Validate build config
lsai-cli config validate

# Run local security scan
semgrep scan --config p/default --config p/security-audit
```
