//! Test harness for the Model SDK.
//!
//! Provides [`MockRegistry`] for testing [`crate::LlmProvider`] implementations
//! in isolation without a running agent. The mock simulates provider
//! registration, health monitoring, and routing by provider type.
//!
//! Also provides assertion helpers for verifying error chains, health check
//! responses, and capability descriptors.

use std::collections::HashMap;

use crate::{
    CapabilityDescriptor, ErrorContext, ProviderType, Subsystem,
};
use lifesavor_agent::providers::skill_provider::HealthStatus;

// ---------------------------------------------------------------------------
// MockRegistry
// ---------------------------------------------------------------------------

/// Entry stored in the [`MockRegistry`] for each registered provider.
#[derive(Debug, Clone)]
pub struct MockProviderEntry {
    /// The instance name of the provider.
    pub instance_name: String,
    /// The provider type (should be [`ProviderType::Llm`] for Model SDK).
    pub provider_type: ProviderType,
    /// Current health status.
    pub health_status: HealthStatus,
}

/// Simulates the agent's Integration Registry for testing LLM provider
/// implementations.
///
/// Supports registering providers by type, querying by type, and tracking
/// health status per provider.
///
/// # Example
///
/// ```rust,ignore
/// use lifesavor_model_sdk::testing::MockRegistry;
/// use lifesavor_model_sdk::ProviderType;
///
/// let mut registry = MockRegistry::new();
/// registry.register("my-ollama", ProviderType::Llm);
/// assert!(registry.get_by_type(ProviderType::Llm).is_some());
/// ```
pub struct MockRegistry {
    providers: HashMap<String, MockProviderEntry>,
}

impl MockRegistry {
    /// Create a new empty mock registry.
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a provider with the given instance name and type.
    ///
    /// The provider starts with [`HealthStatus::Healthy`].
    pub fn register(&mut self, instance_name: &str, provider_type: ProviderType) {
        self.providers.insert(
            instance_name.to_string(),
            MockProviderEntry {
                instance_name: instance_name.to_string(),
                provider_type,
                health_status: HealthStatus::Healthy,
            },
        );
    }

    /// Query for a provider by type. Returns the first matching entry.
    pub fn get_by_type(&self, provider_type: ProviderType) -> Option<&MockProviderEntry> {
        self.providers
            .values()
            .find(|e| e.provider_type == provider_type)
    }

    /// Query for all providers of a given type.
    pub fn get_all_by_type(&self, provider_type: ProviderType) -> Vec<&MockProviderEntry> {
        self.providers
            .values()
            .filter(|e| e.provider_type == provider_type)
            .collect()
    }

    /// Query for a provider by instance name.
    pub fn get_by_name(&self, instance_name: &str) -> Option<&MockProviderEntry> {
        self.providers.get(instance_name)
    }

    /// Update the health status of a registered provider.
    ///
    /// Returns `true` if the provider was found and updated, `false` otherwise.
    pub fn set_health(&mut self, instance_name: &str, status: HealthStatus) -> bool {
        if let Some(entry) = self.providers.get_mut(instance_name) {
            entry.health_status = status;
            true
        } else {
            false
        }
    }

    /// Return the health status of a registered provider.
    pub fn health_of(&self, instance_name: &str) -> Option<&HealthStatus> {
        self.providers.get(instance_name).map(|e| &e.health_status)
    }

    /// Return the number of registered providers.
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Return whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// Remove a provider by instance name. Returns the removed entry if found.
    pub fn unregister(&mut self, instance_name: &str) -> Option<MockProviderEntry> {
        self.providers.remove(instance_name)
    }
}

impl Default for MockRegistry {
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

/// Assert that a [`CapabilityDescriptor`] has at least one capability entry.
pub fn assert_has_capabilities(descriptor: &CapabilityDescriptor) {
    assert!(
        !descriptor.models.is_empty(),
        "CapabilityDescriptor should have at least one model capability"
    );
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_registry_is_empty() {
        let registry = MockRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn register_and_query_by_type() {
        let mut registry = MockRegistry::new();
        registry.register("ollama-1", ProviderType::Llm);
        assert_eq!(registry.len(), 1);

        let entry = registry.get_by_type(ProviderType::Llm).unwrap();
        assert_eq!(entry.instance_name, "ollama-1");
        assert_eq!(entry.provider_type, ProviderType::Llm);
        assert_eq!(entry.health_status, HealthStatus::Healthy);
    }

    #[test]
    fn query_missing_type_returns_none() {
        let mut registry = MockRegistry::new();
        registry.register("ollama-1", ProviderType::Llm);
        assert!(registry.get_by_type(ProviderType::Skill).is_none());
    }

    #[test]
    fn query_by_name() {
        let mut registry = MockRegistry::new();
        registry.register("ollama-1", ProviderType::Llm);
        assert!(registry.get_by_name("ollama-1").is_some());
        assert!(registry.get_by_name("nonexistent").is_none());
    }

    #[test]
    fn set_health_updates_status() {
        let mut registry = MockRegistry::new();
        registry.register("ollama-1", ProviderType::Llm);

        let updated = registry.set_health(
            "ollama-1",
            HealthStatus::Unhealthy {
                details: "down".into(),
            },
        );
        assert!(updated);

        let status = registry.health_of("ollama-1").unwrap();
        assert!(matches!(status, HealthStatus::Unhealthy { .. }));
    }

    #[test]
    fn set_health_missing_provider_returns_false() {
        let mut registry = MockRegistry::new();
        assert!(!registry.set_health("nope", HealthStatus::Healthy));
    }

    #[test]
    fn get_all_by_type() {
        let mut registry = MockRegistry::new();
        registry.register("llm-1", ProviderType::Llm);
        registry.register("llm-2", ProviderType::Llm);
        registry.register("skill-1", ProviderType::Skill);

        let llm_providers = registry.get_all_by_type(ProviderType::Llm);
        assert_eq!(llm_providers.len(), 2);

        let skill_providers = registry.get_all_by_type(ProviderType::Skill);
        assert_eq!(skill_providers.len(), 1);
    }

    #[test]
    fn unregister_removes_provider() {
        let mut registry = MockRegistry::new();
        registry.register("ollama-1", ProviderType::Llm);
        assert_eq!(registry.len(), 1);

        let removed = registry.unregister("ollama-1");
        assert!(removed.is_some());
        assert!(registry.is_empty());
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
    fn assert_unhealthy_passes() {
        assert_unhealthy(&HealthStatus::Unhealthy {
            details: "bad".into(),
        });
    }

    #[test]
    fn assert_error_context_subsystem_passes() {
        let ctx = ErrorContext::new(Subsystem::Provider, "TEST", "msg".to_string());
        assert_error_context_subsystem(&ctx, Subsystem::Provider);
    }
}
