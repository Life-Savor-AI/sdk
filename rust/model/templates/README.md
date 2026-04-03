# my-llm-provider

<!-- Badge placeholders: replace "my-llm-provider" with your component slug -->
![Build Status](https://developer.stage.lifesavor.ai/badges/my-llm-provider/build.svg)
![Version](https://developer.stage.lifesavor.ai/badges/my-llm-provider/version.svg)
![Installs](https://developer.stage.lifesavor.ai/badges/my-llm-provider/installs.svg)

A Life Savor LLM provider built with the `lifesavor-model-sdk`.

## Overview

Describe what your LLM provider does here.

## Prerequisites

- Rust stable toolchain
- `lifesavor-model-sdk` dependency

## Building

```sh
cargo build --release
```

## Testing

```sh
cargo test
```

## Deployment

Place your `component-manifest.toml` and compiled binary in the agent's `config_dir/providers/` directory. See the [Model SDK deployment guide](../docs/DEPLOYMENT.md) for details.
