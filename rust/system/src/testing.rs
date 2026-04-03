//! Test harness for the System SDK.
//!
//! Provides [`MockAgentContext`] for testing [`SystemComponent`] implementations
//! in isolation without a running agent. The mock tracks lifecycle state
//! transitions and provides an inspection API.
//!
//! Also provides assertion helpers for verifying error chains, health check
//! responses, and capability descriptors.

use crate::{ComponentHealthStatus, ErrorContext, Subsystem, SystemComponent, SystemComponentType};

// ---------------------------------------------------------------------------
// Lifecycle state
// ---------------------------------------------------------------------------

/// Observable lifecycle state of a component within [`MockAgentContext`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    /// Component has been registered but not yet initialized.
    Uninitialized,
    /// Component has been successfully initialized.
    Initialized,
    /// Component has been shut down.
    ShutDown,
}

// ---------------------------------------------------------------------------
// MockAgentContext
// ---------------------------------------------------------------------------

/// Simulates the agent's initialization lifecycle for testing
/// [`SystemComponent`] implementations.
///
/// # Example
///
/// ```rust,ignore
/// use lifesavor_system_sdk::testing::MockAgentContext;
/// use lifesavor_system_sdk::builder::SystemComponentBuilder;
/// use lifesavor_system_sdk::{SystemComponentType, ComponentHealthStatus};
///
/// let component = SystemComponentBuilder::new("test", SystemComponentType::Cache)
///     .on_initialize(|| Box::pin(async { Ok(()) }))
///     .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
///     .on_shutdown(|| Box::pin(async { Ok(()) }))
///     .build()
///     .unwrap();
///
/// let mut ctx = MockAgentContext::new();
/// ctx.register(component);
/// ctx.initialize().await.unwrap();
/// assert_eq!(ctx.state(), LifecycleState::Initialized);
/// ```
pub struct MockAgentContext {
    component: Option<Box<dyn SystemComponent>>,
    state: LifecycleState,
    last_health: Option<ComponentHealthStatus>,
}

impl MockAgentContext {
    /// Create a new mock context with no registered component.
    pub fn new() -> Self {
        Self {
            component: None,
            state: LifecycleState::Uninitialized,
            last_health: None,
        }
    }

    /// Register a [`SystemComponent`] for lifecycle testing.
    pub fn register(&mut self, component: Box<dyn SystemComponent>) {
        self.component = Some(component);
        self.state = LifecycleState::Uninitialized;
        self.last_health = None;
    }

    /// Initialize the registered component.
    ///
    /// Transitions state to [`LifecycleState::Initialized`] on success.
    pub async fn initialize(&mut self) -> Result<(), String> {
        let component = self
            .component
            .as_mut()
            .ok_or_else(|| "No component registered".to_string())?;
        component
            .initialize()
            .await
            .map_err(|e| format!("Initialize failed: {e}"))?;
        self.state = LifecycleState::Initialized;
        Ok(())
    }

    /// Run a health check on the registered component.
    ///
    /// Stores the result for later inspection via [`last_health_status`](Self::last_health_status).
    pub async fn health_check(&mut self) -> Result<ComponentHealthStatus, String> {
        let component = self
            .component
            .as_ref()
            .ok_or_else(|| "No component registered".to_string())?;
        let status = component.health_check().await;
        self.last_health = Some(status.clone());
        Ok(status)
    }

    /// Shut down the registered component.
    ///
    /// Transitions state to [`LifecycleState::ShutDown`] on success.
    pub async fn shutdown(&mut self) -> Result<(), String> {
        let component = self
            .component
            .as_mut()
            .ok_or_else(|| "No component registered".to_string())?;
        component
            .shutdown()
            .await
            .map_err(|e| format!("Shutdown failed: {e}"))?;
        self.state = LifecycleState::ShutDown;
        Ok(())
    }

    // -- Inspection API --

    /// Return the current lifecycle state.
    pub fn state(&self) -> LifecycleState {
        self.state
    }

    /// Return the last health check result, if any.
    pub fn last_health_status(&self) -> Option<&ComponentHealthStatus> {
        self.last_health.as_ref()
    }

    /// Return the registered component's name, if any.
    pub fn component_name(&self) -> Option<&str> {
        self.component.as_ref().map(|c| c.component_name())
    }

    /// Return the registered component's type, if any.
    pub fn component_type(&self) -> Option<SystemComponentType> {
        self.component.as_ref().map(|c| c.component_type())
    }

