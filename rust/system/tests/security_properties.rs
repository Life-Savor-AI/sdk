//! Property-based tests for `SecuritySurfaceReport`.
//!
//! **Property 20: SecuritySurfaceReport extracts all security-relevant fields
//! and round-trips via JSON**
//!
//! **Validates: Requirements 36.2, 36.4**

use std::collections::HashMap;

use lifesavor_system_sdk::security_surface::generate_security_report;
use lifesavor_system_sdk::{
    AuthConfig, ConnectionConfig, CredentialSource, HealthCheckConfig, HealthCheckMethod,
    Locality, ProviderManifest, ProviderType, SandboxConfig,
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Strategy for an optional `SandboxConfig` with arbitrary env vars, paths,
/// and max_output_bytes.
fn arb_sandbox_config() -> impl Strategy<Value = Option<SandboxConfig>> {
    prop::option::of((
        prop::collection::vec("[A-Z][A-Z0-9_]{0,15}", 0..=5),
        prop::collection::vec("/[a-z]{1,8}(/[a-z]{1,8}){0,3}", 0..=5),
        prop::option::of(1u64..=10_000_000),
    ).prop_map(|(env_vars, paths, max_output)| SandboxConfig {
        enabled: true,
        allowed_env_vars: env_vars,
        allowed_paths: paths,
        max_memory_mb: None,
        max_cpu_seconds: None,
        max_output_bytes: max_output,
    }))
}

/// Strategy for an optional base_url.
fn arb_base_url() -> impl Strategy<Value = Option<String>> {
    prop::option::of("https?://[a-z]{1,12}\\.[a-z]{2,4}(:[0-9]{2,5})?(/[a-z]{1,8}){0,3}")
}

/// Strategy for vault keys.
fn arb_vault_keys() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec("[A-Z][A-Z0-9_]{0,20}", 0..=5)
}

/// Strategy that generates a `ProviderManifest` with various combinations of
/// vault_keys, sandbox config, and connection base_url.
fn arb_manifest() -> impl Strategy<Value = ProviderManifest> {
    (
        arb_vault_keys(),
        arb_sandbox_config(),
        arb_base_url(),
    ).prop_map(|(vault_keys, sandbox, base_url)| ProviderManifest {
        provider_type: ProviderType::Skill,
        instance_name: "test-provider".to_string(),
        sdk_version: "0.1.0".to_string(),
        connection: ConnectionConfig {
            base_url,
            region: None,
            database_url: None,
            extension_path: None,
            command: Some("/usr/bin/test".to_string()),
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
            method: HealthCheckMethod::CapabilityProbe,
            interval_seconds: 30,
            timeout_seconds: 5,
            consecutive_failures_threshold: 3,
        },
        priority: 100,
        locality: Locality::Local,
        depends_on: vec![],
        capabilities: None,
        cost_limits: None,
        sandbox,
        vault_keys,
        model_aliases: HashMap::new(),
    })
}

// ---------------------------------------------------------------------------
// Property 20: SecuritySurfaceReport extracts all security-relevant fields
//              and round-trips via JSON
// ---------------------------------------------------------------------------

proptest! {
    /// **Property 20 (extraction): `generate_security_report` correctly extracts
    /// all security-relevant fields from an arbitrary `ProviderManifest`.**
    ///
    /// **Validates: Requirements 36.2**
    #[test]
    fn security_report_extracts_all_fields(manifest in arb_manifest()) {
        let report = generate_security_report(&manifest);

        // vault_keys must match
        prop_assert_eq!(&report.vault_keys, &manifest.vault_keys);

        // env_vars, filesystem_paths, max_output_bytes from sandbox
        match &manifest.sandbox {
            Some(sandbox) => {
                prop_assert_eq!(&report.env_vars, &sandbox.allowed_env_vars);
                prop_assert_eq!(&report.filesystem_paths, &sandbox.allowed_paths);
                prop_assert_eq!(report.max_output_bytes, sandbox.max_output_bytes);
            }
            None => {
                prop_assert!(report.env_vars.is_empty());
                prop_assert!(report.filesystem_paths.is_empty());
                prop_assert_eq!(report.max_output_bytes, None);
            }
        }

        // network_endpoints from connection.base_url
        match &manifest.connection.base_url {
            Some(url) => {
                prop_assert_eq!(&report.network_endpoints, &vec![url.clone()]);
            }
            None => {
                prop_assert!(report.network_endpoints.is_empty());
            }
        }

        // bridge_calls is always empty (not declared in manifest)
        prop_assert!(report.bridge_calls.is_empty());
    }

    /// **Property 20 (JSON round-trip): `to_json()` produces valid JSON that
    /// contains all the report fields, and the parsed JSON matches the report.**
    ///
    /// **Validates: Requirements 36.4**
    #[test]
    fn security_report_json_round_trip(manifest in arb_manifest()) {
        let report = generate_security_report(&manifest);
        let json_str = report.to_json();

        // Must parse as valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| TestCaseError::fail(format!("to_json() produced invalid JSON: {e}")))?;

        // Verify each field in the parsed JSON matches the report
        let vault_keys: Vec<String> = parsed["vault_keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        prop_assert_eq!(&vault_keys, &report.vault_keys);

        let env_vars: Vec<String> = parsed["env_vars"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        prop_assert_eq!(&env_vars, &report.env_vars);

        let fs_paths: Vec<String> = parsed["filesystem_paths"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        prop_assert_eq!(&fs_paths, &report.filesystem_paths);

        let endpoints: Vec<String> = parsed["network_endpoints"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        prop_assert_eq!(&endpoints, &report.network_endpoints);

        let bridge: Vec<String> = parsed["bridge_calls"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        prop_assert_eq!(&bridge, &report.bridge_calls);

        // max_output_bytes: null or u64
        match report.max_output_bytes {
            Some(bytes) => {
                prop_assert_eq!(parsed["max_output_bytes"].as_u64(), Some(bytes));
            }
            None => {
                prop_assert!(parsed["max_output_bytes"].is_null());
            }
        }
    }
}
