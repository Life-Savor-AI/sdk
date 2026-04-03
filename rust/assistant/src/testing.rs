//! Test harness for the Assistant SDK.
//!
//! Provides [`MockAssistantStore`] for testing [`crate::AssistantProvider`]
//! implementations in isolation without real filesystem access. The mock
//! stores [`AssistantDefinition`] objects in memory and supports `load(id)`
//! and `list()` operations.
//!
//! Also provides assertion helpers for verifying error chains, health check
//! responses, and capability descriptors.

use std::collections::HashMap;

use crate::{
    AssistantDefinition, AssistantSummary, ErrorContext, Subsystem,
};
use crate::error::AssistantSdkError;
use lifesavor_agent::providers::skill_provider::HealthStatus;

// ---------------------------------------------------------------------------
// MockAssistantStore
// ---------------------------------------------------------------------------

/// Simulates a filesystem of assistant definition files for testing
/// [`crate::AssistantProvider`] implementations.
///
/// Definitions are stored in memory keyed by their `id` field.
///
/// # Example
///
/// ```rust,ignore
/// use lifesavor_assistant_sdk::testing::MockAssistantStore;
/// use lifesavor_assistant_sdk::builder::AssistantDefinitionBuilder;
///
/// let def = AssistantDefinitionBuilder::new()
///     .id("asst-1")
///     .display_name("Test Assistant")
///     .system_prompt_template("Hello {{name}}")
///     .variable("name", "World")
///     .build()
///     .unwrap();
///
/// let mut store = MockAssistantStore::new();
/// store.add(def);
///
/// let loaded = store.load("asst-1").unwrap();
/// assert_eq!(loaded.display_name, "Test Assistant");
///
/// let summaries = store.list();
/// assert_eq!(summaries.len(), 1);
/// ```
pub struct MockAssistantStore {
    definitions: HashMap<String, AssistantDefinition>,
}

impl MockAssistantStore {
    /// Create a new empty store.
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
        }
    }

    /// Add an [`AssistantDefinition`] to the store.
    ///
    /// If a definition with the same `id` already exists, it is replaced.
    pub fn add(&mut self, definition: AssistantDefinition) {
        self.definitions
            .insert(definition.id.clone(), definition);
    }

    /// Load an [`AssistantDefinition`] by id.
    ///
    /// Returns [`AssistantSdkError::NotFound`] if no definition with the
    /// given id exists.
    pub fn load(&self, id: &str) -> Result<&AssistantDefinition, AssistantSdkError> {
        self.definitions
            .get(id)
            .ok_or_else(|| AssistantSdkError::NotFound(format!("Assistant '{id}' not found")))
    }

    /// List summaries of all stored definitions.
    pub fn list(&self) -> Vec<AssistantSummary> {
        self.definitions
            .values()
            .map(|def| AssistantSummary {
                id: def.id.clone(),
                display_name: def.display_name.clone(),
                tool_binding_count: def.tool_bindings.len(),
                guardrail_rule_count: def.guardrail_rules.len(),
            })
            .collect()
    }

    /// Return the number of stored definitions.
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Return whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Remove a definition by id. Returns the removed definition if found.
    pub fn remove(&mut self, id: &str) -> Option<AssistantDefinition> {
        self.definitions.remove(id)
    }
}

