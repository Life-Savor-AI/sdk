//! System SDK error types.
//!
//! Provides [`SystemSdkError`] for domain-specific error handling within system
//! component implementations, plus a convenience [`Result`] type alias.

use crate::{ErrorContext, ManifestValidationError, Subsystem};

/// Domain-specific error type for the System SDK.
///
/// Each variant maps to a failure mode that system component developers may
/// encounter. The enum derives [`thiserror::Error`] for ergonomic `Display`
/// and `Error` implementations, and provides `From` conversions for common
/// external error types.
#[derive(Debug, thiserror::Error)]
pub enum SystemSdkError {
    /// Component initialization failed.
    #[error("Component initialization failed: {0}")]
    InitFailed(String),

    /// Health check failed.
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    /// Shutdown failed.
    #[error("Shutdown failed: {0}")]
    ShutdownFailed(String),

    /// Bridge protocol error.
    #[error("Bridge error: {0}")]
    BridgeError(String),

    /// Manifest validation failed.
    #[error("Manifest validation failed: {0}")]
    ManifestValidation(#[from] ManifestValidationError),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// TOML deserialization error.
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
}

impl SystemSdkError {
    /// Convert this error into an [`ErrorContext`] suitable for inclusion in
    /// the agent's [`ErrorChain`](crate::ErrorChain).
    ///
    /// The subsystem is always [`Subsystem::Bridge`] for the System SDK.
    pub fn into_error_context(&self) -> ErrorContext {
        ErrorContext::new(Subsystem::Bridge, self.error_code(), self.to_string())
    }

    /// Return a machine-readable error code string for this variant.
    fn error_code(&self) -> &'static str {
        match self {
            Self::InitFailed(_) => "COMPONENT_INIT_FAILED",
            Self::HealthCheckFailed(_) => "HEALTH_CHECK_FAILED",
            Self::ShutdownFailed(_) => "COMPONENT_SHUTDOWN_FAILED",
            Self::BridgeError(_) => "BRIDGE_ERROR",
            Self::ManifestValidation(_) => "MANIFEST_VALIDATION_FAILED",
            Self::Io(_) => "IO_ERROR",
            Self::Json(_) => "JSON_ERROR",
            Self::Toml(_) => "TOML_ERROR",
        }
    }
}

/// Convenience result type for the System SDK.
pub type Result<T> = std::result::Result<T, SystemSdkError>;
