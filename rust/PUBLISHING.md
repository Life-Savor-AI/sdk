# Publishing Guide — Life Savor Rust SDK Suite

This guide covers publishing the SDK crates to [crates.io](https://crates.io).

## Prerequisites

- A crates.io account with publish permissions for the `lifesavor-*` crates
- `cargo login` completed with a valid API token
- `cargo-semver-checks` installed: `cargo install cargo-semver-checks`

## Publishing Checklist

### 1. Version Bump

Update the version in each crate's `Cargo.toml` that has changes. Follow [Semantic Versioning](https://semver.org/):

- **Patch** (`0.1.0` → `0.1.1`): Bug fixes, documentation updates
- **Minor** (`0.1.0` → `0.2.0`): New features, new re-exports, new builder methods (backwards-compatible)
- **Major** (`0.1.0` → `1.0.0`): Removed public types, changed trait signatures, breaking builder API changes

### 2. Changelog Update

Update each modified crate's `CHANGELOG.md` and the workspace-level `SDK/CHANGELOG.md`:

- Add a new version section with the release date
- Categorize changes under: Added, Changed, Deprecated, Removed, Fixed, Security
- For major bumps, create a `docs/MIGRATION_X_TO_Y.md` guide in each affected crate

### 3. Semver Compatibility Check

Run `cargo-semver-checks` to detect accidental breaking changes:

```bash
# From SDK/ directory
./scripts/semver-check.sh
```

Or manually per crate:

```bash
cargo semver-checks --manifest-path system/Cargo.toml
cargo semver-checks --manifest-path model/Cargo.toml
cargo semver-checks --manifest-path assistant/Cargo.toml
cargo semver-checks --manifest-path skill/Cargo.toml
```

If breaking changes are detected on a minor/patch bump, either revert the change or bump the major version.

### 4. Dry-Run Validation

Run a dry-run publish for all crates to catch packaging issues before the real publish:

```bash
# From SDK/ directory
./scripts/publish.sh --dry-run
```

Or manually:

```bash
cargo publish --dry-run --manifest-path system/Cargo.toml
cargo publish --dry-run --manifest-path model/Cargo.toml
cargo publish --dry-run --manifest-path assistant/Cargo.toml
cargo publish --dry-run --manifest-path skill/Cargo.toml
```

Verify:

- No missing files (README, LICENSE, templates)
- All dependencies resolve
- Package metadata is complete

### 5. Publish (in dependency order)

The `lifesavor-agent-types` crate must be published first since other SDK crates depend on it:

```bash
# 1. Shared types crate (no dependencies on other lifesavor crates)
cargo publish --manifest-path agent/Cargo.toml

# 2. SDK crates (depend on agent-types, but not on each other)
cargo publish --manifest-path system/Cargo.toml
cargo publish --manifest-path model/Cargo.toml
cargo publish --manifest-path assistant/Cargo.toml
cargo publish --manifest-path skill/Cargo.toml
```

Wait for `lifesavor-agent-types` to appear on crates.io before publishing the SDK crates.

### 6. Post-Publish Verification

After publishing, verify each crate:

1. Check the crate page on crates.io renders correctly (README, metadata, feature flags)
2. In a fresh directory, create a test project depending on the published version:

   ```bash
   cargo init verify-sdk && cd verify-sdk
   cargo add lifesavor-system-sdk
   cargo add lifesavor-model-sdk
   cargo add lifesavor-assistant-sdk
   cargo add lifesavor-skill-sdk
   cargo check
   ```

3. Verify `cargo doc` generates documentation without warnings
4. Tag the release in git: `git tag -a sdk-v0.1.0 -m "SDK Suite v0.1.0"`

## Troubleshooting

| Issue | Solution |
|-------|----------|
| `cargo publish` fails with "crate already exists" | The version was already published. Bump the version and try again. |
| Missing `lifesavor-agent` dependency | For crates.io, `lifesavor-agent-types` uses a version dependency. Publish `agent-types` first, then the SDK crates. |
| Semver check fails unexpectedly | If the change is intentional, bump the major version and add a migration guide. |
| Dry-run passes but publish fails | Check crates.io rate limits. Wait a minute and retry. |
