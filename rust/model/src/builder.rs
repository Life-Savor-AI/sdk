//! Builder for constructing [`LlmProvider`] scaffold implementations.
//!
//! The [`ModelProviderBuilder`] accepts a [`ProviderManifest`], validates that
//! it targets the LLM provider type, runs manifest validation, and produces a
//! scaffold struct implementing [`LlmProvider`] with `unimplemented!()` stubs
//! for all trait methods. Developers then replace the stubs incrementally.
//!
//! # Example
//!
//! ```rust,ignore
//! use lifesavor_model_sdk::prelude::*;
//! use lifesavor_model_sdk::builder::ModelProviderBuilder;
//!
//! let provider = ModelProviderBuilder::new(manifest)
//!     .expect("valid LLM manifest")
//!     .build();
//! ```

use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::error::ModelSdkError;
use crate::{
    CapabilityDescriptor, ChatRequest, InferenceError, InferenceMetrics, LlmProvider,
    ModelInfo, ModelLoadStatus, ProviderManifest, ProviderType, TokenEvent,
    ManifestValidationError, validate_manifest,
};

/// Builder for constructing an [`LlmProvider`] scaffold from a
/// [`ProviderManifest`].
///
/// The builder validates that the manifest's `provider_type` is
/// [`ProviderType::Llm`] and that the manifest passes
/// [`validate_manifest`]. On success, [`build()`](Self::build) returns a
/// scaffold struct that implements [`LlmProvider`] with `unimplemented!()`
/// stubs for every trait method.
#[derive(Debug)]
pub struct ModelProviderBuilder {
    manifest: ProviderManifest,
}

impl ModelProviderBuilder {
    /// Create a new builder from a [`ProviderManifest`].
    ///
    /// Returns an error if:
    /// - `manifest.provider_type` is not [`ProviderType::Llm`]
    /// - [`validate_manifest`] reports validation errors
    pub fn new(manifest: ProviderManifest) -> Result<Self, ModelSdkError> {
        // Reject non-LLM provider types with a descriptive error.
        if manifest.provider_type != ProviderType::Llm {
            return Err(ModelSdkError::ManifestValidation(ManifestValidationError {
                file_path: String::new(),
                field_name: "provider_type".to_string(),
                description: format!(
                    "Model SDK requires provider_type 'llm', got '{}'",
                    manifest.provider_type,
                ),
            }));
        }

        // Run full manifest validation and propagate errors.
        if let Err(errors) = validate_manifest(&manifest, "<builder>") {
            // Return the first validation error (they all map via From).
            if let Some(first) = errors.into_iter().next() {
                return Err(ModelSdkError::ManifestValidation(first));
            }
        }

        Ok(Self { manifest })
    }

    /// Consume the builder and produce a scaffold [`LlmProvider`]
    /// implementation.
    ///
    /// All trait methods are stubbed with `unimplemented!()`. The returned
    /// struct stores the manifest for reference.
    pub fn build(self) -> ScaffoldLlmProvider {
        let model_aliases = self.manifest.model_aliases.clone();
        ScaffoldLlmProvider {
            manifest: self.manifest,
            model_aliases,
        }
    }
}

/// Scaffold [`LlmProvider`] produced by [`ModelProviderBuilder::build`].
///
/// Every trait method panics with `unimplemented!()`. Replace each stub
/// with your real implementation incrementally.
pub struct ScaffoldLlmProvider {
    /// The validated provider manifest.
    pub manifest: ProviderManifest,
    model_aliases: HashMap<String, String>,
}

