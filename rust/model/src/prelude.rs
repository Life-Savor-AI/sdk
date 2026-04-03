// Model SDK prelude — import the most commonly used types with a single
// `use lifesavor_model_sdk::prelude::*;`

// Core trait and types
pub use crate::{
    LlmProvider,
    ChatRequest,
    ModelInfo,
    CapabilityDescriptor,
    ModelCapability,
    ModelLocality,
    PricingTier,
    LatencyClass,
};

// Inference
pub use crate::{
    InferenceError,
    InferenceMetrics,
    ModelLoadStatus,
    TokenEvent,
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

// Ollama provider (always available)
pub use crate::OllamaProvider;

// Cloud provider types (feature-gated)
#[cfg(feature = "bedrock")]
pub use crate::BedrockProvider;

#[cfg(feature = "openai")]
pub use crate::OpenAiCompatibleProvider;

// Tracing
pub use tracing::{info, warn, error, debug, trace, instrument};
