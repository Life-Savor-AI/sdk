# Compatibility — lifesavor-system-sdk

## SDK ↔ Agent Version Mapping

| SDK Version | Agent Version | Notes |
|-------------|---------------|-------|
| 0.1.0 | 0.x.x | Initial release, path dependency on `lifesavor-agent` |

## Rust Toolchain

- Minimum Supported Rust Version (MSRV): **1.75**
- Edition: 2021

## Breaking Changes Policy

- Patch releases (0.1.x) contain only bug fixes
- Minor releases (0.x.0) may add new features but maintain backward compatibility
- Major releases (x.0.0) may contain breaking changes with migration guides
- Deprecated items are marked with `#[deprecated(since, note)]` at least one minor release before removal
