# Migration Guide — Built-In Providers to Components

This guide covers migrating from the old built-in providers (`OpenAiCompatibleProvider`, `BedrockProvider`, `OllamaProvider`) to the new component-based architecture.

## What Changed

The agent no longer ships with any built-in model providers. All model support is delivered through installable marketplace components.

### Removed

| Old Code | Status | Replacement |
|----------|--------|-------------|
| `OpenAiCompatibleProvider` | Removed entirely | API gateway components (e.g., `gpt-4o`) or BYOK components (e.g., `gpt-4o-byok`) |
| `BedrockProvider` | Removed entirely | Not replaced in this release (future component) |
| `OllamaProvider` | Removed entirely | Local/NativeRuntime components (e.g., `llama-3-8b-instruct`, `tinyllama-1-1b`) |
| `InferenceBridge` legacy fallback | Removed | All inference routes through registered providers |
| Ollama configuration sections | Removed | NativeRuntime configuration in `llm-config.yaml` |

### Retained

| Component | Status |
|-----------|--------|
| `LlmProvider` trait | Unchanged — still the core trait for all LLM components |
| `InferenceBridge` | Retained, but without legacy `InferenceEngine` fallback |
| `IntegrationRegistry` | Retained — manages provider lifecycle |
| `ProviderManifest` | Retained — declares component identity and configuration |
| `HealthMonitor` | Retained — tracks per-provider health |
| `NativeRuntime` | Retained — embedded dual-runtime for local models |
| `CredentialManager` | Retained — resolves credentials from vault |

## Migration Steps

### Step 1: Remove Built-In Provider References

If your code imported or referenced the old providers, remove those references:

```rust
// BEFORE (no longer compiles)
use lifesavor_agent::providers::openai_compatible::OpenAiCompatibleProvider;
use lifesavor_agent::providers::bedrock::BedrockProvider;
use lifesavor_model_sdk::OllamaProvider;

// AFTER — use the LlmProvider trait and let the registry resolve providers
use lifesavor_model_sdk::prelude::*;
// LlmProvider, ChatRequest, InferenceMetrics, TokenEvent, etc.
```

### Step 2: Update InferenceError Handling

Three new variants were added to `InferenceError`. Update any `match` arms:

```rust
// BEFORE
match error {
    InferenceError::BackendUnavailable => { /* ... */ }
    InferenceError::ModelNotLoaded(m) => { /* ... */ }
    InferenceError::RequestFailed(msg) => { /* ... */ }
    InferenceError::Cancelled => { /* ... */ }
}

// AFTER — handle new variants
match error {
    InferenceError::BackendUnavailable => { /* ... */ }
    InferenceError::ModelNotLoaded(m) => { /* ... */ }
    InferenceError::RequestFailed(msg) => { /* ... */ }
    InferenceError::Cancelled => { /* ... */ }
    InferenceError::AuthenticationFailed(msg) => {
        // 401/403 from vendor API — check credentials
    }
    InferenceError::RateLimited { retry_after_ms } => {
        // 429 from vendor API — back off and retry
        if let Some(ms) = retry_after_ms {
            tokio::time::sleep(Duration::from_millis(ms)).await;
        }
    }
    InferenceError::ProviderUnavailable(msg) => {
        // 5xx from vendor API — provider is down
    }
}
```

### Step 3: Update ChatMessage Usage

`ChatMessage` now has three optional fields for multimodal and function calling:

```rust
// BEFORE — only role and content
let msg = ChatMessage {
    role: "user".to_string(),
    content: "Hello".to_string(),
};

// AFTER — new fields default to None, existing code still compiles
let msg = ChatMessage {
    role: "user".to_string(),
    content: "Hello".to_string(),
    images: None,           // Optional: base64-encoded image data
    tool_calls: None,       // Optional: function calling responses
    tool_call_id: None,     // Optional: tool result message ID
};

// For multimodal requests
let msg = ChatMessage {
    role: "user".to_string(),
    content: "What's in this image?".to_string(),
    images: Some(vec![base64_image_data]),
    tool_calls: None,
    tool_call_id: None,
};
```

`ChatRequest` also has a new `tools` field:

```rust
let request = ChatRequest {
    execution_id: "exec-123".to_string(),
    conversation_id: "conv-456".to_string(),
    model: "gpt-4o".to_string(),
    messages: vec![msg],
    options: None,
    tools: Some(vec![ToolDefinition {
        name: "get_weather".to_string(),
        description: "Get current weather".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "location": { "type": "string" }
            }
        }),
    }]),
};
```

### Step 4: Replace Ollama with NativeRuntime

If you were using Ollama for local model inference, switch to the NativeRuntime:

```rust
// BEFORE — Ollama-based local inference
// OllamaProvider connected to localhost:11434

// AFTER — NativeRuntime-based local inference
// Install a local model component:
//   lifesavor-dev component install ./developer/components/models/llama-3-8b-instruct
//
// The component uses NativeRuntime (PyTorch + ONNX) with automatic
// hardware detection (CUDA, MPS, CoreML, DirectML, CPU fallback).
// No external Ollama process needed.
```

Key differences from Ollama:

