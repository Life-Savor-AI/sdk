//! Credential resolution trait and associated types.
//!
//! This module defines the `CredentialResolver` async trait for resolving
//! credentials from various sources (vault, environment, file, AWS Secrets
//! Manager). Only the trait interface and data types are included here —
//! the concrete `CredentialManager` struct (which depends on `Vault`,
//! `VaultAccessControl`, and `EventEmitter`) stays in the agent crate.

use async_trait::async_trait;

// ---------------------------------------------------------------------------
// ResolvedCredential
// ---------------------------------------------------------------------------

/// A successfully resolved credential value.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedCredential {
    /// The credential value (API key, token, password, etc.).
    pub value: String,
    /// Which source the credential was resolved from (e.g. "vault", "env", "file").
    pub source: String,
}

// ---------------------------------------------------------------------------
// CredentialError
// ---------------------------------------------------------------------------

/// Errors that can occur during credential resolution.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CredentialError {
    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Vault access denied: {0}")]
    VaultAccessDenied(String),
    #[error("Missing configuration field: {0}")]
    MissingField(String),
    #[error("AWS Secrets Manager error: {0}")]
    AwsSmError(String),
}

// ---------------------------------------------------------------------------
// CredentialResolver trait
// ---------------------------------------------------------------------------

/// Trait for resolving credentials from various sources.
///
/// This trait is object-safe — `Box<dyn CredentialResolver>` compiles.
/// The agent provides the concrete implementation (`CredentialManager`);
/// SDK consumers and component crates program against this trait.
#[async_trait]
pub trait CredentialResolver: Send + Sync {
    /// Resolve the credential described by `auth_config` on behalf of
    /// `provider_id`.
    ///
    /// Returns `Ok(None)` when the auth source is `None` (no credentials
    /// required). Returns `Ok(Some(cred))` on success, or a
    /// `CredentialError` on failure.
    async fn resolve(
        &self,
        auth_config: &crate::manifest::AuthConfig,
        provider_id: &str,
    ) -> Result<Option<ResolvedCredential>, CredentialError>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Unit tests -------------------------------------------------------

    #[test]
    fn credential_error_display() {
        let err = CredentialError::EnvVarNotFound("MY_KEY".to_string());
        assert_eq!(err.to_string(), "Environment variable not found: MY_KEY");

        let err = CredentialError::FileNotFound("/path/to/cred".to_string());
        assert_eq!(err.to_string(), "File not found: /path/to/cred");

        let err = CredentialError::VaultAccessDenied("not allowed".to_string());
        assert_eq!(err.to_string(), "Vault access denied: not allowed");

        let err = CredentialError::MissingField("key_name".to_string());
        assert_eq!(err.to_string(), "Missing configuration field: key_name");

        let err = CredentialError::AwsSmError("timeout".to_string());
        assert_eq!(err.to_string(), "AWS Secrets Manager error: timeout");
    }

    #[test]
    fn resolved_credential_equality() {
        let a = ResolvedCredential {
            value: "secret".to_string(),
            source: "vault".to_string(),
        };
        let b = ResolvedCredential {
            value: "secret".to_string(),
            source: "vault".to_string(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn credential_error_clone() {
        let err = CredentialError::EnvVarNotFound("VAR".to_string());
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    /// Verify that `CredentialResolver` is object-safe by constructing
    /// a `Box<dyn CredentialResolver>`.
    #[tokio::test]
    async fn credential_resolver_is_object_safe() {
        struct MockResolver;

        #[async_trait]
        impl CredentialResolver for MockResolver {
            async fn resolve(
                &self,
                _auth_config: &crate::manifest::AuthConfig,
                _provider_id: &str,
            ) -> Result<Option<ResolvedCredential>, CredentialError> {
                Ok(Some(ResolvedCredential {
                    value: "test-value".to_string(),
                    source: "mock".to_string(),
                }))
            }
        }

        // This line proves object safety — it compiles.
        let resolver: Box<dyn CredentialResolver> = Box::new(MockResolver);

        let auth = crate::manifest::AuthConfig {
            source: crate::manifest::CredentialSource::None,
            key_name: None,
            env_var: None,
            secret_arn: None,
            file_path: None,
        };

        let result = resolver.resolve(&auth, "test-provider").await.unwrap();
        assert!(result.is_some());
        let cred = result.unwrap();
        assert_eq!(cred.value, "test-value");
        assert_eq!(cred.source, "mock");
    }
}
