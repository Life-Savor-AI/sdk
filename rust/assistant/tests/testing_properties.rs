//! Property-based tests for the Assistant SDK `testing` module.
//!
//! **Property 16: MockAssistantStore simulates definition storage — load returns
//! matching or NotFound, list returns all**
//!
//! **Validates: Requirements 15.5**

use std::collections::HashSet;

use lifesavor_assistant_sdk::builder::AssistantDefinitionBuilder;
use lifesavor_assistant_sdk::error::AssistantSdkError;
use lifesavor_assistant_sdk::testing::MockAssistantStore;
use lifesavor_assistant_sdk::AssistantDefinition;
use proptest::prelude::*;

/// Strategy that generates a valid `AssistantDefinition` with a given id and
/// display name.
fn make_definition(id: &str, name: &str) -> AssistantDefinition {
    AssistantDefinitionBuilder::new()
        .id(id)
        .display_name(name)
        .system_prompt_template("You are a helpful assistant.")
        .build()
        .unwrap()
}

/// Strategy that generates a set of (id, display_name) pairs with unique ids.
fn arb_definition_set() -> impl Strategy<Value = Vec<(String, String)>> {
    prop::collection::vec(
        (
            "[a-z][a-z0-9_-]{0,15}",  // id
            "[A-Z][a-zA-Z ]{0,20}",   // display_name
        ),
        1..=10,
    )
    .prop_map(|pairs| {
        let mut seen = HashSet::new();
        pairs
            .into_iter()
            .filter(|(id, _)| seen.insert(id.clone()))
            .collect()
    })
}

proptest! {
    /// **Property 16 (load returns matching): For any set of AssistantDefinitions
    /// loaded into MockAssistantStore, calling load(id) returns the definition
    /// with matching id.**
    ///
    /// **Validates: Requirements 15.5**
    #[test]
    fn store_load_returns_matching_definition(
        defs in arb_definition_set(),
    ) {
        let mut store = MockAssistantStore::new();
        for (id, name) in &defs {
            store.add(make_definition(id, name));
        }

        for (id, name) in &defs {
            let loaded = store.load(id);
            prop_assert!(loaded.is_ok(), "load({:?}) should succeed", id);
            let loaded = loaded.unwrap();
            prop_assert_eq!(&loaded.id, id);
            prop_assert_eq!(&loaded.display_name, name);
        }
    }

    /// **Property 16 (load returns NotFound): For any id NOT in the store,
    /// load returns a NotFound error.**
    ///
    /// **Validates: Requirements 15.5**
    #[test]
    fn store_load_returns_not_found_for_missing_id(
        defs in arb_definition_set(),
        missing_id in "[a-z]{1,4}_missing_[0-9]{1,3}",
    ) {
        let mut store = MockAssistantStore::new();
        for (id, name) in &defs {
            store.add(make_definition(id, name));
        }

        // Ensure missing_id is not in the set
        prop_assume!(!defs.iter().any(|(id, _)| id == &missing_id));

        let result = store.load(&missing_id);
        prop_assert!(result.is_err(), "load({:?}) should fail for missing id", missing_id);
        let err = result.unwrap_err();
        prop_assert!(
            matches!(err, AssistantSdkError::NotFound(_)),
            "Error should be NotFound, got: {:?}",
            err
        );
    }

    /// **Property 16 (list returns all): For any set of AssistantDefinitions
    /// loaded into MockAssistantStore, list() returns summaries for all loaded
    /// definitions.**
    ///
    /// **Validates: Requirements 15.5**
    #[test]
    fn store_list_returns_all_summaries(
        defs in arb_definition_set(),
    ) {
        let mut store = MockAssistantStore::new();
        for (id, name) in &defs {
            store.add(make_definition(id, name));
        }

        let summaries = store.list();
        prop_assert_eq!(summaries.len(), defs.len());

        let summary_ids: HashSet<&str> = summaries.iter().map(|s| s.id.as_str()).collect();
        for (id, _) in &defs {
            prop_assert!(
                summary_ids.contains(id.as_str()),
                "list() should include summary for id={:?}",
                id
            );
        }
    }

    /// **Property 16 (list on empty store): An empty MockAssistantStore returns
    /// an empty list.**
    ///
    /// **Validates: Requirements 15.5**
    #[test]
    fn store_empty_list_returns_empty(_dummy in 0..5u8) {
        let store = MockAssistantStore::new();
        prop_assert!(store.list().is_empty());
        prop_assert!(store.is_empty());
        prop_assert_eq!(store.len(), 0);
    }

    /// **Property 16 (add replaces existing): Adding a definition with the same
    /// id replaces the previous one; load returns the new definition and len
    /// stays the same.**
    ///
    /// **Validates: Requirements 15.5**
    #[test]
    fn store_add_replaces_existing_definition(
        id in "[a-z][a-z0-9_-]{0,15}",
        name1 in "[A-Z][a-zA-Z]{0,10}",
        name2 in "[A-Z][a-zA-Z]{0,10}",
    ) {
        let mut store = MockAssistantStore::new();
        store.add(make_definition(&id, &name1));
        prop_assert_eq!(store.len(), 1);

        store.add(make_definition(&id, &name2));
        prop_assert_eq!(store.len(), 1);

        let loaded = store.load(&id).unwrap();
        prop_assert_eq!(&loaded.display_name, &name2);
    }

    /// **Property 16 (summary fields): For any definition added to the store,
    /// the summary returned by list() has matching id and display_name.**
    ///
    /// **Validates: Requirements 15.5**
    #[test]
    fn store_summary_fields_match_definition(
        id in "[a-z][a-z0-9_-]{0,15}",
        name in "[A-Z][a-zA-Z ]{0,20}",
    ) {
        let mut store = MockAssistantStore::new();
        store.add(make_definition(&id, &name));

        let summaries = store.list();
        prop_assert_eq!(summaries.len(), 1);
        prop_assert_eq!(&summaries[0].id, &id);
        prop_assert_eq!(&summaries[0].display_name, &name);
    }
}
