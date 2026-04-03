//! Skill SDK error types.
//!
//! Provides [`SkillSdkError`] for domain-specific error handling within skill
//! provider implementations, plus a convenience [`Result`] type alias.

use crate::{ErrorContext, Subsystem};
use lifesavor_agent::registry::manifest::ManifestValidationError;

/// Domain-specific error type for the Skill SDK.
///
/// Each variant maps to a failure mode that skill provider developers may
/// encounter. The enum derives [`thiserror::Error`] for ergonomic `Display`
/// and `Error` implementations, and provides `From` conversions for common
/// external error types.
#[derive(Debug, thiserror::Error)]
pub enum SkillSdkError {
    /// Skill execution failed.
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Permission denied for the requested operation.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Sandbox constraint violation detected.
    #[error("Sandbox violation: {0}")]
    SandboxViolation(String),

    /// Tool schema validation failed.
    #[error("Tool schema invalid: {0}")]
    ToolSchemaInvalid(String),

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

impl SkillSdkError {
    /// Convert this error into an [`ErrorContext`] suitable for inclusion in
    /// the agent's [`ErrorChain`](crate::ErrorChain).
    ///
    /// The subsystem is always [`Subsystem::Provider`] for the Skill SDK.
    pub fn into_error_context(&self) -> ErrorContext {
        ErrorContext::new(Subsystem::Provider, self.error_code(), self.to_string())
    }

    /// Return a machine-readable error code string for this variant.
    fn error_code(&self) -> &'static str {
        match self {
            Self::ExecutionFailed(_) => "SKILL_EXECUTION_FAILED",
            Self::PermissionDenied(_) => "PERMISSION_DENIED",
            Self::SandboxViolation(_) => "SANDBOX_VIOLATION",
            Self::ToolSchemaInvalid(_) => "TOOL_SCHEMA_INVALID",
            Self::ManifestValidation(_) => "MANIFEST_VALIDATION_FAILED",
            Self::Io(_) => "IO_ERROR",
            Self::Json(_) => "JSON_ERROR",
            Self::Toml(_) => "TOML_ERROR",
        }
    }
}

/// Convenience result type for the Skill SDK.
pub type Result<T> = std::result::Result<T, SkillSdkError>;
