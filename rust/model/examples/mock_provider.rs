//! Mock LLM provider example.
//!
//! Demonstrates a mock LLM provider with canned responses, a fully populated
//! `CapabilityDescriptor`, and model aliasing. Useful as a template for
//! integration testing without a real LLM backend.
//!
//! Run with: `cargo run --example mock_provider`

use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::mpsc;

use lifesavor_model_sdk::prelude::*;
use lifesavor_model_sdk::builder::ModelProviderBuilder;
use lifesavor_model_sdk::testing::MockRegistry;
use lifesavor_model_sdk::{
    ConnectionConfig, HealthCheckConfig, HealthCheckMethod, Locality,
};

// ---------------------------------------------------------------------------
// Mock provider with canned responses
// ---------------------------------------------------------------------------

/// A mock LLM provider that returns deterministic canned responses.
///
/// Demonstrates `CapabilityDescriptor` construction and model alias
/// resolution without requiring any external service.
struct MockLlmProvider {
    model_aliases: HashMap<String, String>,
    capabilities: CapabilityDescriptor,
}

impl MockLlmProvider {
    fn new(aliases: HashMap<String, String>) -> Self {
        let capabilities = CapabilityDescriptor {
            models: vec![
                ModelCapability {
                    name: "mock-gpt-4".to_string(),
                    locality: ModelLocality::Local,
                    context_window: 128_000,
                    pricing_tier: PricingTier::Free,
                    latency_class: LatencyClass::Low,
                    load_status: ModelLoadStatus::Hot,
                    features: vec!["streaming".to_string(), "function_calling".to_string()],
                },
                ModelCapability {
                    name: "mock-embed-v1".to_string(),
                    locality: ModelLocality::Local,
                    context_window: 8192,
                    pricing_tier: PricingTier::Free,
                    latency_class: LatencyClass::Low,
                    load_status: ModelLoadStatus::Loaded,
                    features: vec!["embeddings".to_string()],
                },
            ],
            features: vec![
                "streaming".to_string(),
                "embeddings".to_string(),
                "function_calling".to_string(),
            ],
            locality: Locality::Local,
        };
        Self {
            model_aliases: aliases,
            capabilities,
        }
    }
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn chat_completion_stream(
        &self,
        request: &ChatRequest,
        tx: mpsc::Sender<TokenEvent>,
    ) -> Result<InferenceMetrics, InferenceError> {
        let model = self.resolve_model_alias(&request.model);
        info!(model = %model, "Mock: streaming canned response");

        let canned = "This is a canned mock response.";
        for (i, word) in canned.split_whitespace().enumerate() {
            let event = TokenEvent {
                execution_id: request.execution_id.clone(),
                token: format!("{word} "),
                index: i as u32,
            };
            let _ = tx.send(event).await;
        }

        Ok(InferenceMetrics {
            input_tokens: 5,
            output_tokens: 6,
            ttft_ms: 1,
            duration_ms: 10,
        })
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, InferenceError> {
        Ok(self
            .capabilities
            .models
            .iter()
            .map(|mc| ModelInfo {
                name: mc.name.clone(),
                locality: mc.locality,
                context_window: Some(mc.context_window),
                features: mc.features.clone(),
            })
            .collect())
    }

    async fn model_load_status(&self, model: &str) -> Result<ModelLoadStatus, InferenceError> {
        // Look up in capability descriptor.
        Ok(self
            .capabilities
            .models
            .iter()
            .find(|mc| mc.name == model)
            .map(|mc| mc.load_status.clone())
            .unwrap_or(ModelLoadStatus::Unloaded))
    }

    async fn generate_embedding(
        &self,
        _text: &str,
        _model: &str,
    ) -> Result<Vec<f32>, InferenceError> {
        Ok(vec![0.0; 384]) // 384-dim zero vector
    }

    fn capability_descriptor(&self) -> CapabilityDescriptor {
        self.capabilities.clone()
    }

    fn resolve_model_alias(&self, alias: &str) -> String {
        self.model_aliases
            .get(alias)
            .cloned()
            .unwrap_or_else(|| alias.to_string())
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("Mock provider example — canned responses, CapabilityDescriptor, aliasing");

    // Build a manifest with model aliases.
    let mut aliases = HashMap::new();
    aliases.insert("default".to_string(), "mock-gpt-4".to_string());
    aliases.insert("embed".to_string(), "mock-embed-v1".to_string());

    let manifest = ProviderManifest {
        provider_type: ProviderType::Llm,
        instance_name: "mock-llm".to_string(),
        sdk_version: "0.1.0".to_string(),
        connection: ConnectionConfig {
            base_url: Some("http://localhost:0".to_string()),
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
            interval_seconds: 60,
            timeout_seconds: 5,
            consecutive_failures_threshold: 3,
            method: HealthCheckMethod::ConnectionPing,
        },
        priority: 50,
        locality: Locality::Local,
        depends_on: vec![],
        capabilities: None,
        cost_limits: None,
        sandbox: None,
        vault_keys: vec![],
        model_aliases: aliases.clone(),
    };

    // Validate via builder.
    let scaffold = ModelProviderBuilder::new(manifest.clone())
        .expect("valid LLM manifest")
        .build();

    // Demonstrate alias resolution on the scaffold.
    assert_eq!(scaffold.resolve_model_alias("default"), "mock-gpt-4");
    assert_eq!(scaffold.resolve_model_alias("embed"), "mock-embed-v1");
    assert_eq!(scaffold.resolve_model_alias("unknown"), "unknown");
    info!("Model alias resolution verified on scaffold");

    // Use the mock provider.
    let provider = MockLlmProvider::new(aliases);

    // CapabilityDescriptor inspection.
    let caps = provider.capability_descriptor();
    info!(
        model_count = caps.models.len(),
        features = ?caps.features,
        locality = ?caps.locality,
        "CapabilityDescriptor"
    );
    for mc in &caps.models {
        info!(
            name = %mc.name,
            context_window = mc.context_window,
            pricing = ?mc.pricing_tier,
            latency = ?mc.latency_class,
            load_status = ?mc.load_status,
            "  model capability"
        );
    }

    // Stream a canned response.
    let (tx, mut rx) = mpsc::channel(32);
    let request = ChatRequest {
        execution_id: "exec-mock-001".to_string(),
        conversation_id: "conv-mock-001".to_string(),
        model: "default".to_string(), // alias → mock-gpt-4
        messages: vec![],
        options: None,
    };

    let metrics = provider.chat_completion_stream(&request, tx).await.unwrap();
    info!(output_tokens = metrics.output_tokens, "Canned response streamed");

    let mut response = String::new();
    while let Ok(tok) = rx.try_recv() {
        response.push_str(&tok.token);
    }
    info!(response = %response.trim(), "Assembled response");

    // Register in MockRegistry.
    let mut registry = MockRegistry::new();
    registry.register("mock-llm", ProviderType::Llm);
    assert!(registry.get_by_type(ProviderType::Llm).is_some());
    info!("Mock provider registered in MockRegistry");

    info!("Mock provider example complete");
}
