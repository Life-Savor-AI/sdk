# Life Savor Skill SDK — AI Context

You are assisting with development of a Life Savor **skill** using the `lifesavor-skill-sdk` Rust crate.

## Architecture

Skills are sandboxed extensions that provide user-facing tools (weather, calendar, etc.). They communicate with the agent via MCP protocol over JSON stdin/stdout.

```
User → Assistant → MCP Tool Call → Skill (sandboxed) → BridgeRequest → SystemComponent
```

## Key Types

### SkillProvider trait
Implement this to create a skill. Use `SkillProviderBuilder` for convenience.

### ToolSchemaBuilder
Builds MCP-compatible tool schemas:
```rust
ToolSchemaBuilder::new("get_weather", "Get weather for a city")
    .add_string_param("city", "City name", true)
    .add_number_param("days", "Forecast days", false)
    .build()
```

### BridgeRequest / BridgeResponse
Skills access system components through bridge requests:
```rust
let response = sandbox.bridge_request(BridgeRequest {
    component: "cache".into(),
    operation: "get".into(),
    params: json!({"key": "weather:seattle"}),
    skill_id: "weather-skill".into(),
    correlation_id: ctx.correlation_id.clone(),
});
```

### Sandbox Constraints
- Skills run in a restricted sandbox
- No direct filesystem access (use bridge to FileStorage component)
- No network access (use bridge to appropriate component)
- CPU and memory limits enforced
- Permissions declared in skill manifest

## Patterns

- Use `SkillProviderBuilder` for quick skill creation
- Declare all required permissions in `skill-manifest.toml`
- Use `MockSandbox` for unit testing without a running agent
- MCP tool names follow `<skill_id>.<tool_name>` format
- Return structured JSON results from tool handlers

## Testing

- Use `MockSandbox` for isolated unit tests
- Use `proptest` for property-based testing of tool handlers
- Use `lifesavor-dev test-bridge` for integration testing against the simulator
