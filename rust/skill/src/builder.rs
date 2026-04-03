//! Builders for constructing [`SkillProvider`] scaffold implementations and
//! [`ToolSchema`] instances.
//!
//! - [`SkillProviderBuilder`] accepts a [`ProviderManifest`], validates that
//!   it targets the Skill provider type, and produces a scaffold implementing
//!   [`SkillProvider`] with `unimplemented!()` stubs.
//! - [`ToolSchemaBuilder`] provides guided construction of [`ToolSchema`]
//!   instances with validation of required fields and JSON Schema input.
//!
//! # Examples
//!
//! ```rust,ignore
//! use lifesavor_skill_sdk::prelude::*;
//! use lifesavor_skill_sdk::builder::{SkillProviderBuilder, ToolSchemaBuilder};
//!
//! let tool = ToolSchemaBuilder::new()
//!     .name("greet")
//!     .description("Greets a user by name")
//!     .input_schema(serde_json::json!({
//!         "type": "object",
//!         "properties": { "name": { "type": "string" } }
//!     }))
//!     .build()
//!     .expect("valid tool schema");
//!
//! let provider = SkillProviderBuilder::new(manifest)
//!     .expect("valid skill manifest")
//!     .tool(tool)
//!     .build();
//! ```

use async_trait::async_trait;
use serde_json::Value;

use crate::error::SkillSdkError;
use crate::{
    HealthStatus, ManifestValidationError, ProviderManifest, ProviderType,
    SkillCapabilityDescriptor, SkillProviderError, SkillProvider, ToolSchema,
    validate_manifest,
};
use lifesavor_agent::skills::SkillExecutionResult;

// ---------------------------------------------------------------------------
// ToolSchemaBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing validated [`ToolSchema`] instances.
///
/// Enforces that `name` and `description` are non-empty and that
/// `input_schema` is a JSON object value.
#[derive(Debug, Default)]
pub struct ToolSchemaBuilder {
    name: Option<String>,
    description: Option<String>,
    input_schema: Option<Value>,
    output_schema: Option<Value>,
}

