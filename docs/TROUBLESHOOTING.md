# Troubleshooting

## Common Issues

### Build Fails with "Config Invalid"

Ensure your `lifesavor-build.yml` matches the schema. Validate locally:

```bash
lsai-cli config validate
```

### Authentication Expired

```bash
lsai-cli auth status   # Check auth state
lsai-cli auth login    # Re-authenticate
```

### Build Timeout (30 minutes)

Builds are automatically terminated after 30 minutes. Optimize your build:

- Use build caching
- Reduce dependencies
- Use pre-built base images

### Security Scan Failures

Check the security scan report in the build details. Fix critical/high findings before re-triggering.

### Rate Limited (429)

Wait for the `Retry-After` period. Check your rate limit dashboard in the portal.

### Deploy Key Issues

- Verify the key is added to your GitHub repository
- Ensure SSH format URL (`git@github.com:org/repo.git`)
- Check key permissions (read access required)

## Diagnostics

Run the built-in diagnostics command:

```bash
lsai-cli diagnostics
lsai-cli diagnostics --json  # Machine-readable output
```

## Getting Help

- [Developer Portal Support](https://developer.lifesavor.ai/support)
- [Documentation](https://developer.lifesavor.ai/documentation)
