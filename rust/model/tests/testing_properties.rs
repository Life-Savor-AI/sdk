//! Property-based tests for the Model SDK `testing` module.
//!
//! **Property 14: MockRegistry simulates provider registration and routing**
//!
//! **Validates: Requirements 15.3**

use lifesavor_model_sdk::testing::MockRegistry;
use lifesavor_model_sdk::ProviderType;
use lifesavor_agent::providers::skill_provider::HealthStatus;
use proptest::prelude::*;

/// Strategy that generates a non-empty provider instance name.
fn arb_instance_name() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_-]{0,31}"
}

/// Strategy that generates a random `ProviderType`.
fn arb_provider_type() -> impl Strategy<Value = ProviderType> {
    prop_oneof![
        Just(ProviderType::Llm),
        Just(ProviderType::Skill),
        Just(ProviderType::Assistant),
        Just(ProviderType::MemoryStore),
    ]
}

/// Strategy that generates a set of (name, type) pairs with unique names.
fn arb_provider_set() -> impl Strategy<Value = Vec<(String, ProviderType)>> {
    prop::collection::vec(
        (arb_instance_name(), arb_provider_type()),
        1..=10,
    )
    .prop_map(|pairs| {
        // Deduplicate by name, keeping the first occurrence
        let mut seen = std::collections::HashSet::new();
        pairs
            .into_iter()
            .filter(|(name, _)| seen.insert(name.clone()))
            .collect()
    })
}

proptest! {
    /// **Property 14 (registration and query by type): For any set of providers
    /// registered with MockRegistry, querying by type returns a provider of that
    /// type if one was registered, or None if none was registered.**
    ///
    /// **Validates: Requirements 15.3**
    #[test]
    fn registry_query_by_type_returns_matching_provider(
        providers in arb_provider_set(),
    ) {
        let mut registry = MockRegistry::new();
        for (name, pt) in &providers {
            registry.register(name, *pt);
        }

        // For each registered type, get_by_type should return a matching entry
        for (_, pt) in &providers {
            let entry = registry.get_by_type(*pt);
            prop_assert!(
                entry.is_some(),
                "get_by_type({:?}) should find a provider after registration",
                pt
            );
            prop_assert_eq!(entry.unwrap().provider_type, *pt);
        }
    }

    /// **Property 14 (query by name): For any set of providers registered with
    /// MockRegistry, querying by name returns the correct entry.**
    ///
    /// **Validates: Requirements 15.3**
    #[test]
    fn registry_query_by_name_returns_correct_entry(
        providers in arb_provider_set(),
    ) {
        let mut registry = MockRegistry::new();
        for (name, pt) in &providers {
            registry.register(name, *pt);
        }

        prop_assert_eq!(registry.len(), providers.len());

        for (name, pt) in &providers {
            let entry = registry.get_by_name(name);
            prop_assert!(entry.is_some(), "get_by_name({:?}) should find the provider", name);
            let entry = entry.unwrap();
            prop_assert_eq!(&entry.instance_name, name);
            prop_assert_eq!(entry.provider_type, *pt);
            prop_assert_eq!(entry.health_status.clone(), HealthStatus::Healthy);
        }
    }

    /// **Property 14 (get_all_by_type count): For any set of providers, the
    /// count returned by get_all_by_type matches the number of providers
    /// registered with that type.**
    ///
    /// **Validates: Requirements 15.3**
    #[test]
    fn registry_get_all_by_type_count_matches(
        providers in arb_provider_set(),
    ) {
        let mut registry = MockRegistry::new();
        for (name, pt) in &providers {
            registry.register(name, *pt);
        }

        for pt in &[ProviderType::Llm, ProviderType::Skill, ProviderType::Assistant, ProviderType::MemoryStore] {
            let expected_count = providers.iter().filter(|(_, t)| t == pt).count();
            let actual = registry.get_all_by_type(*pt);
            prop_assert_eq!(
                actual.len(),
                expected_count,
                "get_all_by_type({:?}) count mismatch",
                pt
            );
        }
    }

    /// **Property 14 (missing type returns None): For any ProviderType not
    /// present in the registry, get_by_type returns None.**
    ///
    /// **Validates: Requirements 15.3**
    #[test]
    fn registry_missing_type_returns_none(
        name in arb_instance_name(),
        registered_type in arb_provider_type(),
        query_type in arb_provider_type(),
    ) {
        prop_assume!(registered_type != query_type);

        let mut registry = MockRegistry::new();
        registry.register(&name, registered_type);

        let result = registry.get_by_type(query_type);
        prop_assert!(
            result.is_none(),
            "get_by_type({:?}) should return None when only {:?} is registered",
            query_type,
            registered_type
        );
    }

    /// **Property 14 (health status tracking): For any registered provider,
    /// setting health status is observable via health_of.**
    ///
    /// **Validates: Requirements 15.3**
    #[test]
    fn registry_health_status_is_observable(
        name in arb_instance_name(),
        pt in arb_provider_type(),
    ) {
        let mut registry = MockRegistry::new();
        registry.register(&name, pt);

        // Initially healthy
        prop_assert_eq!(registry.health_of(&name), Some(&HealthStatus::Healthy));

        // Set to unhealthy
        let updated = registry.set_health(
            &name,
            HealthStatus::Unhealthy { details: "test failure".into() },
        );
        prop_assert!(updated);

        let status = registry.health_of(&name).unwrap();
        prop_assert!(
            !matches!(status, HealthStatus::Healthy),
            "Expected unhealthy status after set_health"
        );

        // Set back to healthy
        let updated = registry.set_health(&name, HealthStatus::Healthy);
        prop_assert!(updated);
        prop_assert_eq!(registry.health_of(&name), Some(&HealthStatus::Healthy));
    }

    /// **Property 14 (unregister): For any registered provider, unregistering
    /// removes it from the registry.**
    ///
    /// **Validates: Requirements 15.3**
    #[test]
    fn registry_unregister_removes_provider(
        providers in arb_provider_set(),
    ) {
        prop_assume!(!providers.is_empty());

        let mut registry = MockRegistry::new();
        for (name, pt) in &providers {
            registry.register(name, *pt);
        }

        let (remove_name, _) = &providers[0];
        let removed = registry.unregister(remove_name);
        prop_assert!(removed.is_some());
        prop_assert_eq!(registry.len(), providers.len() - 1);
        prop_assert!(registry.get_by_name(remove_name).is_none());
    }
}
