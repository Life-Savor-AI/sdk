# Build Configuration Reference

## `lifesavor-build.yml`

The build configuration file must be placed at the root of your repository.

### Schema

```yaml
version: 1 # Required: schema version
component:
  type: skill # Required: model | assistant | skill | system
  name: my-component # Required: component name
build:
  language: rust # Required: rust | go | python | node | cpp
  command: cargo build --release # Required: build command
  artifact: target/release/my-component # Required: artifact path
  targets: # Optional: multi-platform targets
    - platform: linux
      arch: x86_64
    - platform: macos
      arch: aarch64
security:
  skip_scan: false # Optional: skip security scan (global-admin only)
```

### Supported Languages

| Language | Build Tool | Dependency Audit |
| -------- | ---------- | ---------------- |
| Rust     | cargo      | `cargo audit`    |
| Go       | go build   | `govulncheck`    |
| Python   | pip/poetry | `pip-audit`      |
| Node.js  | npm/yarn   | `npm audit`      |
| C/C++    | make/cmake | (manual)         |

### Artifact Size Limits

| Component Type | Max Size |
| -------------- | -------- |
| System         | 500 MB   |
| Model          | 200 MB   |
| Assistant      | 100 MB   |
| Skill          | 100 MB   |

### Build Secrets

Set build secrets via the CLI or portal. They are injected as environment variables during build:

```bash
lsai-cli secrets set --component-id <id> --key API_KEY --value <value>
```

### Scheduled Builds

Configure scheduled builds in the portal: daily, weekly, or custom cron expression (5-field format).

### Validation

```bash
lsai-cli config validate              # Validate local config
lsai-cli config validate path/to/file # Validate specific file
```
