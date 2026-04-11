//! Skill provider interface types.
//!
//! Defines the [`SkillProvider`] async trait and supporting data types
//! (capability descriptors, health status, lifecycle events, execution
//! results, error types). These are the canonical definitions shared
//! between the agent runtime and the skill SDK.
//!
//! Heavy runtime types (`EnforcementContext`, `ProcessSandbox`) stay in
//! the agent crate — only the wire-protocol and trait surface lives here.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::component_declaration::ToolSchema;
use crate::manifest::Locality;

// ---------------------------------------------------------------------------
// SkillExecutionResult
// ---------------------------------------------------------------------------

/// Result of a skill invocation.
#[derive(Debug)]
pub struct SkillExecutionResult {
    pub status: String,
    pub reason_code: Option<String>,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub exit_code: Option<i32>,
    pub stdout_bytes: usize,
}

// ---------------------------------------------------------------------------
// SkillCapabilityDescriptor
// ---------------------------------------------------------------------------

/// Capability descriptor for a skill provider (Req 28.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCapabilityDescriptor {
    /// Available tool schemas.
    pub tools: Vec<ToolSchema>,
    /// Supported input/output formats (e.g., "json", "binary").
    pub supported_formats: Vec<String>,
    /// Execution constraints.
    pub max_timeout_seconds: u64,
    pub max_memory_bytes: Option<u64>,
    /// Provider locality.
    pub locality: Locality,
}

// ---------------------------------------------------------------------------
// McpTransport
// ---------------------------------------------------------------------------

/// MCP transport type for connecting to MCP servers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    /// Communicate via child process stdin/stdout.
    Stdio,
    /// Communicate via HTTP with Server-Sent Events.
    HttpSse,
}

// ---------------------------------------------------------------------------
// ExecutionLifecycleEvent
// ---------------------------------------------------------------------------

/// Execution lifecycle event type (V1.5a protocol, Req 28.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionLifecycleEvent {
    Started,
    Progress,
    Completed,
    Failed,
    Cancelled,
}

// ---------------------------------------------------------------------------
// SkillProviderError
// ---------------------------------------------------------------------------

/// Error type for skill provider operations.
#[derive(Debug)]
pub enum SkillProviderError {
    /// The skill execution failed.
    ExecutionError(String),
    /// A permission check failed.
    PermissionDenied(String),
    /// The skill or operation was not found.
    NotFound(String),
    /// The skill is blocked by the block list.
    Blocked(String),
    /// Data residency violation.
    DataResidencyViolation(String),
    /// Connection or transport error.
    ConnectionError(String),
    /// The operation timed out.
    Timeout(String),
}

impl std::fmt::Display for SkillProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExecutionError(msg) => write!(f, "Execution error: {msg}"),
            Self::PermissionDenied(msg) => write!(f, "Permission denied: {msg}"),
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
            Self::Blocked(msg) => write!(f, "Blocked: {msg}"),
            Self::DataResidencyViolation(msg) => write!(f, "Data residency violation: {msg}"),
            Self::ConnectionError(msg) => write!(f, "Connection error: {msg}"),
            Self::Timeout(msg) => write!(f, "Timeout: {msg}"),
        }
    }
}

impl std::error::Error for SkillProviderError {}

// ---------------------------------------------------------------------------
// HealthStatus
// ---------------------------------------------------------------------------

/// Health status returned by skill provider health checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded { details: String },
    Unhealthy { details: String },
}

// ---------------------------------------------------------------------------
// SkillProvider trait (Req 28.1)
// ---------------------------------------------------------------------------

/// Trait for skill provider implementations.
///
/// All methods are async and object-safe via `async_trait`. Implementations
/// must be `Send + Sync` for use behind `Arc` in the registry.
#[async_trait]
pub trait SkillProvider: Send + Sync {
    /// Invoke a skill operation with the given payload.
    async fn invoke(
        &self,
        operation: &str,
        payload: Value,
    ) -> Result<SkillExecutionResult, SkillProviderError>;

    /// Enumerate available tool schemas.
    async fn list_tools(&self) -> Result<Vec<ToolSchema>, SkillProviderError>;

    /// Check provider health.
    async fn health_check(&self) -> HealthStatus;

    /// Return the capability descriptor for this provider.
    fn capability_descriptor(&self) -> SkillCapabilityDescriptor;
}
