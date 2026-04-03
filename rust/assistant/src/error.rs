//! Assistant SDK error types.
//!
//! Provides [`AssistantSdkError`] for domain-specific error handling within
//! assistant provider implementations, plus a convenience [`Result`] type alias.

use crate::{ErrorContext, Subsystem};
use lifesavor_agent::registry::manifest::ManifestValidationError;

/// Domain-specific error type for the Assistant SDK.
///
/// Each variant maps to a failure mode that assistant provider developers may
/// encounter. The enum derives [`thiserror::Error`] for ergonomic `Display`
/// and `Error` implementations, and provides `From` conversions for common
/// external error types.
#[derive(Debug, thiserror::Error)]
pub enum AssistantSdkError {
    /// Requested assistant definition was not found.
    #[error("Assistant not found: {0}")]
    NotFound(String),

    /// Assistant definition validation failed.
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    /// Template rendering or variable substitution error.
    #[error("Template error: {0}")]
    TemplateError(String),

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

impl AssistantSdkError {
    /// Convert this error into an [`ErrorContext`] suitable for inclusion in
    /// the agent's [`ErrorChain`](crate::ErrorChain).
    ///
    /// The subsystem is always [`Subsystem::Provider`] for the Assistant SDK.
    pub fn into_error_context(&self) -> ErrorContext {
        ErrorContext::new(Subsystem::Provider, self.error_code(), self.to_string())
    }

    /// Return a machine-readable error code string for this variant.
    fn error_code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "ASSISTANT_NOT_FOUND",
            Self::ValidationFailed(_) => "ASSISTANT_VALIDATION_FAILED",
            Self::TemplateError(_) => "TEMPLATE_ERROR",
            Self::ManifestValidation(_) => "MANIFEST_VALIDATION_FAILED",
            Self::Io(_) => "IO_ERROR",
            Self::Json(_) => "JSON_ERROR",
            Self::Toml(_) => "TOML_ERROR",
        }
    }
}

/// Convenience result type for the Assistant SDK.
pub type Result<T> = std::result::Result<T, AssistantSdkError>;
