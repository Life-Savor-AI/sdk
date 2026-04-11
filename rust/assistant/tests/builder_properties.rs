//! Property-based tests for Assistant SDK builders.
//!
//! **Property 5: AssistantDefinitionBuilder enforces required fields**
//!
//! **Validates: Requirements 4.3**
//!
//! **Property 4: Provider builders accept matching manifests and reject mismatched types (Assistant SDK)**
//!
//! **Validates: Requirements 4.4, 6.5**

use std::collections::HashMap;

use lifesavor_assistant_sdk::builder::{AssistantDefinitionBuilder, AssistantProviderBuilder};
use lifesavor_assistant_sdk::{
    AuthConfig, ConnectionConfig, CredentialSource, HealthCheckConfig, HealthCheckMethod,
    Locality, ProviderManifest, ProviderType,
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Strategy that generates a valid base `ProviderManifest` with a given `ProviderType`.
fn arb_manifest_with_type(pt: ProviderType) -> impl Strategy<Value = ProviderManifest> {
    (
        "[a-zA-Z][a-zA-Z0-9_-]{0,30}",   // instance_name
        "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}", // sdk_version (semver-ish)
        "https?://[a-z]{1,20}(\\.[a-z]{1,10}){0,3}(:[0-9]{2,5})?", // base_url
        1u32..=1000,                        // priority
        prop::bool::ANY,                    // locality: Local vs Remote
    )
        .prop_map(move |(instance_name, sdk_version, base_url, priority, is_local)| {
            ProviderManifest {
                provider_type: pt,
                instance_name,
                sdk_version,
                connection: ConnectionConfig {
                    base_url: Some(base_url),
                    region: None,
                    database_url: None,
                    extension_path: None,
                    command: None,
                    args: None,
                    transport: None,
                },
                auth: AuthConfig {
                    source: CredentialSource::None,
                    key_name: None,
                    env_var: None,
                    secret_arn: None,
                    file_path: None,
                },
                health_check: HealthCheckConfig {
                    interval_seconds: 30,
                    timeout_seconds: 5,
                    consecutive_failures_threshold: 3,
                    method: HealthCheckMethod::ConnectionPing,
                },
                priority,
                locality: if is_local { Locality::Local } else { Locality::Remote },
                depends_on: vec![],
                capabilities: None,
                cost_limits: None,
                sandbox: None,
                vault_keys: vec![],
                model_aliases: HashMap::new(),
            }
        })
}

/// Strategy that generates a valid `ProviderManifest` with `ProviderType::Assistant`.
fn arb_assistant_manifest() -> impl Strategy<Value = ProviderManifest> {
    arb_manifest_with_type(ProviderType::Assistant)
}

/// Strategy that generates a valid `ProviderManifest` with a non-Assistant provider type.
fn arb_non_assistant_manifest() -> impl Strategy<Value = (ProviderManifest, ProviderType)> {
    prop::sample::select(vec![
        ProviderType::Llm,
        ProviderType::Skill,
        ProviderType::MemoryStore,
    ])
    .prop_flat_map(|pt| arb_manifest_with_type(pt).prop_map(move |m| (m, pt)))
}

/// Strategy for non-empty strings (used for required builder fields).
fn arb_non_empty_string() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9 _-]{0,40}".prop_map(|s| s.trim().to_string())
        .prop_filter("must be non-empty", |s| !s.is_empty())
}

/// Strategy for a template with variables, returning (template, variables_map).
/// Generates templates like "Hello {{var0}} and {{var1}}" with all variables defined.
fn arb_template_with_vars() -> impl Strategy<Value = (String, HashMap<String, String>)> {
    // Generate 0..=3 variable names, then build a template referencing them
    prop::collection::vec("[a-z][a-z0-9_]{0,10}", 0..=3)
        .prop_flat_map(|var_names| {
            let unique: Vec<String> = var_names
                .into_iter()
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            let names = unique.clone();
            prop::collection::vec("[a-zA-Z0-9 ]{1,20}", names.len())
                .prop_map(move |values| {
                    let mut vars = HashMap::new();
                    let mut template = String::from("System prompt");
                    for (name, value) in names.iter().zip(values.iter()) {
                        template.push_str(&format!(" {{{{{}}}}}", name));
                        vars.insert(name.clone(), value.clone());
                    }
                    (template, vars)
                })
        })
}

// ---------------------------------------------------------------------------
// Property 5: AssistantDefinitionBuilder enforces required fields
// ---------------------------------------------------------------------------

