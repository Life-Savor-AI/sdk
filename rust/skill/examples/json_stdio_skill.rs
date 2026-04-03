//! Minimal JSON stdin/stdout skill example.
//!
//! Demonstrates building a skill provider using the JSON stdin/stdout
//! protocol: tool schema declaration, input parsing, output formatting,
//! and execution lifecycle events.
//!
//! Run with: `cargo run --example json_stdio_skill`

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use lifesavor_skill_sdk::prelude::*;
use lifesavor_skill_sdk::builder::{SkillProviderBuilder, ToolSchemaBuilder};
use lifesavor_skill_sdk::{
    ConnectionConfig, HealthCheckConfig, HealthCheckMethod, Locality,
};
use lifesavor_agent::skills::SkillExecutionResult;

// ---------------------------------------------------------------------------
// Minimal JSON stdin/stdout skill
// ---------------------------------------------------------------------------

/// A minimal skill that echoes input back with a greeting.
///
/// In production, the agent spawns this as a child process and
/// communicates via JSON over stdin/stdout.
struct GreeterSkill {
    tools: Vec<ToolSchema>,
}

#[async_trait]
impl SkillProvider for GreeterSkill {
    async fn invoke(
        &self,
        operation: &str,
        payload: Value,
    ) -> Result<SkillExecutionResult, SkillProviderError> {
        info!(operation = %operation, "Skill invoked");

        // Emit lifecycle event: Started
        let _started = ExecutionLifecycleEvent::Started;
        debug!(event = ?_started, "Lifecycle: execution started");

        // Parse input from the JSON payload.
        let name = payload
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("World");

        // Format output as JSON.
        let result = serde_json::json!({
            "greeting": format!("Hello, {name}!"),
            "operation": operation,
        });

        // Emit lifecycle event: Completed
        let _completed = ExecutionLifecycleEvent::Completed;
        debug!(event = ?_completed, "Lifecycle: execution completed");

        Ok(SkillExecutionResult {
            status: "success".to_string(),
            reason_code: None,
            result: Some(result),
            error: None,
            duration_ms: 1,
            exit_code: Some(0),
            stdout_bytes: 0,
        })
    }

    async fn list_tools(&self) -> Result<Vec<ToolSchema>, SkillProviderError> {
        Ok(self.tools.clone())
    }

    async fn health_check(&self) -> HealthStatus {
        HealthStatus::Healthy
    }

    fn capability_descriptor(&self) -> SkillCapabilityDescriptor {
        SkillCapabilityDescriptor {
            tools: self.tools.clone(),
            supported_formats: vec!["json".to_string()],
            max_timeout_seconds: 30,
            max_memory_bytes: None,
            locality: Locality::Local,
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("JSON stdin/stdout skill example");

    // Build tool schemas using ToolSchemaBuilder.
    let greet_tool = ToolSchemaBuilder::new()
        .name("greet")
        .description("Greets a user by name")
        .input_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Name to greet" }
            },
            "required": ["name"]
        }))
        .build()
        .expect("valid tool schema");

    info!(tool = %greet_tool.name, "Tool schema built");

    // Build a valid Skill manifest.
    let manifest = ProviderManifest {
        provider_type: ProviderType::Skill,
        instance_name: "greeter-skill".to_string(),
        sdk_version: "0.1.0".to_string(),
        connection: ConnectionConfig {
            base_url: None,
            region: None,
            database_url: None,
            extension_path: None,
            command: Some("/usr/local/bin/greeter-skill".to_string()),
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
        priority: 50,
        locality: Locality::Local,
        depends_on: vec![],
        capabilities: None,
        cost_limits: None,
        sandbox: None,
        vault_keys: vec![],
        model_aliases: HashMap::new(),
    };

    // Validate the manifest via the builder.
    let _scaffold = SkillProviderBuilder::new(manifest)
        .expect("valid skill manifest")
        .tool(greet_tool.clone())
        .build();
    info!("SkillProviderBuilder accepted the manifest");

    // Use our custom implementation.
    let skill = GreeterSkill {
        tools: vec![greet_tool],
    };

    // List tools.
    let tools = skill.list_tools().await.unwrap();
    info!(count = tools.len(), "Registered tools");
    for t in &tools {
        info!(name = %t.name, desc = %t.description, "  tool");
    }

    // Invoke the skill with a JSON payload.
    let payload = serde_json::json!({ "name": "Alice" });
    let result = skill.invoke("greet", payload).await.unwrap();
    info!(
        status = %result.status,
        result = ?result.result,
        "Invocation result"
    );

    // Health check.
    let status = skill.health_check().await;
    info!(status = ?status, "Health check");

    // Capability descriptor.
    let caps = skill.capability_descriptor();
    info!(
        tools = caps.tools.len(),
        formats = ?caps.supported_formats,
        locality = ?caps.locality,
        "Capability descriptor"
    );

    info!("JSON stdin/stdout skill example complete");
}
