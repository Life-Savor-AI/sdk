# Life Savor Agent Types — AI Context

You are assisting with development using `lifesavor-agent-types`, the shared type library for the Life Savor agent platform.

## Purpose

This crate defines all shared types used across the agent, SDKs, and components. It is the single source of truth for data contracts.

## Key Types

### SystemComponentType
```rust
pub enum SystemComponentType {
    Tts, Stt, Cache, Identity, FileStorage, Messaging,
    Calendar, DeviceControl, MediaProcessing, UserNotifications,
    Llm, MemoryStore, EventStore,
}
```
Serializes as `snake_case` (e.g., `"memory_store"`).

### ComponentDeclaration
```rust
pub struct ComponentDeclaration {
    pub component_id: String,
    pub component_type: SystemComponentType,
    pub instance_id: Option<String>,
    pub exposed_operations: Vec<String>,
    pub publish_topics: Vec<String>,
    pub requested_access: Vec<AccessRequest>,
    pub requested_vault_keys: Vec<String>,
    pub tool_schemas: Vec<ToolSchema>,
}
```

### ErrorChain
```rust
pub struct ErrorChain {
    pub subsystem: Subsystem,
    pub code: String,
    pub message: String,
    pub correlation_id: Option<String>,
    pub cause: Option<Box<ErrorChain>>,
}
```

### ProviderManifest
TOML-serializable manifest for component/skill providers. Fields: `provider_type`, `instance_name`, `sdk_version`, `connection`, `auth`, `health_check`.

### ProviderType
```rust
pub enum ProviderType { Llm, MemoryStore, Skill, Assistant }
```

## Conventions

- All enums use `#[serde(rename_all = "snake_case")]`
- Types derive `Debug, Clone, Serialize, Deserialize`
- Use `proptest` for serde round-trip property tests
- Instance IDs follow `<type>:<name>` format (e.g., `memory_store:sqlite-vec-rag`)
