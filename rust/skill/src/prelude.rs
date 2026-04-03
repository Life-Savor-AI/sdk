// Skill SDK prelude — import the most commonly used types with a single
// `use lifesavor_skill_sdk::prelude::*;`

// Core trait and types
pub use crate::{
    SkillProvider,
    ToolSchema,
    SkillCapabilityDescriptor,
    ExecutionLifecycleEvent,
    SkillProviderError,
    HealthStatus,
    EnforcementContext,
};

// MCP transport (feature-gated)
#[cfg(feature = "mcp")]
pub use crate::McpTransport;

// Bridge protocol
pub use crate::{
    BridgeRequest,
    BridgeResponse,
    BridgeError,
    SystemCallRequest,
    SystemCallResponse,
};

// Error chain
pub use crate::{
    ErrorChain,
    ErrorContext,
    Subsystem,
};

// Manifest
pub use crate::{
    ProviderManifest,
    ProviderType,
    SandboxConfig,
    ManifestValidationError,
    parse_manifest,
    validate_manifest,
};

// Credentials
pub use crate::{
    CredentialManager,
    ResolvedCredential,
    CredentialError,
    AuthConfig,
    CredentialSource,
};

// Process sandbox
pub use crate::ProcessSandbox;

// Tracing
pub use tracing::{info, warn, error, debug, trace, instrument};
