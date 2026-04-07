# Deploy Keys

Deploy keys provide SSH access for system component builds to clone private repositories.

## Setup

1. Generate an SSH key pair in the developer portal (Settings → Deploy Keys)
2. Add the public key as a deploy key in your GitHub repository
3. The build pipeline uses the private key to clone your repository

## Security

- Private keys are stored encrypted (AES-256) in AWS Secrets Manager
- Keys are injected into the build environment at runtime
- Keys are never exposed in build logs (redacted)
- Rotate keys periodically via the portal

## Troubleshooting

- Ensure the deploy key has read access to the repository
- Check that the repository URL uses SSH format (`git@github.com:...`)
- Verify the key is not expired or revoked
