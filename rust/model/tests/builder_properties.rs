//! Property-based tests for `ModelProviderBuilder` type matching.
//!
//! **Property 4: Provider builders accept matching manifests and reject mismatched types (Model SDK)**
//!
//! **Validates: Requirements 3.4, 6.5**

use std::collections::HashMap;

use lifesavor_model_sdk::builder::ModelProviderBuilder;
use lifesavor_model_sdk::{
    AuthConfig, ConnectionConfig, CredentialSource, HealthCheckConfig, HealthCheckMethod,
    Locality, ProviderManifest, ProviderType,
};
use proptest::prelude::*;

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

/// Strategy that generates a valid `ProviderManifest` with `ProviderType::Llm`.
fn arb_llm_manifest() -> impl Strategy<Value = ProviderManifest> {
    arb_manifest_with_type(ProviderType::Llm)
}

/// Strategy that generates a valid `ProviderManifest` with a non-Llm provider type.
fn arb_non_llm_manifest() -> impl Strategy<Value = (ProviderManifest, ProviderType)> {
    prop::sample::select(vec![
        ProviderType::Skill,
        ProviderType::Assistant,
        ProviderType::MemoryStore,
    ])
    .prop_flat_map(|pt| arb_manifest_with_type(pt).prop_map(move |m| (m, pt)))
}

proptest! {
    /// **Property 4 (positive): For any valid `ProviderManifest` with `provider_type == Llm`,
    /// `ModelProviderBuilder::new()` succeeds.**
    ///
    /// **Validates: Requirements 3.4**
    #[test]
    fn llm_manifest_accepted(manifest in arb_llm_manifest()) {
        let result = ModelProviderBuilder::new(manifest);
        prop_assert!(
            result.is_ok(),
            "ModelProviderBuilder::new() should accept LLM manifests, got error: {:?}",
            result.unwrap_err()
        );
    }

    /// **Property 4 (negative): For any valid `ProviderManifest` with `provider_type != Llm`,
    /// `ModelProviderBuilder::new()` returns an error.**
    ///
    /// **Validates: Requirements 6.5**
    #[test]
    fn non_llm_manifest_rejected((manifest, pt) in arb_non_llm_manifest()) {
        let result = ModelProviderBuilder::new(manifest);
        prop_assert!(
            result.is_err(),
            "ModelProviderBuilder::new() should reject {:?} manifests",
            pt
        );

        let err_msg = result.unwrap_err().to_string();
        prop_assert!(
            err_msg.contains("provider_type"),
            "Error should mention 'provider_type', got: {err_msg}"
        );
    }
}
