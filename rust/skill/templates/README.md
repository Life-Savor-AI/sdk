# my-skill

<!-- Badge placeholders: replace "my-skill" with your component slug -->
![Build Status](https://developer.stage.lifesavor.ai/badges/my-skill/build.svg)
![Version](https://developer.stage.lifesavor.ai/badges/my-skill/version.svg)
![Installs](https://developer.stage.lifesavor.ai/badges/my-skill/installs.svg)

A Life Savor skill provider built with the `lifesavor-skill-sdk`.

## Overview

Describe what your skill does here.

## Prerequisites

- Rust stable toolchain
- `lifesavor-skill-sdk` dependency

## Building

```sh
cargo build --release
```

## Testing

```sh
cargo test
```

## Local Sandbox Testing

Use the SandboxRunner to verify sandbox compliance locally:

```sh
cargo run --bin sandbox-runner -- --manifest my-skill.toml --payload test-input.json
```

## Deployment

Place your `component-manifest.toml` and compiled binary in the agent's `config_dir/providers/` directory. See the [Skill SDK deployment guide](../docs/DEPLOYMENT.md) for details.