impl Default for MockAssistantStore {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

/// Assert that an [`ErrorContext`] has the expected subsystem.
pub fn assert_error_context_subsystem(ctx: &ErrorContext, expected: Subsystem) {
    assert_eq!(
        ctx.subsystem, expected,
        "Expected subsystem {:?}, got {:?}",
        expected, ctx.subsystem,
    );
}

/// Assert that a [`HealthStatus`] is [`HealthStatus::Healthy`].
pub fn assert_healthy(status: &HealthStatus) {
    assert_eq!(
        *status,
        HealthStatus::Healthy,
        "Expected Healthy, got {status:?}",
    );
}

/// Assert that a [`HealthStatus`] is NOT [`HealthStatus::Healthy`].
pub fn assert_unhealthy(status: &HealthStatus) {
    assert_ne!(
        *status,
        HealthStatus::Healthy,
        "Expected unhealthy status, got Healthy",
    );
}

/// Assert that an [`ErrorContext`] has a non-empty code and message.
pub fn assert_error_context_non_empty(ctx: &ErrorContext) {
    assert!(!ctx.code.is_empty(), "ErrorContext code must not be empty");
    assert!(
        !ctx.message.is_empty(),
        "ErrorContext message must not be empty"
    );
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::AssistantDefinitionBuilder;

    fn sample_definition(id: &str, name: &str) -> AssistantDefinition {
        AssistantDefinitionBuilder::new()
            .id(id)
            .display_name(name)
            .system_prompt_template("You are a helpful assistant.")
            .build()
            .unwrap()
    }

    #[test]
    fn new_store_is_empty() {
        let store = MockAssistantStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn add_and_load() {
        let mut store = MockAssistantStore::new();
        store.add(sample_definition("asst-1", "First"));

        let loaded = store.load("asst-1").unwrap();
        assert_eq!(loaded.id, "asst-1");
        assert_eq!(loaded.display_name, "First");
    }

    #[test]
    fn load_not_found() {
        let store = MockAssistantStore::new();
        let result = store.load("nonexistent");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AssistantSdkError::NotFound(_)));
        assert!(err.to_string().contains("nonexistent"));
    }

    #[test]
    fn list_returns_all_summaries() {
        let mut store = MockAssistantStore::new();
        store.add(sample_definition("asst-1", "First"));
        store.add(sample_definition("asst-2", "Second"));

        let summaries = store.list();
        assert_eq!(summaries.len(), 2);

        let ids: Vec<&str> = summaries.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"asst-1"));
        assert!(ids.contains(&"asst-2"));
    }

    #[test]
    fn list_empty_store() {
        let store = MockAssistantStore::new();
        assert!(store.list().is_empty());
    }

    #[test]
    fn add_replaces_existing() {
        let mut store = MockAssistantStore::new();
        store.add(sample_definition("asst-1", "Original"));
        store.add(sample_definition("asst-1", "Replaced"));

        assert_eq!(store.len(), 1);
        let loaded = store.load("asst-1").unwrap();
        assert_eq!(loaded.display_name, "Replaced");
    }

    #[test]
    fn remove_definition() {
        let mut store = MockAssistantStore::new();
        store.add(sample_definition("asst-1", "First"));
        assert_eq!(store.len(), 1);

        let removed = store.remove("asst-1");
        assert!(removed.is_some());
        assert!(store.is_empty());
    }

    #[test]
    fn summary_counts_bindings() {
        let def = AssistantDefinitionBuilder::new()
            .id("asst-1")
            .display_name("Test")
            .system_prompt_template("Hello")
            .tool_binding(crate::ToolBinding {
                skill_id: Some("calc".into()),
                mcp_tool: None,
                system_component: None,
            })
            .build()
            .unwrap();

        let mut store = MockAssistantStore::new();
        store.add(def);

        let summaries = store.list();
        assert_eq!(summaries[0].tool_binding_count, 1);
        assert_eq!(summaries[0].guardrail_rule_count, 0);
    }

    #[test]
    fn assert_healthy_passes() {
        assert_healthy(&HealthStatus::Healthy);
    }

    #[test]
    #[should_panic(expected = "Expected Healthy")]
    fn assert_healthy_panics_for_unhealthy() {
        assert_healthy(&HealthStatus::Unhealthy {
            details: "bad".into(),
        });
    }

    #[test]
    fn assert_error_context_subsystem_passes() {
        let ctx = ErrorContext::new(Subsystem::Provider, "TEST", "msg".to_string());
        assert_error_context_subsystem(&ctx, Subsystem::Provider);
    }
}
