//! Round-trip property tests for Assistant SDK types.
//!
//! **Property 1: Serialization round-trip for AssistantDefinition (JSON)**
//!
//! **Validates: Requirements 17.2**
//!
//! Note: `AssistantDefinition` does not derive `PartialEq`, so we verify
//! round-trip correctness by comparing all key fields after deserialization.

use std::collections::HashMap;

use lifesavor_assistant_sdk::{
    AssistantDefinition, GuardrailRule, HandoffConfig, ToolBinding,
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

fn arb_safe_string() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_ -]{0,30}"
}

fn arb_tool_binding() -> impl Strategy<Value = ToolBinding> {
    prop_oneof![
        arb_safe_string().prop_map(|s| ToolBinding {
            skill_id: Some(s),
            mcp_tool: None,
            system_component: None,
        }),
        arb_safe_string().prop_map(|s| ToolBinding {
            skill_id: None,
            mcp_tool: Some(format!("mcp://server/{}", s)),
            system_component: None,
        }),
        arb_safe_string().prop_map(|s| ToolBinding {
            skill_id: None,
            mcp_tool: None,
            system_component: Some(s),
        }),
    ]
}

fn arb_guardrail_rule() -> impl Strategy<Value = GuardrailRule> {
    (arb_safe_string(), arb_safe_string()).prop_map(|(id, desc)| GuardrailRule {
        id,
        description: desc,
        rule_type: "content_filter".to_string(),
        config: HashMap::new(),
    })
}

fn arb_handoff_config() -> impl Strategy<Value = Option<HandoffConfig>> {
    prop_oneof![
        Just(None),
        arb_safe_string().prop_map(|target| Some(HandoffConfig {
            target_assistant_id: target,
            trigger: "keyword".to_string(),
            keywords: vec!["help".to_string()],
        })),
    ]
}

fn arb_assistant_definition() -> impl Strategy<Value = AssistantDefinition> {
    (
        arb_safe_string(),
        arb_safe_string(),
        arb_safe_string(),
        prop::collection::vec(arb_safe_string(), 0..3),
        prop::collection::vec(arb_tool_binding(), 0..3),
        prop::collection::vec(arb_guardrail_rule(), 0..2),
        arb_handoff_config(),
    )
        .prop_map(
            |(id, display_name, prompt, model_prefs, bindings, rules, handoff)| {
                AssistantDefinition {
                    id,
                    display_name,
                    system_prompt_template: prompt,
                    model_preferences: model_prefs,
                    tool_bindings: bindings,
                    guardrail_rules: rules,
                    context_window_strategy: "sliding".to_string(),
                    pipeline: None,
                    handoff_to: handoff,
                    variables: HashMap::new(),
                }
            },
        )
}

// ---------------------------------------------------------------------------
// Property tests
// ---------------------------------------------------------------------------

proptest! {
    /// **Property 1: AssistantDefinition JSON round-trip**
    ///
    /// **Validates: Requirements 17.2**
    ///
    /// For any valid AssistantDefinition, serializing to JSON then parsing
    /// back SHALL produce an AssistantDefinition with identical key fields.
    #[test]
    fn assistant_definition_json_round_trip(def in arb_assistant_definition()) {
        let json_str = serde_json::to_string(&def)
            .expect("AssistantDefinition should serialize to JSON");
        let deserialized: AssistantDefinition = serde_json::from_str(&json_str)
            .expect("JSON should deserialize back to AssistantDefinition");

        // Compare all key fields (AssistantDefinition lacks PartialEq)
        prop_assert_eq!(&def.id, &deserialized.id);
        prop_assert_eq!(&def.display_name, &deserialized.display_name);
        prop_assert_eq!(&def.system_prompt_template, &deserialized.system_prompt_template);
        prop_assert_eq!(&def.model_preferences, &deserialized.model_preferences);
        prop_assert_eq!(&def.context_window_strategy, &deserialized.context_window_strategy);
        prop_assert_eq!(&def.variables, &deserialized.variables);
        prop_assert_eq!(def.tool_bindings.len(), deserialized.tool_bindings.len());
        prop_assert_eq!(def.guardrail_rules.len(), deserialized.guardrail_rules.len());
        prop_assert_eq!(def.pipeline, deserialized.pipeline);

        // Verify tool bindings round-trip
        for (orig, rt) in def.tool_bindings.iter().zip(deserialized.tool_bindings.iter()) {
            prop_assert_eq!(&orig.skill_id, &rt.skill_id);
            prop_assert_eq!(&orig.mcp_tool, &rt.mcp_tool);
            prop_assert_eq!(&orig.system_component, &rt.system_component);
        }

        // Verify guardrail rules round-trip
        for (orig, rt) in def.guardrail_rules.iter().zip(deserialized.guardrail_rules.iter()) {
            prop_assert_eq!(&orig.id, &rt.id);
            prop_assert_eq!(&orig.description, &rt.description);
            prop_assert_eq!(&orig.rule_type, &rt.rule_type);
        }

        // Verify handoff config round-trip
        match (&def.handoff_to, &deserialized.handoff_to) {
            (Some(orig), Some(rt)) => {
                prop_assert_eq!(&orig.target_assistant_id, &rt.target_assistant_id);
                prop_assert_eq!(&orig.trigger, &rt.trigger);
                prop_assert_eq!(&orig.keywords, &rt.keywords);
            }
            (None, None) => {}
            _ => prop_assert!(false, "handoff_to mismatch"),
        }
    }
}
