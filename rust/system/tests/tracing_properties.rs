//! Property-based tests for `span_with_context` tracing helper.
//!
//! **Validates: Requirements 23.2**

use lifesavor_system_sdk::span_with_context;
use proptest::prelude::*;
use std::sync::Once;

static INIT_SUBSCRIBER: Once = Once::new();

/// Install a global tracing subscriber so spans are not disabled/no-op.
fn ensure_subscriber() {
    INIT_SUBSCRIBER.call_once(|| {
        let _ = tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_test_writer()
                .with_max_level(tracing::Level::TRACE)
                .finish(),
        );
    });
}

/// Strategy that generates a non-empty string suitable for correlation_id / instance_id.
fn arb_non_empty_string() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{1,64}".prop_map(|s| s)
}

proptest! {
    /// **Property 17: span_with_context populates tracing fields for any non-empty
    /// correlation_id and instance_id**
    ///
    /// **Validates: Requirements 23.2**
    ///
    /// For any non-empty `correlation_id` and `instance_id`, the `span_with_context`
    /// helper SHALL produce a `tracing::Span` that is created without panicking and
    /// has the expected name ("system_sdk").
    #[test]
    fn span_with_context_creates_span_without_panicking(
        correlation_id in arb_non_empty_string(),
        instance_id in arb_non_empty_string(),
    ) {
        ensure_subscriber();

        let span = span_with_context(&correlation_id, None, &instance_id);

        // Verify the span has metadata (not disabled) and the expected name.
        let meta = span.metadata();
        prop_assert!(meta.is_some(), "span should have metadata");
        prop_assert_eq!(meta.unwrap().name(), "system_sdk");

        // Verify the span has the expected field names in its metadata.
        let field_names: Vec<&str> = meta.unwrap().fields().iter().map(|f| f.name()).collect();
        prop_assert!(field_names.contains(&"correlation_id"), "should have correlation_id field");
        prop_assert!(field_names.contains(&"user_id"), "should have user_id field");
        prop_assert!(field_names.contains(&"instance_id"), "should have instance_id field");

        // Also test with a user_id provided — should not panic.
        let span_with_user = span_with_context(&correlation_id, Some("test-user"), &instance_id);
        let meta2 = span_with_user.metadata();
        prop_assert!(meta2.is_some(), "span with user_id should have metadata");
        prop_assert_eq!(meta2.unwrap().name(), "system_sdk");
    }
}
