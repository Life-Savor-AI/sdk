//! Builders for constructing [`AssistantDefinition`] instances and
//! [`AssistantProvider`] scaffold implementations.
//!
//! - [`AssistantDefinitionBuilder`] provides guided construction of
//!   [`AssistantDefinition`] with validation of required fields and template
//!   variables.
//! - [`AssistantProviderBuilder`] accepts a [`ProviderManifest`], validates
//!   that it targets the Assistant provider type, and produces a scaffold
//!   implementing [`AssistantProvider`] with `unimplemented!()` stubs.
//!
//! # Examples
//!
//! ```rust,ignore
//! use lifesavor_assistant_sdk::prelude::*;
//! use lifesavor_assistant_sdk::builder::{AssistantDefinitionBuilder, AssistantProviderBuilder};
//!
//! let definition = AssistantDefinitionBuilder::new()
//!     .id("my-assistant")
//!     .display_name("My Assistant")
//!     .system_prompt_template("You are {{role}}.")
//!     .variable("role", "a helpful assistant")
//!     .build()
//!     .expect("valid definition");
//!
//! let provider = AssistantProviderBuilder::new(manifest)
//!     .expect("valid assistant manifest")
//!     .build();
//! ```

use std::collections::HashMap;

use async_trait::async_trait;

use crate::error::AssistantSdkError;
use crate::{
    AssistantDefinition, AssistantProvider, AssistantProviderError, AssistantSummary,
    GuardrailRule, HandoffConfig, ManifestValidationError, ProviderManifest, ProviderType,
    ResolvedAssistant, ToolBinding, validate_manifest,
};

// ---------------------------------------------------------------------------
// AssistantDefinitionBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing validated [`AssistantDefinition`] instances.
///
/// Enforces that `id`, `display_name`, and `system_prompt_template` are
/// non-empty, validates that all `{{var}}` placeholders in the template
/// have corresponding entries in the variables map, and defaults
/// `context_window_strategy` to `"sliding"` if not explicitly set.
#[derive(Debug, Default)]
pub struct AssistantDefinitionBuilder {
    id: Option<String>,
    display_name: Option<String>,
    system_prompt_template: Option<String>,
    variables: HashMap<String, String>,
    model_preferences: Vec<String>,
    tool_bindings: Vec<ToolBinding>,
    guardrail_rules: Vec<GuardrailRule>,
    context_window_strategy: Option<String>,
    pipeline: Option<Vec<String>>,
    handoff_to: Option<HandoffConfig>,
}

