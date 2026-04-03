//! Validation example for the Assistant SDK.
//!
//! Demonstrates `validate_definition` with valid and invalid
//! `AssistantDefinition` instances, showing the error messages produced
//! for each validation failure.
//!
//! Run with: `cargo run --example validation`

use std::collections::HashMap;

use lifesavor_assistant_sdk::prelude::*;
use lifesavor_assistant_sdk::builder::AssistantDefinitionBuilder;

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("Validation example — valid and invalid AssistantDefinitions");

    // -----------------------------------------------------------------------
    // 1. Valid definition — passes validation
    // -----------------------------------------------------------------------
    let valid = AssistantDefinitionBuilder::new()
        .id("valid-assistant")
        .display_name("Valid Assistant")
        .system_prompt_template("You are {{role}}.")
        .variable("role", "a helpful bot")
        .build()
        .expect("builder should succeed for valid inputs");

    match validate_definition(&valid) {
        Ok(()) => info!(id = %valid.id, "Definition is valid"),
        Err(e) => error!(error = %e, "Unexpected validation failure"),
    }

    // -----------------------------------------------------------------------
    // 2. Invalid: empty id
    // -----------------------------------------------------------------------
    let empty_id = AssistantDefinition {
        id: String::new(),
        display_name: "No ID".to_string(),
        system_prompt_template: "Hello".to_string(),
        model_preferences: vec![],
        tool_bindings: vec![],
        guardrail_rules: vec![],
        context_window_strategy: "sliding".to_string(),
        pipeline: None,
        handoff_to: None,
        variables: HashMap::new(),
    };

    match validate_definition(&empty_id) {
        Ok(()) => warn!("Expected validation to fail for empty id"),
        Err(e) => info!(error = %e, "Correctly rejected empty id"),
    }

    // -----------------------------------------------------------------------
    // 3. Invalid: empty display_name
    // -----------------------------------------------------------------------
    let empty_name = AssistantDefinition {
        id: "has-id".to_string(),
        display_name: String::new(),
        system_prompt_template: "Hello".to_string(),
        model_preferences: vec![],
        tool_bindings: vec![],
        guardrail_rules: vec![],
        context_window_strategy: "sliding".to_string(),
        pipeline: None,
        handoff_to: None,
        variables: HashMap::new(),
    };

    match validate_definition(&empty_name) {
        Ok(()) => warn!("Expected validation to fail for empty display_name"),
        Err(e) => info!(error = %e, "Correctly rejected empty display_name"),
    }

    // -----------------------------------------------------------------------
    // 4. Invalid: empty system_prompt_template
    // -----------------------------------------------------------------------
    let empty_prompt = AssistantDefinition {
        id: "has-id".to_string(),
        display_name: "Has Name".to_string(),
        system_prompt_template: String::new(),
        model_preferences: vec![],
        tool_bindings: vec![],
        guardrail_rules: vec![],
        context_window_strategy: "sliding".to_string(),
        pipeline: None,
        handoff_to: None,
        variables: HashMap::new(),
    };

    match validate_definition(&empty_prompt) {
        Ok(()) => warn!("Expected validation to fail for empty prompt"),
        Err(e) => info!(error = %e, "Correctly rejected empty system_prompt_template"),
    }

    // -----------------------------------------------------------------------
    // 5. Invalid: undefined template variable
    // -----------------------------------------------------------------------
    let undefined_var = AssistantDefinition {
        id: "has-id".to_string(),
        display_name: "Has Name".to_string(),
        system_prompt_template: "Hello {{name}}, welcome to {{place}}".to_string(),
        model_preferences: vec![],
        tool_bindings: vec![],
        guardrail_rules: vec![],
        context_window_strategy: "sliding".to_string(),
        pipeline: None,
        handoff_to: None,
        variables: {
            let mut m = HashMap::new();
            m.insert("name".to_string(), "Alice".to_string());
            // "place" is intentionally missing
            m
        },
    };

    match validate_definition(&undefined_var) {
        Ok(()) => warn!("Expected validation to fail for undefined variable"),
        Err(e) => info!(error = %e, "Correctly rejected undefined template variable"),
    }

    // -----------------------------------------------------------------------
    // 6. Builder-level validation catches the same issues
    // -----------------------------------------------------------------------
    let builder_err = AssistantDefinitionBuilder::new()
        .display_name("Missing ID")
        .system_prompt_template("Hello")
        .build();

    match builder_err {
        Ok(_) => warn!("Expected builder to reject missing id"),
        Err(e) => info!(error = %e, "Builder correctly rejected missing id"),
    }

    info!("Validation example complete");
}
