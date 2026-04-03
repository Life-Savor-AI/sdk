//! Local filesystem assistant provider example.
//!
//! Demonstrates a minimal assistant provider that loads `AssistantDefinition`
//! files from a local directory, implementing `load`, `list`, and `resolve`.
//!
//! Run with: `cargo run --example local_fs_provider`

use std::collections::HashMap;

use async_trait::async_trait;

use lifesavor_assistant_sdk::prelude::*;
use lifesavor_assistant_sdk::builder::AssistantDefinitionBuilder;
use lifesavor_assistant_sdk::testing::MockAssistantStore;
use lifesavor_assistant_sdk::{
    ConnectionConfig, HealthCheckConfig, HealthCheckMethod, Locality,
};

// ---------------------------------------------------------------------------
// LocalFsProvider — loads definitions from an in-memory store
// ---------------------------------------------------------------------------

/// A minimal assistant provider backed by a [`MockAssistantStore`].
///
/// In a real implementation you would read JSON/TOML files from a directory.
/// This example uses the mock store to keep things self-contained.
struct LocalFsProvider {
    store: MockAssistantStore,
    #[allow(dead_code)]
    manifest: ProviderManifest,
}

impl LocalFsProvider {
    fn new(manifest: ProviderManifest) -> Self {
        Self {
            store: MockAssistantStore::new(),
            manifest,
        }
    }

    fn add_definition(&mut self, def: AssistantDefinition) {
        self.store.add(def);
    }
}

#[async_trait]
impl AssistantProvider for LocalFsProvider {
    async fn load(&self, id: &str) -> Result<AssistantDefinition, AssistantProviderError> {
        info!(id = %id, "Loading assistant definition");
        self.store
            .load(id)
            .map(|d| d.clone())
            .map_err(|e| AssistantProviderError::NotFound(e.to_string()))
    }

    async fn list(&self) -> Result<Vec<AssistantSummary>, AssistantProviderError> {
        info!("Listing assistant definitions");
        Ok(self.store.list())
    }

    async fn resolve(&self, id: &str) -> Result<ResolvedAssistant, AssistantProviderError> {
        let def = self.load(id).await?;
        let system_prompt = substitute_variables(&def.system_prompt_template, &def.variables);
        info!(id = %id, "Resolved assistant with prompt substitution");
        Ok(ResolvedAssistant {
            resolved_tool_bindings: def.tool_bindings.clone(),
            warnings: vec![],
            definition: def,
            system_prompt,
        })
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("Local filesystem assistant provider example");

    // Build a valid assistant manifest.
    let manifest = ProviderManifest {
        provider_type: ProviderType::Assistant,
        instance_name: "local-fs-assistants".to_string(),
        sdk_version: "0.1.0".to_string(),
        connection: ConnectionConfig {
            base_url: Some("file:///etc/lifesavor/assistants".to_string()),
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
        priority: 1,
        locality: Locality::Local,
        depends_on: vec![],
        capabilities: None,
        cost_limits: None,
        sandbox: None,
        vault_keys: vec![],
        model_aliases: HashMap::new(),
    };

    // Create the provider and populate it with definitions.
    let mut provider = LocalFsProvider::new(manifest);

    let coding_assistant = AssistantDefinitionBuilder::new()
        .id("coding-assistant")
        .display_name("Coding Assistant")
        .system_prompt_template("You are {{role}}. Help with {{language}} code.")
        .variable("role", "an expert programmer")
        .variable("language", "Rust")
        .tool_binding(ToolBinding {
            skill_id: Some("code-search".to_string()),
            mcp_tool: None,
            system_component: None,
        })
        .build()
        .expect("valid definition");

    let support_assistant = AssistantDefinitionBuilder::new()
        .id("support-assistant")
        .display_name("Support Assistant")
        .system_prompt_template("You are a {{tone}} support agent for {{product}}.")
        .variable("tone", "friendly")
        .variable("product", "Life Savor")
        .handoff_config(HandoffConfig {
            target_assistant_id: "escalation-assistant".to_string(),
            trigger: "keyword".to_string(),
            keywords: vec!["escalate".to_string(), "manager".to_string()],
        })
        .build()
        .expect("valid definition");

    provider.add_definition(coding_assistant);
    provider.add_definition(support_assistant);

    // List all definitions.
    let summaries = provider.list().await.unwrap();
    info!(count = summaries.len(), "Listed assistant definitions");
    for s in &summaries {
        info!(
            id = %s.id,
            name = %s.display_name,
            tools = s.tool_binding_count,
            "  assistant"
        );
    }

    // Load a specific definition.
    let def = provider.load("coding-assistant").await.unwrap();
    info!(id = %def.id, name = %def.display_name, "Loaded definition");

    // Resolve with variable substitution.
    let resolved = provider.resolve("coding-assistant").await.unwrap();
    info!(prompt = %resolved.system_prompt, "Resolved system prompt");
    assert!(resolved.system_prompt.contains("expert programmer"));
    assert!(resolved.system_prompt.contains("Rust"));

    // Verify not-found handling.
    let err = provider.load("nonexistent").await.unwrap_err();
    info!(error = %err, "Expected not-found error");

    info!("Local filesystem assistant provider example complete");
}