impl AssistantDefinitionBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the assistant's unique identifier.
    pub fn id(mut self, id: &str) -> Self {
        self.id = Some(id.to_string());
        self
    }

    /// Set the assistant's human-readable display name.
    pub fn display_name(mut self, name: &str) -> Self {
        self.display_name = Some(name.to_string());
        self
    }

    /// Set the system prompt template (supports `{{variable}}` placeholders).
    pub fn system_prompt_template(mut self, template: &str) -> Self {
        self.system_prompt_template = Some(template.to_string());
        self
    }

    /// Add a template variable for prompt substitution.
    pub fn variable(mut self, key: &str, value: &str) -> Self {
        self.variables.insert(key.to_string(), value.to_string());
        self
    }

    /// Add a tool binding to the assistant.
    pub fn tool_binding(mut self, binding: ToolBinding) -> Self {
        self.tool_bindings.push(binding);
        self
    }

    /// Add a guardrail rule to the assistant.
    pub fn guardrail_rule(mut self, rule: GuardrailRule) -> Self {
        self.guardrail_rules.push(rule);
        self
    }

    /// Set the context window management strategy.
    ///
    /// Defaults to `"sliding"` if not called.
    pub fn context_window_strategy(mut self, strategy: &str) -> Self {
        self.context_window_strategy = Some(strategy.to_string());
        self
    }

    /// Add a model preference to the ordered list.
    pub fn model_preference(mut self, model: &str) -> Self {
        self.model_preferences.push(model.to_string());
        self
    }

    /// Set the handoff configuration for conversation routing.
    pub fn handoff_config(mut self, config: HandoffConfig) -> Self {
        self.handoff_to = Some(config);
        self
    }

    /// Consume the builder and produce a validated [`AssistantDefinition`].
    ///
    /// Returns [`AssistantSdkError::ValidationFailed`] if:
    /// - `id` is missing or empty
    /// - `display_name` is missing or empty
    /// - `system_prompt_template` is missing or empty
    /// - Any `{{var}}` placeholder in the template is not defined in the
    ///   variables map
    pub fn build(self) -> Result<AssistantDefinition, AssistantSdkError> {
        let id = self.id.unwrap_or_default();
        if id.is_empty() {
            return Err(AssistantSdkError::ValidationFailed(
                "Assistant definition 'id' must not be empty".to_string(),
            ));
        }

        let display_name = self.display_name.unwrap_or_default();
        if display_name.is_empty() {
            return Err(AssistantSdkError::ValidationFailed(
                "Assistant definition 'display_name' must not be empty".to_string(),
            ));
        }

        let system_prompt_template = self.system_prompt_template.unwrap_or_default();
        if system_prompt_template.is_empty() {
            return Err(AssistantSdkError::ValidationFailed(
                "Assistant definition 'system_prompt_template' must not be empty".to_string(),
            ));
        }

        // Validate that all {{var}} placeholders have defined variables.
        let mut pos = 0;
        while let Some(start) = system_prompt_template[pos..].find("{{") {
            let abs_start = pos + start + 2;
            if let Some(end) = system_prompt_template[abs_start..].find("}}") {
                let var_name = system_prompt_template[abs_start..abs_start + end].trim();
                if !var_name.is_empty() && !self.variables.contains_key(var_name) {
                    return Err(AssistantSdkError::ValidationFailed(format!(
                        "Undefined variable '{{{{{}}}}}' in system_prompt_template",
                        var_name
                    )));
                }
                pos = abs_start + end + 2;
            } else {
                break;
            }
        }

        let context_window_strategy = self
            .context_window_strategy
            .unwrap_or_else(|| "sliding".to_string());

        Ok(AssistantDefinition {
            id,
            display_name,
            system_prompt_template,
            model_preferences: self.model_preferences,
            tool_bindings: self.tool_bindings,
            guardrail_rules: self.guardrail_rules,
            context_window_strategy,
            pipeline: self.pipeline,
            handoff_to: self.handoff_to,
            variables: self.variables,
        })
    }
}

// ---------------------------------------------------------------------------
// AssistantProviderBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing an [`AssistantProvider`] scaffold from a
/// [`ProviderManifest`].
///
/// Validates that the manifest's `provider_type` is
/// [`ProviderType::Assistant`] and that the manifest passes
/// [`validate_manifest`]. On success, [`build()`](Self::build) returns a
/// scaffold struct implementing [`AssistantProvider`] with
/// `unimplemented!()` stubs for every trait method.
#[derive(Debug)]
pub struct AssistantProviderBuilder {
    manifest: ProviderManifest,
}

impl AssistantProviderBuilder {
    /// Create a new builder from a [`ProviderManifest`].
    ///
    /// Returns an error if:
    /// - `manifest.provider_type` is not [`ProviderType::Assistant`]
    /// - [`validate_manifest`] reports validation errors
    pub fn new(manifest: ProviderManifest) -> Result<Self, AssistantSdkError> {
        if manifest.provider_type != ProviderType::Assistant {
            return Err(AssistantSdkError::ManifestValidation(
                ManifestValidationError {
                    file_path: String::new(),
                    field_name: "provider_type".to_string(),
                    description: format!(
                        "Assistant SDK requires provider_type 'assistant', got '{}'",
                        manifest.provider_type,
                    ),
                },
            ));
        }

        if let Err(errors) = validate_manifest(&manifest, "<builder>") {
            if let Some(first) = errors.into_iter().next() {
                return Err(AssistantSdkError::ManifestValidation(first));
            }
        }

        Ok(Self { manifest })
    }

    /// Consume the builder and produce a scaffold [`AssistantProvider`]
    /// implementation.
    ///
    /// All trait methods are stubbed with `unimplemented!()`. Replace each
    /// stub with your real implementation incrementally.
    pub fn build(self) -> ScaffoldAssistantProvider {
        ScaffoldAssistantProvider {
            manifest: self.manifest,
        }
    }
}

/// Scaffold [`AssistantProvider`] produced by
/// [`AssistantProviderBuilder::build`].
///
/// Every trait method panics with `unimplemented!()`. Replace each stub
/// with your real implementation incrementally.
pub struct ScaffoldAssistantProvider {
    /// The validated provider manifest.
    pub manifest: ProviderManifest,
}

