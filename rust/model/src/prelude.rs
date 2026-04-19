//! Model SDK prelude — import the most commonly used types with a single
//! `use lifesavor_model_sdk::prelude::*;`

// Core trait and types
pub use crate::{
    LlmProvider,
    ChatRequest,
    ChatMessage,
    ToolCall,
    ToolDefinition,
    ModelInfo,
    CapabilityDescriptor,
    ModelCapability,
    ModelLocality,
    PricingTier,
    LatencyClass,
};

// Inference
pub use crate::{
    CancellableInference,
    InferenceError,
    InferenceMetrics,
    ModelLoadStatus,
    TokenEvent,
    content_type,
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

// RAG provider types (Req 42)
pub use crate::{
    RagSearchRequest,
    RagUpsertRequest,
    RagResult,
    RagProviderStatus,
    RagSearchParams,
    RagUpsertParams,
};

// Process sandbox
pub use crate::ProcessSandbox;

// Tracing
pub use tracing::{info, warn, error, debug, trace, instrument};