| Aspect | Ollama (old) | NativeRuntime (new) |
|--------|-------------|-------------------|
| Process | External process on port 11434 | Embedded in agent binary |
| Backends | Ollama's runtime | PyTorch (`tch-rs`) + ONNX Runtime (`ort`) |
| Hardware | Ollama's detection | Automatic: CUDA, MPS, CoreML, DirectML, Neuron, CPU |
| State | Ollama manages | Hot/Warm/Cold state machine with keep-alive timers |
| Models | Ollama model library | HuggingFace repos with checksum verification |
| Config | Ollama config | `llm-config.yaml` with `native_runtime` section |

### Step 5: Replace Direct API Calls with Components

If you were using `OpenAiCompatibleProvider` to call OpenAI/Anthropic/etc. directly:

**For platform-managed billing (lifesavor_hosted):**
Install the corresponding API component. The component routes through the service-api gateway, which handles API keys, billing, and budget enforcement.

```bash
lifesavor-dev component install ./developer/components/models/gpt-4o
```

**For user-managed billing (BYOK):**
Install the BYOK variant. The component calls the vendor API directly with the user's own key.

```bash
lifesavor-dev component install ./developer/components/models/gpt-4o-byok
```

### Step 6: Update the InferenceBridge Usage

The `InferenceBridge` no longer falls back to the legacy `InferenceEngine`. All inference goes through registered providers:

```rust
// BEFORE — bridge with legacy fallback
// If no providers registered, falls back to InferenceEngine

// AFTER — bridge requires registered providers
// If no providers are registered, inference requests fail with
// InferenceError::BackendUnavailable
//
// Install at least one model component before attempting inference.
```

### Step 7: Update Model SDK Imports

The Model SDK no longer re-exports the removed providers:

```rust
// BEFORE
use lifesavor_model_sdk::OllamaProvider;           // Removed
use lifesavor_model_sdk::OpenAiCompatibleProvider;  // Removed
use lifesavor_model_sdk::BedrockProvider;           // Removed

// AFTER — new types available
use lifesavor_model_sdk::prelude::*;
use lifesavor_model_sdk::{
    // Core trait
    LlmProvider,
    // Extended types
    ChatMessage,        // Now with images, tool_calls, tool_call_id
    ToolCall,           // Function call response
    ToolDefinition,     // Function declaration for tool-use models
    // Extended errors
    InferenceError,     // Now with AuthenticationFailed, RateLimited, ProviderUnavailable
    // Credential management (for BYOK)
    CredentialManager,
    ResolvedCredential,
    CredentialError,
    // RAG access
    RagProvider,
    RagSearchRequest,
    // Vault/PII
    VaultDecryptParams,
    content_contains_vault_tags,
};
```

## Configuration Migration

### Old: Ollama in llm-config.yaml

```yaml
# REMOVED — no longer supported
ollama:
  endpoint: "http://localhost:11434"
  models:
    - llama2
    - codellama
```

### New: NativeRuntime in llm-config.yaml

```yaml
native_runtime:
  models_dir: "~/.local/share/lifesavor-agent/models/"
  # Or override for external storage:
  # models_dir: "/mnt/external/models/"
```

### Old: OpenAI API key in agent config

```yaml
# REMOVED — no longer supported
openai:
  api_key: "sk-..."
```

### New: API key in agent vault (BYOK only)

For BYOK components, store the API key in the agent vault. The component's manifest declares which vault keys it needs:

```toml
vault_keys = ["openai-api-key"]

[auth]
strategy = "vault"
vault_key = "openai-api-key"
```

For `lifesavor_hosted` API components, no key configuration is needed — the gateway manages keys.

## Component Equivalents

| Old Provider | New Component(s) |
|-------------|-----------------|
| `OpenAiCompatibleProvider` (GPT-4) | `gpt-4o` (API) or `gpt-4o-byok` (BYOK) |
| `OpenAiCompatibleProvider` (GPT-3.5) | `gpt-4o-mini` (API) or `gpt-4o-mini-byok` (BYOK) |
| `BedrockProvider` (Claude) | `claude-3-5-sonnet` (API) or `claude-3-5-sonnet-byok` (BYOK) |
| `OllamaProvider` (Llama 2) | `llama-3-8b-instruct` (Local/NativeRuntime) |
| `OllamaProvider` (CodeLlama) | `codellama-13b-instruct` (Local/NativeRuntime) |
| `OllamaProvider` (Mistral) | `mistral-7b-instruct` (Local/NativeRuntime) |

## Troubleshooting

### "No providers registered" error

Install at least one model component:

```bash
lifesavor-dev component list                    # Check what's installed
lifesavor-dev component install ./path/to/model # Install a component
```

### "SDK version mismatch" error

Ensure your component's `sdk_version` in the manifest matches the agent's `AGENT_SDK_VERSION` (currently `"0.5.0"`).

### "Vault key not in allowlist" error

BYOK components must declare all vault keys in the manifest's `vault_keys` array. The `CredentialManager` rejects access to undeclared keys.

### Model not loading (Local)

Check hardware requirements in the component's `model-deps.json`:

```bash
# Check available RAM vs. model requirements
lifesavor-dev component health my-model
```

The NativeRuntime selects the best quantization variant based on available RAM. If RAM is insufficient for even the smallest variant, the model won't load.