#[async_trait]
impl AssistantProvider for ScaffoldAssistantProvider {
    async fn load(&self, _id: &str) -> Result<AssistantDefinition, AssistantProviderError> {
        unimplemented!("load: replace this stub with your implementation")
    }

    async fn list(&self) -> Result<Vec<AssistantSummary>, AssistantProviderError> {
        unimplemented!("list: replace this stub with your implementation")
    }

    async fn resolve(&self, _id: &str) -> Result<ResolvedAssistant, AssistantProviderError> {
        unimplemented!("resolve: replace this stub with your implementation")
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

    /// Helper to build a valid Assistant manifest for testing.
    fn valid_assistant_manifest() -> ProviderManifest {
        ProviderManifest {
            provider_type: ProviderType::Assistant,
            instance_name: "test-assistant-provider".to_string(),
            sdk_version: "0.1.0".to_string(),
            connection: ConnectionConfig {
                base_url: Some("file:///assistants".to_string()),
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
            priority: 1,
            locality: Locality::Local,
            depends_on: vec![],
            capabilities: None,
            cost_limits: None,
            sandbox: None,
            vault_keys: vec![],
            model_aliases: HashMap::new(),
        }
    }

    // -- AssistantDefinitionBuilder tests --

    #[test]
    fn builder_produces_valid_definition() {
        let def = AssistantDefinitionBuilder::new()
            .id("my-assistant")
            .display_name("My Assistant")
            .system_prompt_template("You are {{role}}.")
            .variable("role", "a helpful bot")
            .build()
            .unwrap();

        assert_eq!(def.id, "my-assistant");
        assert_eq!(def.display_name, "My Assistant");
        assert_eq!(def.system_prompt_template, "You are {{role}}.");
        assert_eq!(def.context_window_strategy, "sliding");
        assert_eq!(def.variables.get("role").unwrap(), "a helpful bot");
    }

    #[test]
    fn builder_defaults_context_window_strategy_to_sliding() {
        let def = AssistantDefinitionBuilder::new()
            .id("a")
            .display_name("A")
            .system_prompt_template("Hello")
            .build()
            .unwrap();
        assert_eq!(def.context_window_strategy, "sliding");
    }

    #[test]
    fn builder_allows_custom_context_window_strategy() {
        let def = AssistantDefinitionBuilder::new()
            .id("a")
            .display_name("A")
            .system_prompt_template("Hello")
            .context_window_strategy("summary")
            .build()
            .unwrap();
        assert_eq!(def.context_window_strategy, "summary");
    }

    #[test]
    fn builder_rejects_missing_id() {
        let err = AssistantDefinitionBuilder::new()
            .display_name("A")
            .system_prompt_template("Hello")
            .build()
            .unwrap_err();
        assert!(err.to_string().contains("'id' must not be empty"));
    }

    #[test]
    fn builder_rejects_empty_id() {
        let err = AssistantDefinitionBuilder::new()
            .id("")
            .display_name("A")
            .system_prompt_template("Hello")
            .build()
            .unwrap_err();
        assert!(err.to_string().contains("'id' must not be empty"));
    }

    #[test]
    fn builder_rejects_missing_display_name() {
        let err = AssistantDefinitionBuilder::new()
            .id("a")
            .system_prompt_template("Hello")
            .build()
            .unwrap_err();
        assert!(err.to_string().contains("'display_name' must not be empty"));
    }

    #[test]
    fn builder_rejects_missing_system_prompt_template() {
        let err = AssistantDefinitionBuilder::new()
            .id("a")
            .display_name("A")
            .build()
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("'system_prompt_template' must not be empty")
        );
    }

    #[test]
    fn builder_rejects_undefined_template_variable() {
        let err = AssistantDefinitionBuilder::new()
            .id("a")
            .display_name("A")
            .system_prompt_template("Hello {{name}}, welcome to {{place}}")
            .variable("name", "Alice")
            // "place" is not defined
            .build()
            .unwrap_err();
        assert!(err.to_string().contains("Undefined variable"));
        assert!(err.to_string().contains("place"));
    }

    #[test]
    fn builder_accepts_template_with_all_variables_defined() {
        let def = AssistantDefinitionBuilder::new()
            .id("a")
            .display_name("A")
            .system_prompt_template("{{greeting}} {{name}}")
            .variable("greeting", "Hello")
            .variable("name", "World")
            .build()
            .unwrap();
        assert_eq!(def.variables.len(), 2);
    }

    #[test]
    fn builder_accepts_template_without_variables() {
        let def = AssistantDefinitionBuilder::new()
            .id("a")
            .display_name("A")
            .system_prompt_template("No variables here")
            .build()
            .unwrap();
        assert!(def.variables.is_empty());
    }

    #[test]
    fn builder_adds_tool_bindings() {
        let def = AssistantDefinitionBuilder::new()
            .id("a")
            .display_name("A")
            .system_prompt_template("Hello")
            .tool_binding(ToolBinding {
                skill_id: Some("search".to_string()),
                mcp_tool: None,
                system_component: None,
            })
            .build()
            .unwrap();
        assert_eq!(def.tool_bindings.len(), 1);
    }

    #[test]
    fn builder_adds_guardrail_rules() {
        let def = AssistantDefinitionBuilder::new()
            .id("a")
            .display_name("A")
            .system_prompt_template("Hello")
            .guardrail_rule(GuardrailRule {
                id: "no-pii".to_string(),
                description: "No PII".to_string(),
                rule_type: "content_filter".to_string(),
                config: HashMap::new(),
            })
            .build()
            .unwrap();
        assert_eq!(def.guardrail_rules.len(), 1);
    }

    #[test]
    fn builder_adds_model_preferences() {
        let def = AssistantDefinitionBuilder::new()
            .id("a")
            .display_name("A")
            .system_prompt_template("Hello")
            .model_preference("llama3")
            .model_preference("mistral")
            .build()
            .unwrap();
        assert_eq!(def.model_preferences, vec!["llama3", "mistral"]);
    }

    #[test]
    fn builder_sets_handoff_config() {
        let def = AssistantDefinitionBuilder::new()
            .id("a")
            .display_name("A")
            .system_prompt_template("Hello")
            .handoff_config(HandoffConfig {
                target_assistant_id: "support".to_string(),
                trigger: "keyword".to_string(),
                keywords: vec!["help".to_string()],
            })
            .build()
            .unwrap();
        assert!(def.handoff_to.is_some());
        assert_eq!(
            def.handoff_to.unwrap().target_assistant_id,
            "support"
        );
    }

    // -- AssistantProviderBuilder tests --

    #[test]
    fn provider_builder_accepts_valid_assistant_manifest() {
        let manifest = valid_assistant_manifest();
        let builder = AssistantProviderBuilder::new(manifest);
        assert!(builder.is_ok());
    }

    #[test]
    fn provider_builder_rejects_llm_provider_type() {
        let mut manifest = valid_assistant_manifest();
        manifest.provider_type = ProviderType::Llm;
        let err = AssistantProviderBuilder::new(manifest).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("provider_type"), "error should mention provider_type: {msg}");
        assert!(msg.contains("assistant"), "error should mention expected type: {msg}");
    }

