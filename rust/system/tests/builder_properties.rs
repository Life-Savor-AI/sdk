//! Property-based tests for `SystemComponentBuilder`.
//!
//! **Validates: Requirements 2.3**

use lifesavor_system_sdk::builder::SystemComponentBuilder;
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
/// We filter out whitespace-only strings since the builder rejects those.
fn arb_non_empty_name() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_-]{0,63}".prop_map(|s| s)
}

proptest! {
    /// **Property 3: Builder produces valid SystemComponent for valid inputs**
    ///
    /// **Validates: Requirements 2.3**
    ///
    /// For any non-empty component name and valid SystemComponentType, the
    /// SystemComponentBuilder with initialize, health_check, and shutdown
    /// closures provided SHALL produce a `Box<dyn SystemComponent>` that can
    /// be initialized, health-checked, and shut down without error.
    #[test]
    fn builder_produces_valid_component_for_valid_inputs(
        name in arb_non_empty_name(),
        comp_type in arb_component_type(),
    ) {
        // Build the component
        let result = SystemComponentBuilder::new(&name, comp_type)
            .on_initialize(|| Box::pin(async { Ok(()) }))
            .on_health_check(|| Box::pin(async { ComponentHealthStatus::Healthy }))
            .on_shutdown(|| Box::pin(async { Ok(()) }))
            .build();

        // Assert build succeeds
        prop_assert!(result.is_ok(), "build() should succeed for name={:?}, type={:?}", name, comp_type);

        let component = result.unwrap();

        // Assert name and type match
        prop_assert_eq!(component.component_name(), name.as_str());
        prop_assert_eq!(component.component_type(), comp_type);

        // Run the lifecycle (initialize, health_check, shutdown) and assert no errors
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let mut component = component;
            let init_result = component.initialize().await;
            prop_assert!(init_result.is_ok(), "initialize() should succeed");

            let health = component.health_check().await;
            prop_assert_eq!(health, ComponentHealthStatus::Healthy);

            let shutdown_result = component.shutdown().await;
            prop_assert!(shutdown_result.is_ok(), "shutdown() should succeed");
            Ok(())
        })?;
    }
}
