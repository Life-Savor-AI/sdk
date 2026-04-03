// System SDK prelude — import the most commonly used types with a single
// `use lifesavor_system_sdk::prelude::*;`

// Core trait and types
pub use crate::{
    SystemComponent,
    SystemComponentType,
    ComponentHealthStatus,
    SystemComponentInfo,
    SystemComponentRegistry,
};

// Bridge protocol
pub use crate::{
    SystemComponentBridge,
    BridgeRequest,
    BridgeResponse,
    BridgeError,
    SystemCallRequest,
    SystemCallResponse,
};

// Streaming
pub use crate::{
    StreamingEnvelope,
    StreamStatus,
    StreamMetadata,
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
