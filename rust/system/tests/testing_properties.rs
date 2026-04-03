//! Property-based tests for the System SDK `testing` module.
//!
//! **Property 13: MockAgentContext simulates lifecycle — initialize, health_check,
//! shutdown observable**
//!
//! **Validates: Requirements 15.2**

use lifesavor_system_sdk::builder::SystemComponentBuilder;
use lifesavor_system_sdk::testing::{LifecycleState, MockAgentContext};
use lifesavor_system_sdk::{ComponentHealthStatus, SystemComponentType};
use proptest::prelude::*;

/// Strategy that generates a random `SystemComponentType` from the available
/// (non-feature-gated) variants.
fn arb_component_type() -> impl Strategy<Value = SystemComponentType> {
    prop_oneof![
        Just(SystemComponentType::Identity),
        Just(SystemComponentType::Cache),
    ]
}

/// Strategy that generates a non-empty string suitable for a component name.
fn arb_component_name() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_-]{0,31}"
}

proptest! {
    /// **Property 13 (lifecycle transitions): For any component name and type,
    /// registering a component in MockAgentContext and running initialize →
    /// health_check → shutdown produces the expected state transitions:
    /// Uninitialized → Initialized → (still Initialized after health_check) → ShutDown.**
    ///
    /// **Validates: Requirements 15.2**
    #[test]
    fn mock_agent_context_lifecycle_transitions(
        name in arb_component_name(),
        comp_type in arb_component_type(),
    ) {
        let component = SystemComponentBuilder::new(&name, comp_type)
            .on_initialize(|| Box::pin(async { Ok(()) }))
            .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
            .on_shutdown(|| Box::pin(async { Ok(()) }))
            .build()
            .unwrap();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let mut ctx = MockAgentContext::new();

            // Before registration: Uninitialized, no component
            prop_assert_eq!(ctx.state(), LifecycleState::Uninitialized);
            prop_assert!(!ctx.has_component());

            // Register
            ctx.register(component);
            prop_assert!(ctx.has_component());
            prop_assert_eq!(ctx.component_name(), Some(name.as_str()));
            prop_assert_eq!(ctx.component_type(), Some(comp_type));
            prop_assert_eq!(ctx.state(), LifecycleState::Uninitialized);

            // Initialize
            ctx.initialize().await.unwrap();
            prop_assert_eq!(ctx.state(), LifecycleState::Initialized);

            // Health check
            let status = ctx.health_check().await.unwrap();
            prop_assert_eq!(status, ComponentHealthStatus::Healthy);
            prop_assert_eq!(
                ctx.last_health_status(),
                Some(&ComponentHealthStatus::Healthy)
            );
            // State remains Initialized after health check
            prop_assert_eq!(ctx.state(), LifecycleState::Initialized);

            // Shutdown
            ctx.shutdown().await.unwrap();
            prop_assert_eq!(ctx.state(), LifecycleState::ShutDown);

            Ok(())
        })?;
    }

    /// **Property 13 (inspection API): For any component name and type,
    /// the MockAgentContext inspection API (component_name, component_type,
    /// has_component) correctly reflects the registered component.**
    ///
    /// **Validates: Requirements 15.2**
    #[test]
    fn mock_agent_context_inspection_reflects_registered_component(
        name in arb_component_name(),
        comp_type in arb_component_type(),
    ) {
        let component = SystemComponentBuilder::new(&name, comp_type)
            .on_initialize(|| Box::pin(async { Ok(()) }))
            .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
            .on_shutdown(|| Box::pin(async { Ok(()) }))
            .build()
            .unwrap();

        let mut ctx = MockAgentContext::new();
        ctx.register(component);

        prop_assert!(ctx.has_component());
        prop_assert_eq!(ctx.component_name(), Some(name.as_str()));
        prop_assert_eq!(ctx.component_type(), Some(comp_type));
        prop_assert!(ctx.last_health_status().is_none());
    }

    /// **Property 13 (no component error): Calling initialize, health_check,
    /// or shutdown on an empty MockAgentContext returns an error.**
    ///
    /// **Validates: Requirements 15.2**
    #[test]
    fn mock_agent_context_errors_without_component(_dummy in 0..5u8) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let mut ctx = MockAgentContext::new();
            prop_assert!(ctx.initialize().await.is_err());
            prop_assert!(ctx.health_check().await.is_err());
            prop_assert!(ctx.shutdown().await.is_err());
            Ok(())
        })?;
    }
}
