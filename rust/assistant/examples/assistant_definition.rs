//! Assistant definition example.
//!
//! Demonstrates sample `AssistantDefinition` files in JSON and TOML formats
//! with variable substitution, tool bindings, guardrail rules, model
//! preferences, and handoff configuration.
//!
//! Run with: `cargo run --example assistant_definition`

use std::collections::HashMap;

use lifesavor_assistant_sdk::prelude::*;
use lifesavor_assistant_sdk::builder::AssistantDefinitionBuilder;

// ---------------------------------------------------------------------------
// Build a rich definition using the builder API
// ---------------------------------------------------------------------------

fn build_full_definition() -> AssistantDefinition {
    AssistantDefinitionBuilder::new()
        .id("customer-support-v2")
        .display_name("Customer Support v2")
        .system_prompt_template(
            "You are {{role}} for {{company}}. \
             Always greet the user by saying: \"{{greeting}}\".",
        )
        .variable("role", "a helpful support agent")
        .variable("company", "Life Savor")
        .variable("greeting", "Welcome! How can I help you today?")
        // Model preferences — ordered by priority.
        .model_preference("llama3-70b")
        .model_preference("mistral-7b")
        // Tool bindings.
        .tool_binding(ToolBinding {
            skill_id: Some("knowledge-search".to_string()),
            mcp_tool: None,
            system_component: None,
        })
        .tool_binding(ToolBinding {
            skill_id: None,
            mcp_tool: Some("mcp://ticket-system/create-ticket".to_string()),
            system_component: None,
        })
        .tool_binding(ToolBinding {
            skill_id: None,
            mcp_tool: None,
            system_component: Some("cache".to_string()),
        })
        // Guardrail rules.
        .guardrail_rule(GuardrailRule {
            id: "no-pii".to_string(),
            description: "Block personally identifiable information in responses".to_string(),
            rule_type: "content_filter".to_string(),
            config: {
                let mut m = HashMap::new();
                m.insert(
                    "patterns".to_string(),
                    serde_json::json!(["SSN", "credit_card"]),
                );
                m
            },
        })
        .guardrail_rule(GuardrailRule {
            id: "topic-restrict".to_string(),
            description: "Restrict conversation to product support topics".to_string(),
            rule_type: "topic_restriction".to_string(),
            config: {
                let mut m = HashMap::new();
                m.insert(
                    "allowed_topics".to_string(),
                    serde_json::json!(["billing", "technical", "account"]),
                );
                m
            },
        })
        // Handoff configuration.
        .handoff_config(HandoffConfig {
            target_assistant_id: "escalation-agent".to_string(),
            trigger: "keyword".to_string(),
            keywords: vec![
                "escalate".to_string(),
                "manager".to_string(),
                "supervisor".to_string(),
            ],
        })
        .context_window_strategy("summary")
        .build()
        .expect("valid definition")
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("Assistant definition example — JSON and TOML serialization");

    let def = build_full_definition();

    // --- JSON serialization ---
    let json = serde_json::to_string_pretty(&def).expect("JSON serialize");
    info!("AssistantDefinition as JSON:");
    println!("{json}");

    // Round-trip: JSON → AssistantDefinition.
    let from_json: AssistantDefinition =
        serde_json::from_str(&json).expect("JSON deserialize");
    assert_eq!(from_json.id, def.id);
    assert_eq!(from_json.display_name, def.display_name);
    assert_eq!(from_json.tool_bindings.len(), def.tool_bindings.len());
    assert_eq!(from_json.guardrail_rules.len(), def.guardrail_rules.len());
    info!("JSON round-trip verified");

    // --- TOML serialization ---
    let toml_str = toml::to_string_pretty(&def).expect("TOML serialize");
    info!("AssistantDefinition as TOML:");
    println!("{toml_str}");

    // Round-trip: TOML → AssistantDefinition.
    let from_toml: AssistantDefinition =
        toml::from_str(&toml_str).expect("TOML deserialize");
    assert_eq!(from_toml.id, def.id);
    assert_eq!(from_toml.context_window_strategy, "summary");
    assert_eq!(from_toml.model_preferences, vec!["llama3-70b", "mistral-7b"]);
    info!("TOML round-trip verified");

    // --- Variable substitution ---
    let prompt = substitute_variables(&def.system_prompt_template, &def.variables);
    info!(prompt = %prompt, "Substituted system prompt");
    assert!(prompt.contains("helpful support agent"));
    assert!(prompt.contains("Life Savor"));
    assert!(prompt.contains("Welcome! How can I help you today?"));

    // --- Handoff config ---
    let handoff = def.handoff_to.as_ref().expect("handoff configured");
    info!(
        target = %handoff.target_assistant_id,
        trigger = %handoff.trigger,
        keywords = ?handoff.keywords,
        "Handoff configuration"
    );

    info!("Assistant definition example complete");
}
