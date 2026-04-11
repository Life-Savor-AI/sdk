//! Property-based tests for Skill SDK builders and sandbox compliance.
//!
//! **Property 6: ToolSchemaBuilder produces valid schemas for valid inputs, rejects missing fields**
//! **Validates: Requirements 5.5**
//!
//! **Property 4: Provider builders accept matching manifests and reject mismatched types (Skill SDK)**
//! **Validates: Requirements 5.4, 6.5**
//!
//! **Property 12: SandboxComplianceChecker detects violations for disallowed access, no violations for allowed access**
//! **Validates: Requirements 10.2**

use std::collections::HashMap;
use std::path::PathBuf;

use lifesavor_skill_sdk::builder::{SkillProviderBuilder, ToolSchemaBuilder};
use lifesavor_skill_sdk::sandbox_compliance::{ComplianceViolation, SandboxComplianceChecker};
use lifesavor_skill_sdk::{
    AuthConfig, ConnectionConfig, CredentialSource, HealthCheckConfig, HealthCheckMethod,
    Locality, ProviderManifest, ProviderType, SandboxConfig,
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Strategy that generates a valid base `ProviderManifest` with a given `ProviderType`.
fn arb_manifest_with_type(pt: ProviderType) -> impl Strategy<Value = ProviderManifest> {
    (
        "[a-zA-Z][a-zA-Z0-9_-]{0,30}",                                    // instance_name
        "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}",                          // sdk_version
        "/[a-z]{1,10}(/[a-z]{1,10}){0,3}",                                // command path
        1u32..=1000,                                                        // priority
        prop::bool::ANY,                                                    // locality
    )
        .prop_map(move |(instance_name, sdk_version, command, priority, is_local)| {
            ProviderManifest {
                provider_type: pt,
                instance_name,
                sdk_version,
                connection: ConnectionConfig {
                    base_url: None,
                    region: None,
                    database_url: None,
                    extension_path: None,
                    command: Some(command),
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

/// Strategy for a valid Skill manifest.
fn arb_skill_manifest() -> impl Strategy<Value = ProviderManifest> {
    arb_manifest_with_type(ProviderType::Skill)
}

/// Strategy for a manifest with a non-Skill provider type.
fn arb_non_skill_manifest() -> impl Strategy<Value = (ProviderManifest, ProviderType)> {
    prop::sample::select(vec![
        ProviderType::Llm,
        ProviderType::Assistant,
        ProviderType::MemoryStore,
    ])
    .prop_flat_map(|pt| arb_manifest_with_type(pt).prop_map(move |m| (m, pt)))
}

/// Strategy for a non-empty string suitable for tool name / description.
fn arb_non_empty_string() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_-]{0,40}"
}

/// Strategy for a valid JSON object value (for input_schema).
fn arb_json_object() -> impl Strategy<Value = serde_json::Value> {
    prop::collection::vec(
        ("[a-z]{1,10}", "[a-zA-Z0-9 ]{0,20}"),
        0..=4,
    )
    .prop_map(|pairs| {
        let mut map = serde_json::Map::new();
        map.insert("type".to_string(), serde_json::Value::String("object".to_string()));
        if !pairs.is_empty() {
            let mut props = serde_json::Map::new();
            for (key, _) in pairs {
                let mut field = serde_json::Map::new();
                field.insert("type".to_string(), serde_json::Value::String("string".to_string()));
                props.insert(key, serde_json::Value::Object(field));
            }
            map.insert("properties".to_string(), serde_json::Value::Object(props));
        }
        serde_json::Value::Object(map)
    })
}

/// Strategy for a `SandboxConfig` with arbitrary allowed env vars and paths.
fn arb_sandbox_config() -> impl Strategy<Value = SandboxConfig> {
    (
        prop::collection::vec("[A-Z][A-Z0-9_]{0,15}", 0..=5),  // allowed_env_vars
        prop::collection::vec("/[a-z]{1,8}(/[a-z]{1,8}){0,3}", 0..=5),  // allowed_paths
        prop::option::of(1u64..=1_000_000),                     // max_output_bytes
    )
        .prop_map(|(env_vars, paths, max_output)| {
            SandboxConfig {
                enabled: true,
                allowed_env_vars: env_vars,
                allowed_paths: paths,
                max_memory_mb: None,
                max_cpu_seconds: None,
                max_output_bytes: max_output,
            }
        })
}

// ---------------------------------------------------------------------------
// Property 6: ToolSchemaBuilder produces valid schemas for valid inputs,
//             rejects missing fields
// ---------------------------------------------------------------------------

proptest! {
    /// **Property 6 (positive): For any non-empty tool name, non-empty description,
    /// and valid JSON Schema object, `ToolSchemaBuilder::build()` succeeds and
    /// produces a `ToolSchema` with matching fields.**
    ///
    /// **Validates: Requirements 5.5**
    #[test]
    fn tool_schema_builder_succeeds_with_valid_inputs(
        name in arb_non_empty_string(),
        desc in arb_non_empty_string(),
        schema in arb_json_object(),
    ) {
        let result = ToolSchemaBuilder::new()
            .name(&name)
            .description(&desc)
            .input_schema(schema.clone())
            .build();

        prop_assert!(
            result.is_ok(),
            "ToolSchemaBuilder::build() should succeed for name={:?}, desc={:?}, got: {:?}",
            name, desc, result.unwrap_err()
        );

        let tool = result.unwrap();
        prop_assert_eq!(&tool.name, &name);
        prop_assert_eq!(&tool.description, &desc);
        prop_assert!(tool.input_schema.is_object());
    }

    /// **Property 6 (negative — missing name): When name is missing,
    /// `ToolSchemaBuilder::build()` returns an error.**
    ///
    /// **Validates: Requirements 5.5**
    #[test]
    fn tool_schema_builder_rejects_missing_name(
        desc in arb_non_empty_string(),
        schema in arb_json_object(),
    ) {
        let result = ToolSchemaBuilder::new()
            .description(&desc)
            .input_schema(schema)
            .build();

        prop_assert!(
            result.is_err(),
            "ToolSchemaBuilder::build() should fail when name is missing"
        );
        let err_msg = result.unwrap_err().to_string();
        prop_assert!(
            err_msg.contains("name"),
            "Error should mention 'name', got: {err_msg}"
        );
    }

    /// **Property 6 (negative — missing description): When description is missing,
    /// `ToolSchemaBuilder::build()` returns an error.**
    ///
    /// **Validates: Requirements 5.5**
    #[test]
    fn tool_schema_builder_rejects_missing_description(
        name in arb_non_empty_string(),
        schema in arb_json_object(),
    ) {
        let result = ToolSchemaBuilder::new()
            .name(&name)
            .input_schema(schema)
            .build();

        prop_assert!(
            result.is_err(),
            "ToolSchemaBuilder::build() should fail when description is missing"
        );
        let err_msg = result.unwrap_err().to_string();
        prop_assert!(
            err_msg.contains("description"),
            "Error should mention 'description', got: {err_msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Property 4: Provider builders accept matching manifests and reject
    //             mismatched types (Skill SDK)
    // -----------------------------------------------------------------------

    /// **Property 4 (positive): For any valid `ProviderManifest` with
    /// `provider_type == Skill`, `SkillProviderBuilder::new()` succeeds.**
    ///
    /// **Validates: Requirements 5.4**
    #[test]
    fn skill_manifest_accepted(manifest in arb_skill_manifest()) {
        let result = SkillProviderBuilder::new(manifest);
        prop_assert!(
            result.is_ok(),
            "SkillProviderBuilder::new() should accept Skill manifests, got error: {:?}",
            result.unwrap_err()
        );
    }

    /// **Property 4 (negative): For any valid `ProviderManifest` with
    /// `provider_type != Skill`, `SkillProviderBuilder::new()` returns an error.**
    ///
    /// **Validates: Requirements 6.5**
    #[test]
    fn non_skill_manifest_rejected((manifest, pt) in arb_non_skill_manifest()) {
        let result = SkillProviderBuilder::new(manifest);
        prop_assert!(
            result.is_err(),
            "SkillProviderBuilder::new() should reject {:?} manifests",
            pt
        );

        let err_msg = result.unwrap_err().to_string();
        prop_assert!(
            err_msg.contains("provider_type"),
            "Error should mention 'provider_type', got: {err_msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Property 12: SandboxComplianceChecker detects violations for disallowed
    //              access, no violations for allowed access
    // -----------------------------------------------------------------------

    /// **Property 12 (env vars — allowed): For any SandboxConfig and any set of
    /// accessed env vars that are ALL in the allowlist, the checker reports no
    /// violations.**
    ///
    /// **Validates: Requirements 10.2**
    #[test]
    fn sandbox_no_violations_for_allowed_env_vars(config in arb_sandbox_config()) {
        let checker = SandboxComplianceChecker::new(config.clone());
        // Access only variables that are in the allowlist
        let accessed: Vec<String> = config.allowed_env_vars.clone();
        let violations = checker.check_env_vars(&accessed);
        prop_assert!(
            violations.is_empty(),
            "Should report no violations for allowed env vars, got: {:?}",
            violations
        );
    }

    /// **Property 12 (env vars — disallowed): For any SandboxConfig and any
    /// accessed env var NOT in the allowlist, the checker reports exactly one
    /// UndeclaredEnvVar violation per disallowed variable.**
    ///
    /// **Validates: Requirements 10.2**
    #[test]
    fn sandbox_detects_undeclared_env_vars(
        config in arb_sandbox_config(),
        extra_vars in prop::collection::vec("[A-Z]{1,5}_EXTRA_[0-9]{1,3}", 1..=3),
    ) {
        // Ensure extra_vars are NOT in the allowlist
        let disallowed: Vec<String> = extra_vars
            .into_iter()
            .filter(|v| !config.allowed_env_vars.contains(v))
            .collect();

        // Skip if all generated vars happened to be in the allowlist
        prop_assume!(!disallowed.is_empty());

        let checker = SandboxComplianceChecker::new(config);
        let violations = checker.check_env_vars(&disallowed);

        prop_assert_eq!(
            violations.len(),
            disallowed.len(),
            "Should report one violation per disallowed env var"
        );

        for (violation, var) in violations.iter().zip(disallowed.iter()) {
            match violation {
                ComplianceViolation::UndeclaredEnvVar { var_name } => {
                    prop_assert_eq!(var_name, var);
                }
                other => {
                    prop_assert!(false, "Expected UndeclaredEnvVar, got: {:?}", other);
                }
            }
        }
    }

    /// **Property 12 (filesystem — allowed): For any SandboxConfig and any set of
    /// accessed paths that are ALL under an allowed path prefix, the checker
    /// reports no violations.**
    ///
    /// **Validates: Requirements 10.2**
    #[test]
    fn sandbox_no_violations_for_allowed_paths(
        config in arb_sandbox_config(),
        suffixes in prop::collection::vec("[a-z]{1,8}\\.txt", 0..=3),
    ) {
        // Skip if no allowed paths to derive from
        prop_assume!(!config.allowed_paths.is_empty());

        // Build accessed paths that are children of allowed paths
        let accessed: Vec<PathBuf> = suffixes
            .iter()
            .enumerate()
            .map(|(i, suffix)| {
                let base = &config.allowed_paths[i % config.allowed_paths.len()];
                PathBuf::from(base).join(suffix)
            })
            .collect();

        let checker = SandboxComplianceChecker::new(config);
        let violations = checker.check_filesystem(&accessed);
        prop_assert!(
            violations.is_empty(),
            "Should report no violations for paths under allowed prefixes, got: {:?}",
            violations
        );
    }

    /// **Property 12 (filesystem — disallowed): For any SandboxConfig and any
    /// accessed path NOT under any allowed prefix, the checker reports a
    /// DisallowedPath violation.**
    ///
    /// **Validates: Requirements 10.2**
    #[test]
    fn sandbox_detects_disallowed_paths(
        config in arb_sandbox_config(),
        disallowed_paths in prop::collection::vec(
            "/forbidden_[a-z]{1,6}/[a-z]{1,8}", 1..=3
        ),
    ) {
        // Filter to paths that are genuinely not under any allowed prefix
        let truly_disallowed: Vec<PathBuf> = disallowed_paths
            .into_iter()
            .map(PathBuf::from)
            .filter(|p| {
                !config
                    .allowed_paths
                    .iter()
                    .any(|allowed| p.starts_with(allowed))
            })
            .collect();

        prop_assume!(!truly_disallowed.is_empty());

        let checker = SandboxComplianceChecker::new(config);
        let violations = checker.check_filesystem(&truly_disallowed);

        prop_assert_eq!(
            violations.len(),
            truly_disallowed.len(),
            "Should report one violation per disallowed path"
        );

        for violation in &violations {
            match violation {
                ComplianceViolation::DisallowedPath { .. } => {}
                other => {
                    prop_assert!(false, "Expected DisallowedPath, got: {:?}", other);
                }
            }
        }
    }

    /// **Property 12 (output size — allowed): When output size is within the
    /// configured limit (or no limit is set), the checker reports no violation.**
    ///
    /// **Validates: Requirements 10.2**
    #[test]
    fn sandbox_no_violation_for_output_within_limit(
        config in arb_sandbox_config(),
    ) {
        let checker = SandboxComplianceChecker::new(config.clone());
        let size = config.max_output_bytes.map(|l| l as usize).unwrap_or(0);
        let violation = checker.check_output_size(size);
        prop_assert!(
            violation.is_none(),
            "Should report no violation for output at or below limit, got: {:?}",
            violation
        );
    }

    /// **Property 12 (output size — exceeded): When output size exceeds the
    /// configured limit, the checker reports an OutputSizeExceeded violation.**
    ///
    /// **Validates: Requirements 10.2**
    #[test]
    fn sandbox_detects_output_size_exceeded(
        limit in 1u64..=1_000_000u64,
        excess in 1usize..=1000,
    ) {
        let config = SandboxConfig {
            enabled: true,
            allowed_env_vars: vec![],
            allowed_paths: vec![],
            max_memory_mb: None,
            max_cpu_seconds: None,
            max_output_bytes: Some(limit),
        };
        let checker = SandboxComplianceChecker::new(config);
        let size = limit as usize + excess;
        let violation = checker.check_output_size(size);
        prop_assert!(
            violation.is_some(),
            "Should report violation when output exceeds limit"
        );
        match violation.unwrap() {
            ComplianceViolation::OutputSizeExceeded { actual, limit: l } => {
                prop_assert_eq!(actual, size);
                prop_assert_eq!(l, limit);
            }
            other => {
                prop_assert!(false, "Expected OutputSizeExceeded, got: {:?}", other);
            }
        }
    }
}
