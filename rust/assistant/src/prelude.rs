// Assistant SDK prelude — import the most commonly used types with a single
// `use lifesavor_assistant_sdk::prelude::*;`

// Core trait and types
pub use crate::{
    AssistantProvider,
    AssistantDefinition,
    AssistantSummary,
    ResolvedAssistant,
    AssistantProviderError,
    ToolBinding,
    GuardrailRule,
    HandoffConfig,
    validate_definition,
    substitute_variables,
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
