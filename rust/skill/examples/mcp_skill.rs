//! Minimal MCP server skill example.
//!
//! Demonstrates building a skill that uses the MCP (Model Context Protocol)
//! transport: tool registration, stdio transport configuration, and
//! capability handshake via `McpTransport`.
//!
//! Run with: `cargo run --example mcp_skill --features mcp`

use std::collections::HashMap;

use lifesavor_skill_sdk::prelude::*;
use lifesavor_skill_sdk::builder::ToolSchemaBuilder;
use lifesavor_skill_sdk::{
    ConnectionConfig, HealthCheckConfig, HealthCheckMethod, Locality,
};

#[cfg(feature = "mcp")]
use lifesavor_skill_sdk::McpTransport;

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("MCP skill example — tool registration and capability handshake");

    // --- Tool registration ---
    // MCP skills declare their tools the same way as JSON stdin/stdout skills.
    let search_tool = ToolSchemaBuilder::new()
        .name("web_search")
        .description("Searches the web for a query")
        .input_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query" },
                "max_results": { "type": "integer", "default": 5 }
            },
            "required": ["query"]
        }))
        .build()
        .expect("valid tool schema");

    let fetch_tool = ToolSchemaBuilder::new()
        .name("fetch_url")
        .description("Fetches content from a URL")
        .input_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "URL to fetch" }
            },
            "required": ["url"]
        }))
        .build()
        .expect("valid tool schema");

    info!(
        tools = ?[&search_tool.name, &fetch_tool.name],
        "Registered MCP tools"
    );

    // --- MCP manifest with stdio transport ---
    // The manifest declares `transport = "stdio"` so the agent spawns the
    // skill as a child process and communicates via MCP over stdin/stdout.
    let manifest = ProviderManifest {
        provider_type: ProviderType::Skill,
        instance_name: "mcp-web-skill".to_string(),
        sdk_version: "0.1.0".to_string(),
        connection: ConnectionConfig {
            base_url: None,
            region: None,
            database_url: None,
            extension_path: None,
            command: Some("/usr/local/bin/mcp-web-skill".to_string()),
            args: Some(vec!["--mcp".to_string()]),
            transport: Some("stdio".to_string()),
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
            method: HealthCheckMethod::CapabilityProbe,
        },
        priority: 50,
        locality: Locality::Local,
        depends_on: vec![],
        capabilities: None,
        cost_limits: None,
        sandbox: Some(SandboxConfig {
            enabled: true,
            allowed_env_vars: vec!["HOME".into(), "PATH".into()],
            allowed_paths: vec!["/tmp/mcp-skill".into()],
            max_memory_mb: None,
            max_cpu_seconds: None,
            max_output_bytes: Some(5_242_880), // 5 MiB
        }),
        vault_keys: vec![],
        model_aliases: HashMap::new(),
    };

    // Validate the manifest.
    let validation = validate_manifest(&manifest, "<mcp-example>");
    match validation {
        Ok(()) => info!("Manifest validation passed"),
        Err(errors) => {
            for e in &errors {
                warn!(field = %e.field_name, desc = %e.description, "Validation error");
            }
        }
    }

    // --- Capability handshake simulation ---
    // In production, the MCP protocol starts with an `initialize` request
    // from the agent, and the skill responds with its capabilities.
    #[cfg(feature = "mcp")]
    {
        let transport = McpTransport::Stdio;
        info!(transport = ?transport, "MCP transport configured");

        // Simulate the capability handshake response.
        let capabilities = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {
                    "listChanged": false
                }
            },
            "serverInfo": {
                "name": manifest.instance_name,
                "version": manifest.sdk_version,
            }
        });
        info!("MCP initialize response:\n{}", serde_json::to_string_pretty(&capabilities).unwrap());

        // Simulate tools/list response.
        let tools_list = serde_json::json!({
            "tools": [
                {
                    "name": search_tool.name,
                    "description": search_tool.description,
                    "inputSchema": search_tool.input_schema,
                },
                {
                    "name": fetch_tool.name,
                    "description": fetch_tool.description,
                    "inputSchema": fetch_tool.input_schema,
                }
            ]
        });
        info!("MCP tools/list response:\n{}", serde_json::to_string_pretty(&tools_list).unwrap());
    }

    #[cfg(not(feature = "mcp"))]
    {
        info!("MCP feature not enabled — skipping transport-specific demo");
        info!("Re-run with: cargo run --example mcp_skill --features mcp");
    }

    // --- Lifecycle events ---
    let events = [
        ExecutionLifecycleEvent::Started,
        ExecutionLifecycleEvent::Progress,
        ExecutionLifecycleEvent::Completed,
    ];
    for event in &events {
        info!(event = ?event, "Lifecycle event");
    }

    info!("MCP skill example complete");
}
