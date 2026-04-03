//! Model SDK error types.
//!
//! Provides [`ModelSdkError`] for domain-specific error handling within LLM
//! provider implementations, plus a convenience [`Result`] type alias.

use crate::{ErrorContext, Subsystem};
use lifesavor_agent::registry::manifest::ManifestValidationError;

/// Domain-specific error type for the Model SDK.
///
/// Each variant maps to a failure mode that LLM provider developers may
/// encounter. The enum derives [`thiserror::Error`] for ergonomic `Display`
/// and `Error` implementations, and provides `From` conversions for common
/// external error types.
#[derive(Debug, thiserror::Error)]
pub enum ModelSdkError {
    /// Provider operation timed out.
    #[error("Provider timeout: {0}")]
    Timeout(String),

    /// Requested model was not found.
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Inference request failed.
    #[error("Inference failed: {0}")]
    InferenceFailed(String),

    /// Streaming error during token delivery.
    #[error("Streaming error: {0}")]
    StreamingError(String),

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

impl ModelSdkError {
    /// Convert this error into an [`ErrorContext`] suitable for inclusion in
    /// the agent's [`ErrorChain`](crate::ErrorChain).
    ///
    /// The subsystem is always [`Subsystem::Provider`] for the Model SDK.
    pub fn into_error_context(&self) -> ErrorContext {
        ErrorContext::new(Subsystem::Provider, self.error_code(), self.to_string())
    }

    /// Return a machine-readable error code string for this variant.
    fn error_code(&self) -> &'static str {
        match self {
            Self::Timeout(_) => "PROVIDER_TIMEOUT",
            Self::ModelNotFound(_) => "MODEL_NOT_FOUND",
            Self::InferenceFailed(_) => "INFERENCE_FAILED",
            Self::StreamingError(_) => "STREAMING_ERROR",
            Self::ManifestValidation(_) => "MANIFEST_VALIDATION_FAILED",
            Self::Io(_) => "IO_ERROR",
            Self::Json(_) => "JSON_ERROR",
            Self::Toml(_) => "TOML_ERROR",
        }
    }
}

/// Convenience result type for the Model SDK.
pub type Result<T> = std::result::Result<T, ModelSdkError>;
