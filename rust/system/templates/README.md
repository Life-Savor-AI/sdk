# my-system-component

<!-- Badge placeholders: replace "my-system-component" with your component slug -->
![Build Status](https://developer.stage.lifesavor.ai/badges/my-system-component/build.svg)
![Version](https://developer.stage.lifesavor.ai/badges/my-system-component/version.svg)
![Installs](https://developer.stage.lifesavor.ai/badges/my-system-component/installs.svg)

A Life Savor system component built with the `lifesavor-system-sdk`.

## Overview

Describe what your system component does here.

## Prerequisites

- Rust stable toolchain
- `lifesavor-system-sdk` dependency

## Building

```sh
cargo build --release
```

## Testing

```sh
cargo test
```

## Deployment

Place your `component-manifest.toml` and compiled binary in the agent's `config_dir/providers/` directory. See the [System SDK deployment guide](../docs/DEPLOYMENT.md) for details.