#[async_trait]
impl LlmProvider for ScaffoldLlmProvider {
    async fn chat_completion_stream(
        &self,
        _request: &ChatRequest,
        _tx: mpsc::Sender<TokenEvent>,
    ) -> Result<InferenceMetrics, InferenceError> {
        unimplemented!("chat_completion_stream: replace this stub with your implementation")
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, InferenceError> {
        unimplemented!("list_models: replace this stub with your implementation")
    }

    async fn model_load_status(&self, _model: &str) -> Result<ModelLoadStatus, InferenceError> {
        unimplemented!("model_load_status: replace this stub with your implementation")
    }

    async fn generate_embedding(
        &self,
        _text: &str,
        _model: &str,
    ) -> Result<Vec<f32>, InferenceError> {
        unimplemented!("generate_embedding: replace this stub with your implementation")
    }

    fn capability_descriptor(&self) -> CapabilityDescriptor {
        unimplemented!("capability_descriptor: replace this stub with your implementation")
    }

    fn resolve_model_alias(&self, alias: &str) -> String {
        self.model_aliases
            .get(alias)
            .cloned()
            .unwrap_or_else(|| alias.to_string())
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AuthConfig, ConnectionConfig, CredentialSource, HealthCheckConfig,
        HealthCheckMethod, Locality,
    };

    /// Helper to build a valid LLM manifest for testing.
    fn valid_llm_manifest() -> ProviderManifest {
        ProviderManifest {
            provider_type: ProviderType::Llm,
            instance_name: "test-llm".to_string(),
            sdk_version: "0.1.0".to_string(),
            connection: ConnectionConfig {
                base_url: Some("http://localhost:11434".to_string()),
                region: None,
                database_url: None,
                extension_path: None,
                command: None,
                args: None,
                transport: None,
            },
            auth: AuthConfig {
                source: CredentialSource::None,
                key_name: None,
                env_var: None,
                secret_arn: None,
                file_path: None,
            },
            health_check: HealthCheckConfig {
                interval_seconds: 30,
                timeout_seconds: 5,
                consecutive_failures_threshold: 3,
                method: HealthCheckMethod::ConnectionPing,
            },
            priority: 100,
            locality: Locality::Local,
            depends_on: vec![],
            capabilities: None,
            cost_limits: None,
            sandbox: None,
            vault_keys: vec![],
            model_aliases: HashMap::new(),
        }
    }

    #[test]
    fn new_accepts_valid_llm_manifest() {
        let manifest = valid_llm_manifest();
        let builder = ModelProviderBuilder::new(manifest);
        assert!(builder.is_ok());
    }

    #[test]
    fn new_rejects_skill_provider_type() {
        let mut manifest = valid_llm_manifest();
        manifest.provider_type = ProviderType::Skill;
        let err = ModelProviderBuilder::new(manifest).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("provider_type"), "error should mention provider_type: {msg}");
        assert!(msg.contains("llm"), "error should mention expected type: {msg}");
    }

    #[test]
    fn new_rejects_assistant_provider_type() {
        let mut manifest = valid_llm_manifest();
        manifest.provider_type = ProviderType::Assistant;
        let err = ModelProviderBuilder::new(manifest).unwrap_err();
        assert!(err.to_string().contains("provider_type"));
    }

    #[test]
    fn new_rejects_vector_store_provider_type() {
        let mut manifest = valid_llm_manifest();
        manifest.provider_type = ProviderType::MemoryStore;
        let err = ModelProviderBuilder::new(manifest).unwrap_err();
        assert!(err.to_string().contains("provider_type"));
    }

    #[test]
    fn new_rejects_invalid_manifest_empty_instance_name() {
        let mut manifest = valid_llm_manifest();
        manifest.instance_name = String::new();
        let err = ModelProviderBuilder::new(manifest).unwrap_err();
        assert!(err.to_string().contains("instance_name"));
    }

    #[test]
    fn new_rejects_invalid_manifest_empty_sdk_version() {
        let mut manifest = valid_llm_manifest();
        manifest.sdk_version = String::new();
        let err = ModelProviderBuilder::new(manifest).unwrap_err();
        assert!(err.to_string().contains("sdk_version"));
    }

    #[test]
    fn new_rejects_missing_base_url() {
        let mut manifest = valid_llm_manifest();
        manifest.connection.base_url = None;
        let err = ModelProviderBuilder::new(manifest).unwrap_err();
        assert!(err.to_string().contains("base_url"));
    }

    #[test]
    fn build_returns_scaffold_with_manifest() {
        let manifest = valid_llm_manifest();
        let provider = ModelProviderBuilder::new(manifest.clone())
            .unwrap()
            .build();
        assert_eq!(provider.manifest.instance_name, "test-llm");
    }

    #[test]
    fn scaffold_resolves_model_alias() {
        let mut manifest = valid_llm_manifest();
        manifest.model_aliases.insert("fast".to_string(), "llama3:8b".to_string());
        let provider = ModelProviderBuilder::new(manifest).unwrap().build();
        assert_eq!(provider.resolve_model_alias("fast"), "llama3:8b");
        assert_eq!(provider.resolve_model_alias("unknown"), "unknown");
    }
}
