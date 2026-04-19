# Provider Patterns

The Life Savor agent supports four distinct provider patterns for model components. Each pattern has different infrastructure requirements, billing behavior, and implementation details.

## Pattern Overview

| Pattern | Use Case | Billing | Key Management | Example Components |
|---------|----------|---------|----------------|-------------------|
| API Gateway | Commercial models (lifesavor_hosted) | Platform billing via fmMeter | Gateway retrieves from vault | gpt-4o, claude-3-5-sonnet, gemini-1-5-pro |
| Local/NativeRuntime | Open-source models | Free (no billing) | N/A | llama-3-8b, tinyllama-1-1b, phi-3-mini |
| BYOK | Commercial models (user's key) | Free (user pays vendor) | CredentialManager → vault | gpt-4o-byok, claude-3-5-sonnet-byok |
| TTS/Voice | Audio models | Free (no billing) | N/A | whisper-large-v3, xtts-v2, bark |

---

## Pattern 1: API Gateway (lifesavor_hosted)

API components route inference through the service-api model execution gateway. The component formats requests and consumes the gateway's SSE response — it never touches vendor API keys directly.

### Flow

```
Component → POST /v1/model-execution/execute-stream → Gateway
Gateway → resolveEffectiveProfile() → buildRoutingPlan() → checkBudgetEnforcement()
Gateway → Vendor Adapter → Vendor API (OpenAI/Anthropic/Google/Mistral)
Vendor API → SSE token stream → Gateway → Component
Gateway → fmMeter.computeCost() + tokenLedger.appendEvent()
```

### Implementation

```rust
use lifesavor_model_sdk::prelude::*;
use async_trait::async_trait;
use tokio::sync::mpsc;

pub struct ApiGatewayProvider {
    gateway_url: String,
    model_id: String,
}

#[async_trait]
impl LlmProvider for ApiGatewayProvider {
    async fn chat_completion_stream(
        &self,
        request: &ChatRequest,
        tx: mpsc::Sender<TokenEvent>,
    ) -> Result<InferenceMetrics, InferenceError> {
        // Format the request for the gateway
        let gateway_request = serde_json::json!({
            "message": request.messages.last().map(|m| &m.content).unwrap_or(&String::new()),
            "tools_requested": request.tools,
            "model_id": self.model_id,
            "context": request.messages.iter()
                .take(request.messages.len().saturating_sub(1))
                .map(|m| serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                    "images": m.images,
                    "tool_calls": m.tool_calls,
                    "tool_call_id": m.tool_call_id,
                }))
                .collect::<Vec<_>>(),
        });

        // POST to the gateway's SSE endpoint
        let response = reqwest::Client::new()
            .post(format!("{}/v1/model-execution/execute-stream", self.gateway_url))
            .json(&gateway_request)
            .send()
            .await
            .map_err(|e| InferenceError::ProviderUnavailable(e.to_string()))?;

        // Handle HTTP error codes
        match response.status().as_u16() {
            401 | 403 => return Err(InferenceError::AuthenticationFailed(
                "Gateway authentication failed".into()
            )),
            429 => return Err(InferenceError::RateLimited {
                retry_after_ms: response.headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .map(|s| s * 1000),
            }),
            500..=599 => return Err(InferenceError::ProviderUnavailable(
                format!("Gateway returned {}", response.status())
            )),
            _ => {}
        }

        // Consume SSE stream, forward TokenEvents
        let mut input_tokens = 0u64;
        let mut output_tokens = 0u64;
        let start = std::time::Instant::now();
        let mut ttft_ms = 0u64;
        let mut first_token = true;

        // ... parse SSE events, send tokens via tx ...
        // On "done" event, extract usage metrics from the gateway response

        Ok(InferenceMetrics {
            input_tokens,
            output_tokens,
            ttft_ms,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn capability_descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            models: vec![ModelCapability {
                name: self.model_id.clone(),
                locality: ModelLocality::Remote,
                context_window: 128_000,
                pricing_tier: PricingTier::PerToken {
                    input_per_1k: 0.005,
                    output_per_1k: 0.015,
                },
                latency_class: LatencyClass::Medium,
                load_status: ModelLoadStatus::Ready,
                features: vec![
                    "text_generation".into(),
                    "chat".into(),
                    "function_calling".into(),
                    "vision".into(),
                ],
            }],
            features: vec!["text_generation".into(), "chat".into()],
            locality: Locality::Remote,
        }
    }

    // ... other trait methods ...
#     async fn list_models(&self) -> Result<Vec<ModelInfo>, InferenceError> { todo!() }
#     async fn model_load_status(&self, _model: &str) -> Result<ModelLoadStatus, InferenceError> { todo!() }
#     async fn generate_embedding(&self, _text: &str, _model: &str) -> Result<Vec<f32>, InferenceError> { todo!() }
#     fn resolve_model_alias(&self, alias: &str) -> String { alias.to_string() }
}
```

### Provider Manifest

```toml
provider_type = "llm"
instance_name = "gpt-4o"
sdk_version = "0.5.0"
priority = 80
locality = "remote"

[connection]
endpoint = "gateway"

[auth]
strategy = "gateway"

[health_check]
method = "http_get"
interval_seconds = 30
timeout_seconds = 10
```

### Key Points

- The component never stores or manages vendor API keys
- The gateway handles billing via `fmMeter`/`tokenLedger`
- `PricingTier::PerToken` reflects vendor pricing in USD per 1K tokens
- `auth.strategy = "gateway"` — the gateway retrieves keys from the vault
- Map gateway error events to `InferenceError` variants (`AuthenticationFailed`, `RateLimited`, `ProviderUnavailable`)

---

## Pattern 2: Local/NativeRuntime

Local components run open-source models on the agent's embedded dual-runtime (PyTorch via `tch-rs` + ONNX Runtime via `ort`). The `NativeRuntime` handles hardware detection, model loading, state management, and inference.

### Flow

```
Component → detect_platform() → DetectedPlatform (CUDA/MPS/CoreML/CPU)
Component → select_variant(available_ram) → QuantizationVariant
Component → NativeRuntime::load_pytorch_model() or load_onnx_model()
NativeRuntime → ModelStateManager: Cold → Hot
Component → NativeRuntime::run_inference(tokens) → TokenEvent stream
```

### Implementation

```rust
use lifesavor_model_sdk::prelude::*;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Quantization variants with RAM thresholds for variant selection.
pub struct QuantizationVariant {
    pub model_tag: String,
    pub quantization: String,
    pub estimated_ram_mb: u64,
    pub reason: String,
}

pub struct LocalModelProvider {
    config: LocalModelConfig,
    // NativeRuntime handle provided by the agent at registration
}

impl LocalModelProvider {
    /// Select the best quantization variant based on available RAM.
    /// Higher RAM → higher quality quantization (monotonicity property).
    pub fn select_variant(available_ram_mb: u64) -> QuantizationVariant {
        // Example for an 8B parameter model:
        if available_ram_mb >= 16_000 {
            QuantizationVariant {
                model_tag: "my-model-f16".into(),
                quantization: "F16".into(),
                estimated_ram_mb: 16_000,
                reason: "Full precision, sufficient RAM".into(),
            }
        } else if available_ram_mb >= 8_000 {
            QuantizationVariant {
                model_tag: "my-model-q8_0".into(),
                quantization: "Q8_0".into(),
                estimated_ram_mb: 8_000,
                reason: "8-bit quantization for moderate RAM".into(),
            }
        } else {
            QuantizationVariant {
                model_tag: "my-model-q4_k_m".into(),
                quantization: "Q4_K_M".into(),
                estimated_ram_mb: 4_000,
                reason: "4-bit quantization for low RAM".into(),
            }
        }
    }
}

#[async_trait]
impl LlmProvider for LocalModelProvider {
    async fn chat_completion_stream(
        &self,
        request: &ChatRequest,
        tx: mpsc::Sender<TokenEvent>,
    ) -> Result<InferenceMetrics, InferenceError> {
        // Delegate to NativeRuntime for inference
        // The runtime handles:
        //   - Hardware-specific acceleration (CUDA, MPS, CoreML, CPU)
        //   - Hot/Warm/Cold state management
        //   - Concurrency limits (CPU default 4, GPU default 1)
        //   - Metrics collection
        todo!("Delegate to NativeRuntime")
    }

    fn capability_descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            models: vec![ModelCapability {
                name: "my-model".into(),
                locality: ModelLocality::Local,
                context_window: 4096,
                pricing_tier: PricingTier::Free,
                latency_class: LatencyClass::Medium,
                load_status: ModelLoadStatus::Cold,
                features: vec!["text_generation".into(), "chat".into()],
            }],
            features: vec!["text_generation".into(), "chat".into()],
            locality: Locality::Local,
        }
    }

    // ... other trait methods ...
#     async fn list_models(&self) -> Result<Vec<ModelInfo>, InferenceError> { todo!() }
#     async fn model_load_status(&self, _model: &str) -> Result<ModelLoadStatus, InferenceError> { todo!() }
#     async fn generate_embedding(&self, _text: &str, _model: &str) -> Result<Vec<f32>, InferenceError> { todo!() }
#     fn resolve_model_alias(&self, alias: &str) -> String { alias.to_string() }
}
```

### Hardware Detection

The `NativeRuntime` probes the host and returns a `DetectedPlatform`:

```rust
pub struct DetectedPlatform {
    pub os: String,
    pub arch: String,
    pub onnx_execution_provider: ExecutionProviderKind, // CPU, CUDA, CoreML, DirectML, Neuron
    pub pytorch_device: PyTorchDeviceKind,              // CPU, CUDA, MPS
    pub acceleration_notes: Vec<String>,
    pub detected_at: DateTime<Utc>,
}
```

Components use this to select the appropriate backend and quantization variant.

### Model State Machine

Models transition through three states:

- **Cold** — model weights on disk, not loaded into memory
- **Warm** — model loaded but keep-alive timer expired, may be partially paged out
- **Hot** — model fully loaded and ready for inference

The `NativeRuntime` manages these transitions automatically. Components query state via `model_load_status()`.

### Provider Manifest

```toml
provider_type = "llm"
instance_name = "my-model"
sdk_version = "0.5.0"
priority = 50
locality = "local"

[connection]
endpoint = "native://my-model"

[auth]
strategy = "none"

[health_check]
method = "process_alive"
interval_seconds = 30
timeout_seconds = 5
```

### Key Points

- `PricingTier::Free` — no platform billing for local models
- `locality = "local"` in the manifest
- Variant selection must be monotonic: more RAM → equal or better quantization
- The `NativeRuntime` handles all backend details; components never manage PyTorch/ONNX directly
- Model weights are stored in the configurable `models_dir` (default `~/.local/share/lifesavor-agent/models/`)

---

## Pattern 3: BYOK (Bring Your Own Key)

BYOK components call vendor APIs directly using the user's own API key. No gateway routing, no platform billing.

### Flow

```
Component → CredentialManager::resolve(auth_config) → ResolvedCredential (user's API key)
Component → Direct vendor API call (e.g., https://api.openai.com/v1/chat/completions)
Vendor API → SSE token stream → Component
Component → Parse tokens + InferenceMetrics
(No platform billing)
```

### Implementation

```rust
use lifesavor_model_sdk::prelude::*;
use async_trait::async_trait;
use tokio::sync::mpsc;

pub struct ByokProvider {
    config: ByokConfig,
    credential_manager: CredentialManager,
}

#[async_trait]
impl LlmProvider for ByokProvider {
    async fn chat_completion_stream(
        &self,
        request: &ChatRequest,
        tx: mpsc::Sender<TokenEvent>,
    ) -> Result<InferenceMetrics, InferenceError> {
        // Retrieve the user's API key from the vault
        let credential = self.credential_manager
            .resolve(&self.config.auth, "byok-provider")
            .await
            .map_err(|e| InferenceError::AuthenticationFailed(e.to_string()))?
            .ok_or_else(|| InferenceError::AuthenticationFailed(
                "No API key configured".into()
            ))?;

        // Call the vendor API directly (example: OpenAI)
        let vendor_request = serde_json::json!({
            "model": self.config.model_name,
            "messages": request.messages.iter().map(|m| serde_json::json!({
                "role": m.role,
                "content": m.content,
            })).collect::<Vec<_>>(),
            "stream": true,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
        });

        let response = reqwest::Client::new()
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", credential.value()))
            .json(&vendor_request)
            .send()
            .await
            .map_err(|e| InferenceError::ProviderUnavailable(e.to_string()))?;

        // IMPORTANT: mask credentials in all log output
        tracing::info!(
            model = %self.config.model_name,
            "BYOK inference request sent (key: {}...)",
            &credential.value()[..8]
        );

        // Parse SSE stream, forward tokens, collect metrics
        // ...

        todo!("Parse vendor SSE stream")
    }

    fn capability_descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            models: vec![ModelCapability {
                name: format!("{}-byok", self.config.model_name),
                locality: ModelLocality::Remote,
                context_window: 128_000,
                pricing_tier: PricingTier::Free, // User pays vendor directly
                latency_class: LatencyClass::Medium,
                load_status: ModelLoadStatus::Ready,
                features: vec!["text_generation".into(), "chat".into()],
            }],
            features: vec!["text_generation".into(), "chat".into()],
            locality: Locality::Remote,
        }
    }

    // ... other trait methods ...
#     async fn list_models(&self) -> Result<Vec<ModelInfo>, InferenceError> { todo!() }
#     async fn model_load_status(&self, _model: &str) -> Result<ModelLoadStatus, InferenceError> { todo!() }
#     async fn generate_embedding(&self, _text: &str, _model: &str) -> Result<Vec<f32>, InferenceError> { todo!() }
#     fn resolve_model_alias(&self, alias: &str) -> String { alias.to_string() }
}
```

### Provider Manifest

```toml
provider_type = "llm"
instance_name = "gpt-4o-byok"
sdk_version = "0.5.0"
priority = 70
locality = "remote"
vault_keys = ["openai-api-key"]

[connection]
endpoint = "https://api.openai.com"

[auth]
strategy = "vault"
vault_key = "openai-api-key"

[health_check]
method = "http_get"
interval_seconds = 60
timeout_seconds = 10
```

### Key Points

- `PricingTier::Free` — no platform billing; the user pays the vendor directly
- `provider_type = "byok_cloud"` in the routing plan
- Credentials retrieved via `CredentialManager` with `auth.strategy = "vault"`
- Always mask API keys in log output using `mask_credential`
- BYOK components are separate crates from their `lifesavor_hosted` counterparts
- Declare `vault_keys` in the manifest for the credential allowlist

---

## Pattern 4: TTS/Voice (SystemComponent)

Voice/TTS components implement `SystemComponent` from the System SDK rather than `LlmProvider`, since they process audio rather than text. They register via JSON-RPC and expose audio-specific bridge operations.

### Flow

```
Component → SystemComponent::initialize()
Component → register via JSON-RPC "component.register"
Agent → bridge request: "transcribe" (ASR) or "synthesize" (TTS)
Component → local inference backend (whisper.cpp, etc.)
Component → BridgeResponse with audio/text data
```

### Implementation

```rust
use lifesavor_system_sdk::prelude::*;
use async_trait::async_trait;

pub struct WhisperComponent {
    config: WhisperConfig,
}

#[async_trait]
impl SystemComponent for WhisperComponent {
    fn name(&self) -> &str { "whisper-large-v3" }

    fn component_type(&self) -> SystemComponentType {
        SystemComponentType::Voice
    }

    async fn initialize(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Load whisper.cpp model, initialize audio pipeline
        Ok(())
    }

    async fn health_check(&self) -> ComponentHealthStatus {
        ComponentHealthStatus::Healthy
    }

    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Release model resources
        Ok(())
    }
}

// Bridge operations for audio processing
impl WhisperComponent {
    /// Transcribe audio input to text (ASR).
    pub async fn transcribe(&self, audio_data: &[u8], format: &str) -> Result<String, String> {
        // Accept WAV/MP3/FLAC audio, return transcribed text
        todo!("Implement whisper.cpp transcription")
    }
}

// For TTS components (xtts-v2, bark, styletts2):
pub struct TtsComponent {
    config: TtsConfig,
}

impl TtsComponent {
    /// Synthesize text to audio (TTS).
    pub async fn synthesize(&self, text: &str) -> Result<Vec<u8>, String> {
        // Accept text, return audio data
        todo!("Implement TTS synthesis")
    }
}
```

### Bridge Operations

Voice components expose these bridge operations:

| Operation | Direction | Input | Output |
|-----------|-----------|-------|--------|
| `transcribe` | Audio → Text | WAV/MP3/FLAC audio bytes | Transcribed text |
| `synthesize` | Text → Audio | Text string | Audio bytes |

### MCP Tools

```rust
// ASR component (Whisper)
pub fn mcp_tools() -> Vec<McpToolDefinition> {
    vec![McpToolDefinition {
        name: "whisper-large-v3.transcribe".into(),
        description: "Transcribe audio to text".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "audio": { "type": "string", "description": "Base64-encoded audio data" },
                "format": { "type": "string", "enum": ["wav", "mp3", "flac"] },
                "language": { "type": "string", "description": "ISO 639-1 language code" }
            },
            "required": ["audio"]
        }),
    }]
}

// TTS component (e.g., xtts-v2)
pub fn mcp_tools() -> Vec<McpToolDefinition> {
    vec![McpToolDefinition {
        name: "xtts-v2.synthesize".into(),
        description: "Synthesize text to speech audio".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "text": { "type": "string", "description": "Text to synthesize" },
                "voice": { "type": "string", "description": "Voice preset ID" }
            },
            "required": ["text"]
        }),
    }]
}
```

### Provider Manifest

```toml
provider_type = "system"
instance_name = "whisper-large-v3"
sdk_version = "0.5.0"
priority = 50
locality = "local"

[connection]
endpoint = "native://whisper-large-v3"

[auth]
strategy = "none"

[health_check]
method = "process_alive"
interval_seconds = 30
timeout_seconds = 5
```

### Key Points

- Implements `SystemComponent` trait, not `LlmProvider`
- Uses `lifesavor-system-sdk` as the primary SDK dependency
- `marketplace.toml` category is `"Voice"`, not `"Llm"`
- Audio output uses `StreamingEnvelope` with `content_type: "audio"` for WebSocket streaming
- Local execution via specialized inference backends (whisper.cpp, etc.)

---

## Choosing a Pattern

| Question | Answer → Pattern |
|----------|-----------------|
| Is it a commercial model hosted by a vendor? | API Gateway or BYOK |
| Does the user want platform billing? | API Gateway |
| Does the user want to use their own API key? | BYOK |
| Is it an open-source model running locally? | Local/NativeRuntime |
| Does it process audio instead of text? | TTS/Voice |