    /// Return whether a component is registered.
    pub fn has_component(&self) -> bool {
        self.component.is_some()
    }
}

impl Default for MockAgentContext {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

/// Assert that an [`ErrorContext`] has the expected subsystem.
///
/// Panics with a descriptive message if the subsystem does not match.
pub fn assert_error_context_subsystem(ctx: &ErrorContext, expected: Subsystem) {
    assert_eq!(
        ctx.subsystem, expected,
        "Expected subsystem {:?}, got {:?}",
        expected, ctx.subsystem,
    );
}

/// Assert that a [`ComponentHealthStatus`] is [`ComponentHealthStatus::Healthy`].
pub fn assert_healthy(status: &ComponentHealthStatus) {
    assert_eq!(
        *status,
        ComponentHealthStatus::Healthy,
        "Expected Healthy, got {status:?}",
    );
}

/// Assert that a [`ComponentHealthStatus`] is NOT [`ComponentHealthStatus::Healthy`].
pub fn assert_unhealthy(status: &ComponentHealthStatus) {
    assert_ne!(
        *status,
        ComponentHealthStatus::Healthy,
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
    use crate::builder::SystemComponentBuilder;

    fn build_test_component() -> Box<dyn SystemComponent> {
        SystemComponentBuilder::new("test-component", SystemComponentType::Cache)
            .on_initialize(|| Box::pin(async { Ok(()) }))
            .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
            .on_shutdown(|| Box::pin(async { Ok(()) }))
            .build()
            .unwrap()
    }

    #[test]
    fn new_context_is_uninitialized() {
        let ctx = MockAgentContext::new();
        assert_eq!(ctx.state(), LifecycleState::Uninitialized);
        assert!(!ctx.has_component());
        assert!(ctx.last_health_status().is_none());
    }

    #[test]
    fn register_sets_component() {
        let mut ctx = MockAgentContext::new();
        ctx.register(build_test_component());
        assert!(ctx.has_component());
        assert_eq!(ctx.component_name(), Some("test-component"));
        assert_eq!(ctx.component_type(), Some(SystemComponentType::Cache));
        assert_eq!(ctx.state(), LifecycleState::Uninitialized);
    }

    #[tokio::test]
    async fn full_lifecycle() {
        let mut ctx = MockAgentContext::new();
        ctx.register(build_test_component());

        // Initialize
        ctx.initialize().await.unwrap();
        assert_eq!(ctx.state(), LifecycleState::Initialized);

        // Health check
        let status = ctx.health_check().await.unwrap();
        assert_eq!(status, ComponentHealthStatus::Healthy);
        assert_eq!(
            ctx.last_health_status(),
            Some(&ComponentHealthStatus::Healthy)
        );

        // Shutdown
        ctx.shutdown().await.unwrap();
        assert_eq!(ctx.state(), LifecycleState::ShutDown);
    }

    #[tokio::test]
    async fn initialize_without_component_errors() {
        let mut ctx = MockAgentContext::new();
        let result = ctx.initialize().await;
        assert!(result.is_err());
    }

    #[test]
    fn assert_healthy_passes_for_healthy() {
        assert_healthy(&ComponentHealthStatus::Healthy);
    }

    #[test]
    #[should_panic(expected = "Expected Healthy")]
    fn assert_healthy_panics_for_unhealthy() {
        assert_healthy(&ComponentHealthStatus::Unhealthy {
            details: "bad".into(),
        });
    }

    #[test]
    fn assert_unhealthy_passes_for_unhealthy() {
        assert_unhealthy(&ComponentHealthStatus::Unhealthy {
            details: "bad".into(),
        });
    }

    #[test]
    #[should_panic(expected = "Expected unhealthy")]
    fn assert_unhealthy_panics_for_healthy() {
        assert_unhealthy(&ComponentHealthStatus::Healthy);
    }

    #[test]
    fn assert_error_context_subsystem_passes() {
        let ctx = ErrorContext::new(Subsystem::Bridge, "TEST", "test message".to_string());
        assert_error_context_subsystem(&ctx, Subsystem::Bridge);
    }

    #[test]
    fn assert_error_context_non_empty_passes() {
        let ctx = ErrorContext::new(Subsystem::Bridge, "TEST", "test message".to_string());
        assert_error_context_non_empty(&ctx);
    }
}
