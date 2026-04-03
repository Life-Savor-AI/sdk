//! Round-trip property tests for Skill SDK types.
//!
//! **Property 1: Serialization round-trip for ToolSchema (JSON)**
//!
//! **Validates: Requirements 17.3**
//!
//! Note: `ToolSchema` does not derive `PartialEq`, so we verify round-trip
//! correctness by comparing all fields after deserialization.

use lifesavor_skill_sdk::ToolSchema;
use proptest::prelude::*;
use serde_json::json;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

fn arb_safe_string() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_-]{0,30}"
}

fn arb_input_schema() -> impl Strategy<Value = serde_json::Value> {
    prop_oneof![
        Just(json!({"type": "object", "properties": {}})),
        arb_safe_string().prop_map(|name| json!({
            "type": "object",
            "properties": {
                name: {"type": "string"}
            }
        })),
        Just(json!({"type": "object", "properties": {"count": {"type": "integer"}}})),
    ]
}

fn arb_output_schema() -> impl Strategy<Value = Option<serde_json::Value>> {
    prop_oneof![
        Just(None),
        Just(Some(json!({"type": "object", "properties": {"result": {"type": "string"}}}))),
        Just(Some(json!({"type": "string"}))),
    ]
}

fn arb_tool_schema() -> impl Strategy<Value = ToolSchema> {
    (
        arb_safe_string(),
        arb_safe_string(),
        arb_input_schema(),
        arb_output_schema(),
    )
        .prop_map(|(name, description, input_schema, output_schema)| ToolSchema {
            name,
            description,
            input_schema,
            output_schema,
        })
}

// ---------------------------------------------------------------------------
// Property tests
// ---------------------------------------------------------------------------

proptest! {
    /// **Property 1: ToolSchema JSON round-trip**
    ///
    /// **Validates: Requirements 17.3**
    ///
    /// For any valid ToolSchema, serializing to JSON then parsing back SHALL
    /// produce a ToolSchema with identical fields.
    #[test]
    fn tool_schema_json_round_trip(schema in arb_tool_schema()) {
        let json_str = serde_json::to_string(&schema)
            .expect("ToolSchema should serialize to JSON");
        let deserialized: ToolSchema = serde_json::from_str(&json_str)
            .expect("JSON should deserialize back to ToolSchema");

        // Compare all fields (ToolSchema lacks PartialEq)
        prop_assert_eq!(&schema.name, &deserialized.name);
        prop_assert_eq!(&schema.description, &deserialized.description);
        prop_assert_eq!(&schema.input_schema, &deserialized.input_schema);
        prop_assert_eq!(&schema.output_schema, &deserialized.output_schema);
    }
}