impl ToolSchemaBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the tool's unique name.
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Set the tool's human-readable description.
    pub fn description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    /// Set the JSON Schema for the tool's input parameters.
    pub fn input_schema(mut self, schema: Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    /// Set the JSON Schema for the tool's output (optional).
    pub fn output_schema(mut self, schema: Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// Consume the builder and produce a validated [`ToolSchema`].
    ///
    /// Returns [`SkillSdkError::ToolSchemaInvalid`] if:
    /// - `name` is missing or empty
    /// - `description` is missing or empty
    /// - `input_schema` is provided but is not a JSON object
    pub fn build(self) -> Result<ToolSchema, SkillSdkError> {
        let name = self.name.unwrap_or_default();
        if name.is_empty() {
            return Err(SkillSdkError::ToolSchemaInvalid(
                "Tool schema 'name' must not be empty".to_string(),
            ));
        }

        let description = self.description.unwrap_or_default();
        if description.is_empty() {
            return Err(SkillSdkError::ToolSchemaInvalid(
                "Tool schema 'description' must not be empty".to_string(),
            ));
        }

        let input_schema = self.input_schema.unwrap_or_else(|| {
            serde_json::json!({ "type": "object" })
        });

        // Validate that input_schema is a JSON object.
        if !input_schema.is_object() {
            return Err(SkillSdkError::ToolSchemaInvalid(
                "Tool schema 'input_schema' must be a JSON object".to_string(),
            ));
        }

        Ok(ToolSchema {
            name,
            description,
            input_schema,
            output_schema: self.output_schema,
        })
    }
}

// ---------------------------------------------------------------------------
// SkillProviderBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing a [`SkillProvider`] scaffold from a
/// [`ProviderManifest`].
///
/// Validates that the manifest's `provider_type` is [`ProviderType::Skill`]
/// and that the manifest passes [`validate_manifest`]. Tool schemas can be
/// added via [`tool()`](Self::tool). On success, [`build()`](Self::build)
/// returns a scaffold struct implementing [`SkillProvider`] with
/// `unimplemented!()` stubs for every trait method.
#[derive(Debug)]
pub struct SkillProviderBuilder {
    manifest: ProviderManifest,
    tools: Vec<ToolSchema>,
}

impl SkillProviderBuilder {
    /// Create a new builder from a [`ProviderManifest`].
    ///
    /// Returns an error if:
    /// - `manifest.provider_type` is not [`ProviderType::Skill`]
    /// - [`validate_manifest`] reports validation errors
    pub fn new(manifest: ProviderManifest) -> Result<Self, SkillSdkError> {
        if manifest.provider_type != ProviderType::Skill {
            return Err(SkillSdkError::ManifestValidation(ManifestValidationError {
                file_path: String::new(),
                field_name: "provider_type".to_string(),
                description: format!(
                    "Skill SDK requires provider_type 'skill', got '{}'",
                    manifest.provider_type,
                ),
            }));
        }

        if let Err(errors) = validate_manifest(&manifest, "<builder>") {
            if let Some(first) = errors.into_iter().next() {
                return Err(SkillSdkError::ManifestValidation(first));
            }
        }

        Ok(Self {
            manifest,
            tools: Vec::new(),
        })
    }

    /// Add a tool schema to the skill provider.
    pub fn tool(mut self, schema: ToolSchema) -> Self {
        self.tools.push(schema);
        self
    }

    /// Consume the builder and produce a scaffold [`SkillProvider`]
    /// implementation.
    ///
    /// All trait methods are stubbed with `unimplemented!()` except
    /// `list_tools` which returns the registered tool schemas. The returned
    /// struct stores the manifest and tools for reference.
    pub fn build(self) -> ScaffoldSkillProvider {
        ScaffoldSkillProvider {
            manifest: self.manifest,
            tools: self.tools,
        }
    }
}

/// Scaffold [`SkillProvider`] produced by [`SkillProviderBuilder::build`].
///
/// `list_tools` returns the tool schemas registered via the builder.
/// All other trait methods panic with `unimplemented!()`. Replace each stub
/// with your real implementation incrementally.
pub struct ScaffoldSkillProvider {
    /// The validated provider manifest.
    pub manifest: ProviderManifest,
    /// Tool schemas registered via the builder.
    pub tools: Vec<ToolSchema>,
}

#[async_trait]
impl SkillProvider for ScaffoldSkillProvider {
    async fn invoke(
        &self,
        _operation: &str,
        _payload: Value,
    ) -> Result<SkillExecutionResult, SkillProviderError> {
        unimplemented!("invoke: replace this stub with your JSON stdin/stdout implementation")
    }

    async fn list_tools(&self) -> Result<Vec<ToolSchema>, SkillProviderError> {
        Ok(self.tools.clone())
    }

    async fn health_check(&self) -> HealthStatus {
        unimplemented!("health_check: replace this stub with your implementation")
    }

    fn capability_descriptor(&self) -> SkillCapabilityDescriptor {
        unimplemented!("capability_descriptor: replace this stub with your implementation")
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AuthConfig, ConnectionConfig, CredentialSource, HealthCheckConfig,
        HealthCheckMethod, Locality,
    };
    use std::collections::HashMap;

    /// Helper to build a valid Skill manifest for testing.
    fn valid_skill_manifest() -> ProviderManifest {
        ProviderManifest {
            provider_type: ProviderType::Skill,
            instance_name: "test-skill".to_string(),
            sdk_version: "0.1.0".to_string(),
            connection: ConnectionConfig {
                base_url: None,
                region: None,
                database_url: None,
                extension_path: None,
                command: Some("/usr/bin/my-skill".to_string()),
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
        }
    }

    // -- ToolSchemaBuilder tests --

    #[test]
    fn tool_builder_produces_valid_schema() {
        let schema = ToolSchemaBuilder::new()
            .name("greet")
            .description("Greets a user")
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": { "name": { "type": "string" } }
            }))
            .build()
            .unwrap();

        assert_eq!(schema.name, "greet");
        assert_eq!(schema.description, "Greets a user");
        assert!(schema.input_schema.is_object());
        assert!(schema.output_schema.is_none());
    }

    #[test]
    fn tool_builder_defaults_input_schema_to_empty_object() {
        let schema = ToolSchemaBuilder::new()
            .name("ping")
            .description("Pings the service")
            .build()
            .unwrap();

        assert_eq!(schema.input_schema, serde_json::json!({ "type": "object" }));
    }

    #[test]
    fn tool_builder_accepts_output_schema() {
        let schema = ToolSchemaBuilder::new()
            .name("calc")
            .description("Calculates")
            .output_schema(serde_json::json!({ "type": "number" }))
            .build()
            .unwrap();

        assert!(schema.output_schema.is_some());
    }

    #[test]
    fn tool_builder_rejects_missing_name() {
        let err = ToolSchemaBuilder::new()
            .description("A tool")
            .build()
            .unwrap_err();
        assert!(err.to_string().contains("'name' must not be empty"));
    }

    #[test]
    fn tool_builder_rejects_empty_name() {
        let err = ToolSchemaBuilder::new()
            .name("")
            .description("A tool")
            .build()
            .unwrap_err();
        assert!(err.to_string().contains("'name' must not be empty"));
    }

    #[test]
    fn tool_builder_rejects_missing_description() {
        let err = ToolSchemaBuilder::new()
            .name("tool")
            .build()
            .unwrap_err();
        assert!(err.to_string().contains("'description' must not be empty"));
    }

    #[test]
    fn tool_builder_rejects_empty_description() {
        let err = ToolSchemaBuilder::new()
            .name("tool")
            .description("")
            .build()
            .unwrap_err();
        assert!(err.to_string().contains("'description' must not be empty"));
    }

    #[test]
    fn tool_builder_rejects_non_object_input_schema() {
        let err = ToolSchemaBuilder::new()
            .name("tool")
            .description("A tool")
            .input_schema(serde_json::json!("not an object"))
            .build()
            .unwrap_err();
        assert!(err.to_string().contains("'input_schema' must be a JSON object"));
    }

    #[test]
    fn tool_builder_rejects_array_input_schema() {
        let err = ToolSchemaBuilder::new()
            .name("tool")
            .description("A tool")
            .input_schema(serde_json::json!([1, 2, 3]))
            .build()
            .unwrap_err();
        assert!(err.to_string().contains("'input_schema' must be a JSON object"));
    }

    // -- SkillProviderBuilder tests --

    #[test]
    fn provider_builder_accepts_valid_skill_manifest() {
        let manifest = valid_skill_manifest();
        let builder = SkillProviderBuilder::new(manifest);
        assert!(builder.is_ok());
    }

    #[test]
    fn provider_builder_rejects_llm_provider_type() {
        let mut manifest = valid_skill_manifest();
        manifest.provider_type = ProviderType::Llm;
        let err = SkillProviderBuilder::new(manifest).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("provider_type"), "error should mention provider_type: {msg}");
        assert!(msg.contains("skill"), "error should mention expected type: {msg}");
    }

    #[test]
    fn provider_builder_rejects_assistant_provider_type() {
        let mut manifest = valid_skill_manifest();
        manifest.provider_type = ProviderType::Assistant;
        let err = SkillProviderBuilder::new(manifest).unwrap_err();
        assert!(err.to_string().contains("provider_type"));
    }

    #[test]
    fn provider_builder_rejects_vector_store_provider_type() {
        let mut manifest = valid_skill_manifest();
        manifest.provider_type = ProviderType::VectorStore;
        let err = SkillProviderBuilder::new(manifest).unwrap_err();
        assert!(err.to_string().contains("provider_type"));
    }

    #[test]
    fn provider_builder_rejects_empty_instance_name() {
        let mut manifest = valid_skill_manifest();
        manifest.instance_name = String::new();
        let err = SkillProviderBuilder::new(manifest).unwrap_err();
        assert!(err.to_string().contains("instance_name"));
    }

    #[test]
    fn provider_builder_rejects_empty_sdk_version() {
        let mut manifest = valid_skill_manifest();
        manifest.sdk_version = String::new();
        let err = SkillProviderBuilder::new(manifest).unwrap_err();
        assert!(err.to_string().contains("sdk_version"));
    }

    #[test]
    fn provider_builder_rejects_missing_command() {
        let mut manifest = valid_skill_manifest();
        manifest.connection.command = None;
        let err = SkillProviderBuilder::new(manifest).unwrap_err();
        assert!(err.to_string().contains("command"));
    }

    #[test]
    fn provider_builder_build_returns_scaffold_with_manifest() {
        let manifest = valid_skill_manifest();
        let provider = SkillProviderBuilder::new(manifest.clone())
            .unwrap()
            .build();
        assert_eq!(provider.manifest.instance_name, "test-skill");
    }

    #[test]
    fn provider_builder_adds_tools() {
        let manifest = valid_skill_manifest();
        let tool = ToolSchemaBuilder::new()
            .name("greet")
            .description("Greets")
            .build()
            .unwrap();

        let provider = SkillProviderBuilder::new(manifest)
            .unwrap()
            .tool(tool)
            .build();

        assert_eq!(provider.tools.len(), 1);
        assert_eq!(provider.tools[0].name, "greet");
    }

    #[tokio::test]
    async fn scaffold_list_tools_returns_registered_tools() {
        let manifest = valid_skill_manifest();
        let tool = ToolSchemaBuilder::new()
            .name("search")
            .description("Searches")
            .build()
            .unwrap();

        let provider = SkillProviderBuilder::new(manifest)
            .unwrap()
            .tool(tool)
            .build();

        let tools = provider.list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "search");
    }
}
