//! Minimal Ollama LLM provider example.
//!
//! Demonstrates building an LLM provider that wraps a local Ollama instance,
//! implementing `chat_completion_stream`, `list_models`, `model_load_status`,
//! and `generate_embedding` with streaming via `StreamingEnvelope`.
//!
//! Run with: `cargo run --example ollama_provider`

use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::mpsc;

use lifesavor_model_sdk::prelude::*;
use lifesavor_model_sdk::builder::ModelProviderBuilder;
use lifesavor_model_sdk::{
    ConnectionConfig, HealthCheckConfig, HealthCheckMethod, Locality,
};

// ---------------------------------------------------------------------------
// Minimal Ollama-style provider
// ---------------------------------------------------------------------------

/// A minimal LLM provider wrapping a local Ollama instance.
///
/// In production you would delegate to the real `OllamaProvider`; this
/// example shows the trait surface and streaming pattern.
struct MinimalOllamaProvider {
    base_url: String,
    model_aliases: HashMap<String, String>,
}

impl MinimalOllamaProvider {
    fn new(base_url: &str, aliases: HashMap<String, String>) -> Self {
        Self {
            base_url: base_url.to_string(),
            model_aliases: aliases,
        }
    }
}

#[async_trait]
impl LlmProvider for MinimalOllamaProvider {
    async fn chat_completion_stream(
        &self,
        request: &ChatRequest,
        tx: mpsc::Sender<TokenEvent>,
    ) -> Result<InferenceMetrics, InferenceError> {
        let model = self.resolve_model_alias(&request.model);
        info!(model = %model, base_url = %self.base_url, "Starting chat completion stream");

        // Simulate streaming tokens back to the caller.
        let tokens = ["Hello", " from", " Ollama", "!"];
        for (i, tok) in tokens.iter().enumerate() {
            let event = TokenEvent {
                execution_id: request.execution_id.clone(),
                token: tok.to_string(),
                index: i as u32,
            };
            if tx.send(event).await.is_err() {
                warn!("Token receiver dropped");
                break;
            }
        }

        Ok(InferenceMetrics {
            input_tokens: 10,
            output_tokens: tokens.len() as u64,
            ttft_ms: 50,
            duration_ms: 200,
        })
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, InferenceError> {
        info!(base_url = %self.base_url, "Listing models");
        Ok(vec![
            ModelInfo {
                name: "llama3:8b".to_string(),
                locality: ModelLocality::Local,
                context_window: Some(8192),
                features: vec!["streaming".to_string()],
            },
            ModelInfo {
                name: "mistral:7b".to_string(),
                locality: ModelLocality::Local,
                context_window: Some(4096),
                features: vec!["streaming".to_string()],
            },
        ])
    }

    async fn model_load_status(&self, model: &str) -> Result<ModelLoadStatus, InferenceError> {
        info!(model = %model, "Checking model load status");
        // Simulate: llama3 is hot, everything else is unloaded.
        if model.starts_with("llama3") {
            Ok(ModelLoadStatus::Hot)
        } else {
            Ok(ModelLoadStatus::Unloaded)
        }
    }

    async fn generate_embedding(
        &self,
        text: &str,
        model: &str,
    ) -> Result<Vec<f32>, InferenceError> {
        info!(model = %model, text_len = text.len(), "Generating embedding");
        // Return a deterministic placeholder embedding.
        Ok(vec![0.1, 0.2, 0.3, 0.4, 0.5])
    }

    fn capability_descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            models: vec![ModelCapability {
                name: "llama3:8b".to_string(),
                locality: ModelLocality::Local,
                context_window: 8192,
                pricing_tier: PricingTier::Free,
                latency_class: LatencyClass::Low,
                load_status: ModelLoadStatus::Hot,
                features: vec!["streaming".to_string()],
            }],
            features: vec!["streaming".to_string(), "embeddings".to_string()],
            locality: Locality::Local,
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
// Streaming envelope demonstration
// ---------------------------------------------------------------------------

/// Wrap token events into `StreamingEnvelope` frames for WebSocket delivery.
fn wrap_tokens_as_envelopes(tokens: &[TokenEvent], correlation_id: &str) -> Vec<StreamingEnvelope> {
    let stream_id = uuid::Uuid::new_v4().to_string();
    let metadata = StreamMetadata {
        source_component: "llm".to_string(),
        correlation_id: correlation_id.to_string(),
        total_chunks: Some(tokens.len() as u64 + 1),
        extra: HashMap::new(),
    };

    let mut envelopes: Vec<StreamingEnvelope> = tokens
        .iter()
        .enumerate()
        .map(|(i, tok)| {
            StreamingEnvelope::data(
                &stream_id,
                i as u64,
                "text/plain",
                &tok.token,
                metadata.clone(),
            )
        })
        .collect();

    envelopes.push(StreamingEnvelope::complete(
        &stream_id,
        tokens.len() as u64,
        "text/plain",
        metadata,
    ));

    envelopes
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("Ollama provider example — building provider via ModelProviderBuilder");

    // Build a valid LLM manifest.
    let manifest = ProviderManifest {
        provider_type: ProviderType::Llm,
        instance_name: "ollama-local".to_string(),
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
        model_aliases: {
            let mut m = HashMap::new();
            m.insert("fast".to_string(), "llama3:8b".to_string());
            m
        },
    };

    // Validate the manifest via the builder.
    let _scaffold = ModelProviderBuilder::new(manifest.clone())
        .expect("valid LLM manifest")
        .build();
    info!("ModelProviderBuilder accepted the manifest");

    // Use our custom provider implementation.
    let provider = MinimalOllamaProvider::new(
        "http://localhost:11434",
        manifest.model_aliases.clone(),
    );

    // List models.
    let models = provider.list_models().await.unwrap();
    info!(count = models.len(), "Available models");
    for m in &models {
        info!(name = %m.name, locality = ?m.locality, "  model");
    }

    // Stream a chat completion.
    let (tx, mut rx) = mpsc::channel(32);
    let request = ChatRequest {
        execution_id: "exec-001".to_string(),
        conversation_id: "conv-001".to_string(),
        model: "fast".to_string(), // alias → llama3:8b
        messages: vec![],
        options: None,
    };

    let metrics = provider.chat_completion_stream(&request, tx).await.unwrap();
    info!(
        input_tokens = metrics.input_tokens,
        output_tokens = metrics.output_tokens,
        ttft_ms = metrics.ttft_ms,
        duration_ms = metrics.duration_ms,
        "Inference complete"
    );

    // Collect tokens and wrap as StreamingEnvelope frames.
    let mut tokens = Vec::new();
    while let Ok(tok) = rx.try_recv() {
        tokens.push(tok);
    }

    let envelopes = wrap_tokens_as_envelopes(&tokens, "corr-001");
    for env in &envelopes {
        info!(
            stream_id = %env.stream_id,
            seq = env.sequence,
            status = ?env.status,
            payload = %env.payload,
            "StreamingEnvelope"
        );
    }

    // Capability descriptor.
    let caps = provider.capability_descriptor();
    info!(
        models = caps.models.len(),
        features = ?caps.features,
        locality = ?caps.locality,
        "Capability descriptor"
    );

    info!("Ollama provider example complete");
}