    #[test]
    fn provider_builder_rejects_skill_provider_type() {
        let mut manifest = valid_assistant_manifest();
        manifest.provider_type = ProviderType::Skill;
        let err = AssistantProviderBuilder::new(manifest).unwrap_err();
        assert!(err.to_string().contains("provider_type"));
    }

    #[test]
    fn provider_builder_rejects_vector_store_provider_type() {
        let mut manifest = valid_assistant_manifest();
        manifest.provider_type = ProviderType::VectorStore;
        let err = AssistantProviderBuilder::new(manifest).unwrap_err();
        assert!(err.to_string().contains("provider_type"));
    }

    #[test]
    fn provider_builder_rejects_empty_instance_name() {
        let mut manifest = valid_assistant_manifest();
        manifest.instance_name = String::new();
        let err = AssistantProviderBuilder::new(manifest).unwrap_err();
        assert!(err.to_string().contains("instance_name"));
    }

    #[test]
    fn provider_builder_rejects_empty_sdk_version() {
        let mut manifest = valid_assistant_manifest();
        manifest.sdk_version = String::new();
        let err = AssistantProviderBuilder::new(manifest).unwrap_err();
        assert!(err.to_string().contains("sdk_version"));
    }

    #[test]
    fn provider_builder_build_returns_scaffold() {
        let manifest = valid_assistant_manifest();
        let provider = AssistantProviderBuilder::new(manifest.clone())
            .unwrap()
            .build();
        assert_eq!(provider.manifest.instance_name, "test-assistant-provider");
    }
}
