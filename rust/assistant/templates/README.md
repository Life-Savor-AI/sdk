# my-assistant-provider

<!-- Badge placeholders: replace "my-assistant-provider" with your component slug -->
![Build Status](https://developer.stage.lifesavor.ai/badges/my-assistant-provider/build.svg)
![Version](https://developer.stage.lifesavor.ai/badges/my-assistant-provider/version.svg)
![Installs](https://developer.stage.lifesavor.ai/badges/my-assistant-provider/installs.svg)

A Life Savor assistant provider built with the `lifesavor-assistant-sdk`.

## Overview

Describe what your assistant provider does here.

## Prerequisites

- Rust stable toolchain
- `lifesavor-assistant-sdk` dependency

## Building

```sh
cargo build --release
```

## Testing

```sh
cargo test
```

## Deployment

Place your `component-manifest.toml` and compiled binary in the agent's `config_dir/providers/` directory. See the [Assistant SDK deployment guide](../docs/DEPLOYMENT.md) for details.
