//! Hot/cold model management example.
//!
//! Demonstrates the `model_load_status` trait method and the
//! `ModelLoadStatus` enum (`Hot`, `Loaded`, `Cold`, `Unloaded`).
//! Shows how a provider reports model lifecycle states for the agent's
//! warm-up manager to make routing decisions.
//!
//! Run with: `cargo run --example hot_cold_management`

use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::mpsc;

use lifesavor_model_sdk::prelude::*;
use lifesavor_model_sdk::Locality;

// ---------------------------------------------------------------------------
// Provider with hot/cold model tracking
// ---------------------------------------------------------------------------

/// A provider that tracks model load states to demonstrate the
/// `ModelLoadStatus` lifecycle.
struct HotColdProvider {
    /// Simulated model states.
    model_states: HashMap<String, ModelLoadStatus>,
}

impl HotColdProvider {
    fn new() -> Self {
        let mut states = HashMap::new();
        states.insert("llama3:70b".to_string(), ModelLoadStatus::Hot);
        states.insert("llama3:8b".to_string(), ModelLoadStatus::Loaded);
        states.insert("codellama:13b".to_string(), ModelLoadStatus::Cold);
        // "mistral:7b" is intentionally absent → Unloaded
        Self {
            model_states: states,
        }
    }
}

#[async_trait]
impl LlmProvider for HotColdProvider {
    async fn chat_completion_stream(
        &self,
        _request: &ChatRequest,
        _tx: mpsc::Sender<TokenEvent>,
    ) -> Result<InferenceMetrics, InferenceError> {
        Ok(InferenceMetrics {
            input_tokens: 0,
            output_tokens: 0,
            ttft_ms: 0,
            duration_ms: 0,
        })
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, InferenceError> {
        Ok(self
            .model_states
            .keys()
            .map(|name| ModelInfo {
                name: name.clone(),
                locality: ModelLocality::Local,
                context_window: Some(4096),
                features: vec!["streaming".to_string()],
            })
            .collect())
    }

    async fn model_load_status(&self, model: &str) -> Result<ModelLoadStatus, InferenceError> {
        Ok(self
            .model_states
            .get(model)
            .cloned()
            .unwrap_or(ModelLoadStatus::Unloaded))
    }

    async fn generate_embedding(
        &self,
        _text: &str,
        _model: &str,
    ) -> Result<Vec<f32>, InferenceError> {
        Ok(vec![])
    }

    fn capability_descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            models: self
                .model_states
                .iter()
                .map(|(name, status)| ModelCapability {
                    name: name.clone(),
                    locality: ModelLocality::Local,
                    context_window: 4096,
                    pricing_tier: PricingTier::Free,
                    latency_class: LatencyClass::Low,
                    load_status: status.clone(),
                    features: vec!["streaming".to_string()],
                })
                .collect(),
            features: vec!["streaming".to_string()],
            locality: Locality::Local,
        }
    }

    fn resolve_model_alias(&self, alias: &str) -> String {
        alias.to_string()
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("Hot/cold model management example");

    let provider = HotColdProvider::new();

    // Query load status for each model.
    let models_to_check = ["llama3:70b", "llama3:8b", "codellama:13b", "mistral:7b"];

    for model in &models_to_check {
        let status = provider.model_load_status(model).await.unwrap();
        info!(model = %model, status = %status, "Model load status");

        // Demonstrate routing logic based on status.
        match &status {
            ModelLoadStatus::Hot => {
                info!(model = %model, "  → Always loaded, lowest latency — preferred for routing");
            }
            ModelLoadStatus::Loaded => {
                info!(model = %model, "  → Currently loaded, available for requests");
            }
            ModelLoadStatus::Cold => {
                info!(model = %model, "  → Past cold timeout, eligible for unloading");
            }
            ModelLoadStatus::Unloaded => {
                info!(model = %model, "  → Not loaded, requires warm-up before use");
            }
        }
    }

    // Show capability descriptor with load statuses.
    let caps = provider.capability_descriptor();
    info!(model_count = caps.models.len(), "Capability descriptor");
    for mc in &caps.models {
        info!(
            name = %mc.name,
            load_status = ?mc.load_status,
            "  capability"
        );
    }

    info!("Hot/cold management example complete");
}
