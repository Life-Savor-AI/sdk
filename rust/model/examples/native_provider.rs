//! Minimal native LLM provider example.
//!
//! Demonstrates building an LLM provider that wraps the NativeRuntime,
//! implementing `chat_completion_stream`, `list_models`, `model_load_status`,
//! and `generate_embedding` with streaming via `StreamingEnvelope`.
//!
//! Run with: `cargo run --example native_provider`

use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::info;

use lifesavor_model_sdk::prelude::*;
use lifesavor_model_sdk::builder::ModelProviderBuilder;

// ---------------------------------------------------------------------------
// Minimal native-runtime-style provider
// ---------------------------------------------------------------------------

/// A minimal LLM provider wrapping the NativeRuntime.
///
/// In production you would delegate to the real NativeRuntime; this
/// example shows the trait surface and streaming pattern.
struct MinimalNativeProvider {
    base_url: String,
    model_aliases: HashMap<String, String>,
}

impl MinimalNativeProvider {
    fn new(base_url: &str, aliases: HashMap<String, String>) -> Self {
        Self {
            base_url: base_url.to_string(),
            model_aliases: aliases,
        }
    }
}

#[async_trait]
impl LlmProvider for MinimalNativeProvider {
    async fn chat_completion_stream(
        &self,
        request: &ChatRequest,
        tx: mpsc::Sender<TokenEvent>,
    ) -> Result<InferenceMetrics, InferenceError> {

        // Simulate streaming tokens back to the caller.
        let tokens = ["Hello", " from", " NativeRuntime", "!"];
        for (i, tok) in tokens.iter().enumerate() {
            let event = TokenEvent {
                execution_id: request.execution_id.clone(),
                token: tok.to_string(),
                index: i as u32,
            };
            if tx.send(event).await.is_err() {
                break; // receiver dropped
            }
        }

        Ok(InferenceMetrics {
            input_tokens: request.messages.len() as u64 * 10,
            output_tokens: tokens.len() as u64,
            ttft_ms: 12,
            duration_ms: 48,
        })
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, InferenceError> {
        Ok(vec![ModelInfo {
            name: "tinyllama:1.1b".to_string(),
            modified_at: "2024-01-01T00:00:00Z".to_string(),
            size: 637_000_000,
        }])
    }

    async fn model_load_status(&self, model: &str) -> Result<ModelLoadStatus, InferenceError> {
        let _ = model;
        Ok(ModelLoadStatus::Hot)
    }

    async fn generate_embedding(
        &self,
        text: &str,
        model: &str,
    ) -> Result<Vec<f32>, InferenceError> {
        let _ = (text, model);
        Ok(vec![0.0; 384])
    }

    fn capability_descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            models: vec![ModelCapability {
                name: "tinyllama:1.1b".to_string(),
                locality: ModelLocality::Local,
                context_window: 2048,
                pricing_tier: PricingTier::Free,
                latency_class: LatencyClass::Low,
                load_status: ModelLoadStatus::Hot,
                features: vec!["text_generation".into(), "chat".into()],
            }],
            features: vec!["text_generation".into(), "chat".into()],
            locality: crate::Locality::Local,
        }
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
async fn main() {
    tracing_subscriber::fmt::init();

    info!("Native provider example — building provider via ModelProviderBuilder");

    // Build a valid LLM manifest.
    let manifest = ProviderManifest {
        provider_type: ProviderType::Llm,
        instance_name: "native-local".to_string(),
        sdk_version: "0.1.0".to_string(),
        connection: ConnectionConfig {
            endpoint: Some("http://localhost:11434".to_string()),
            gateway: None,
            socket_path: None,
            protocol: None,
            tls: None,
        },
        auth: AuthConfig {
            strategy: "none".to_string(),
            source: None,
            vault_key: None,
            env_var: None,
            file_path: None,
        },
        health_check: HealthCheckConfig {
            method: HealthCheckMethod::HttpGet,
            interval_seconds: 30,
            timeout_seconds: 5,
            endpoint: None,
        },
        priority: 10,
        locality: crate::Locality::Local,
        depends_on: vec![],
        capabilities: None,
        cost_limits: None,
        sandbox: None,
        vault_keys: vec![],
        model_aliases: HashMap::from([
            ("tiny".to_string(), "tinyllama:1.1b".to_string()),
        ]),
    };

    // Use our custom provider implementation.
    let provider = MinimalNativeProvider::new(
        "http://localhost:11434",
        manifest.model_aliases.clone(),
    );

    // Validate the manifest.
    match validate_manifest(&manifest) {
        Ok(()) => info!("Manifest validation passed"),
        Err(e) => {
            eprintln!("Manifest validation failed: {e}");
            return;
        }
    }

    // Stream a chat completion.
    let request = ChatRequest {
        execution_id: "ex-001".to_string(),
        conversation_id: "conv-001".to_string(),
        model: "tinyllama:1.1b".to_string(),
        messages: vec![],
        options: None,
        tools: None,
    };

    let (tx, mut rx) = mpsc::channel(32);
    let metrics = provider.chat_completion_stream(&request, tx).await.unwrap();

    while let Some(event) = rx.recv().await {
        info!(token = %event.token, index = event.index, "received token");
    }

    info!(
        input_tokens = metrics.input_tokens,
        output_tokens = metrics.output_tokens,
        ttft_ms = metrics.ttft_ms,
        duration_ms = metrics.duration_ms,
        "inference complete"
    );

    info!("Native provider example complete");
}