proptest! {
    /// **Property 5 (positive): For any non-empty id, display_name, and
    /// system_prompt_template where all template variables are defined,
    /// `AssistantDefinitionBuilder::build()` succeeds.**
    ///
    /// **Validates: Requirements 4.3**
    #[test]
    fn definition_builder_succeeds_with_valid_inputs(
        id in arb_non_empty_string(),
        display_name in arb_non_empty_string(),
        (template, vars) in arb_template_with_vars(),
    ) {
        let mut builder = AssistantDefinitionBuilder::new()
            .id(&id)
            .display_name(&display_name)
            .system_prompt_template(&template);

        for (key, value) in &vars {
            builder = builder.variable(key, value);
        }

        let result = builder.build();
        prop_assert!(
            result.is_ok(),
            "AssistantDefinitionBuilder::build() should succeed with valid inputs, got error: {:?}",
            result.unwrap_err()
        );

        let def = result.unwrap();
        prop_assert_eq!(&def.id, &id);
        prop_assert_eq!(&def.display_name, &display_name);
        prop_assert_eq!(&def.system_prompt_template, &template);
    }

    /// **Property 5 (negative — missing id): When id is missing/empty,
    /// `AssistantDefinitionBuilder::build()` returns an error.**
    ///
    /// **Validates: Requirements 4.3**
    #[test]
    fn definition_builder_rejects_empty_id(
        display_name in arb_non_empty_string(),
        (template, vars) in arb_template_with_vars(),
    ) {
        let mut builder = AssistantDefinitionBuilder::new()
            .display_name(&display_name)
            .system_prompt_template(&template);

        for (key, value) in &vars {
            builder = builder.variable(key, value);
        }

        let result = builder.build();
        prop_assert!(
            result.is_err(),
            "AssistantDefinitionBuilder::build() should fail when id is missing"
        );
        let err_msg = result.unwrap_err().to_string();
        prop_assert!(
            err_msg.contains("id"),
            "Error should mention 'id', got: {err_msg}"
        );
    }

    /// **Property 5 (negative — missing display_name): When display_name is
    /// missing/empty, `AssistantDefinitionBuilder::build()` returns an error.**
    ///
    /// **Validates: Requirements 4.3**
    #[test]
    fn definition_builder_rejects_empty_display_name(
        id in arb_non_empty_string(),
        (template, vars) in arb_template_with_vars(),
    ) {
        let mut builder = AssistantDefinitionBuilder::new()
            .id(&id)
            .system_prompt_template(&template);

        for (key, value) in &vars {
            builder = builder.variable(key, value);
        }

        let result = builder.build();
        prop_assert!(
            result.is_err(),
            "AssistantDefinitionBuilder::build() should fail when display_name is missing"
        );
        let err_msg = result.unwrap_err().to_string();
        prop_assert!(
            err_msg.contains("display_name"),
            "Error should mention 'display_name', got: {err_msg}"
        );
    }

    /// **Property 5 (negative — missing system_prompt_template): When
    /// system_prompt_template is missing/empty, build() returns an error.**
    ///
    /// **Validates: Requirements 4.3**
    #[test]
    fn definition_builder_rejects_empty_system_prompt_template(
        id in arb_non_empty_string(),
        display_name in arb_non_empty_string(),
    ) {
        let result = AssistantDefinitionBuilder::new()
            .id(&id)
            .display_name(&display_name)
            .build();

        prop_assert!(
            result.is_err(),
            "AssistantDefinitionBuilder::build() should fail when system_prompt_template is missing"
        );
        let err_msg = result.unwrap_err().to_string();
        prop_assert!(
            err_msg.contains("system_prompt_template"),
            "Error should mention 'system_prompt_template', got: {err_msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Property 4: Provider builders accept matching manifests and reject
    //             mismatched types (Assistant SDK)
    // -----------------------------------------------------------------------

    /// **Property 4 (positive): For any valid `ProviderManifest` with
    /// `provider_type == Assistant`, `AssistantProviderBuilder::new()` succeeds.**
    ///
    /// **Validates: Requirements 4.4**
    #[test]
    fn assistant_manifest_accepted(manifest in arb_assistant_manifest()) {
        let result = AssistantProviderBuilder::new(manifest);
        prop_assert!(
            result.is_ok(),
            "AssistantProviderBuilder::new() should accept Assistant manifests, got error: {:?}",
            result.unwrap_err()
        );
    }

    /// **Property 4 (negative): For any valid `ProviderManifest` with
    /// `provider_type != Assistant`, `AssistantProviderBuilder::new()` returns
    /// an error.**
    ///
    /// **Validates: Requirements 6.5**
    #[test]
    fn non_assistant_manifest_rejected((manifest, pt) in arb_non_assistant_manifest()) {
        let result = AssistantProviderBuilder::new(manifest);
        prop_assert!(
            result.is_err(),
            "AssistantProviderBuilder::new() should reject {:?} manifests",
            pt
        );

        let err_msg = result.unwrap_err().to_string();
        prop_assert!(
            err_msg.contains("provider_type"),
            "Error should mention 'provider_type', got: {err_msg}"
        );
    }
}
