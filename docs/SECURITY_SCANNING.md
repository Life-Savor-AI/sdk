# Security Scanning

All builds undergo two-phase security scanning before artifacts are published.

## Phase 1: Dependency Audit

Language-specific dependency vulnerability scanning:

| Language | Tool        | Command                   |
| -------- | ----------- | ------------------------- |
| Rust     | cargo audit | `cargo audit --json`      |
| Go       | govulncheck | `govulncheck -json ./...` |
| Python   | pip-audit   | `pip-audit --format=json` |
| Node.js  | npm audit   | `npm audit --json`        |

## Phase 2: SAST (Semgrep)

Static Application Security Testing using Semgrep:

```bash
semgrep scan --config p/default --config p/security-audit --json
```

## Severity Levels

| Severity | Build Impact              |
| -------- | ------------------------- |
| Critical | Build fails               |
| High     | Build fails               |
| Medium   | Warning (build continues) |
| Low      | Warning (build continues) |

## Security History Badge

Components with 3+ consecutive clean builds display a security badge on their marketplace listing. The count resets on any critical or high finding.

## Skipping Scans

Only global-admin users can set `skip_security_scan: true` in the build configuration. This is intended for emergency hotfixes only.
