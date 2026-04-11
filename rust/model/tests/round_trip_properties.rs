//! Round-trip property tests for Model SDK types.
//!
//! **Property 1: Serialization round-trip for ProviderManifest (TOML)**
//!
//! **Validates: Requirements 6.4, 17.1**

use std::collections::HashMap;

use lifesavor_model_sdk::{
    AuthConfig, ConnectionConfig, CredentialSource, HealthCheckConfig, HealthCheckMethod,
    Locality, ProviderManifest, ProviderType,
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

fn arb_provider_type() -> impl Strategy<Value = ProviderType> {
    prop_oneof![
        Just(ProviderType::Llm),
        Just(ProviderType::MemoryStore),
        Just(ProviderType::Skill),
        Just(ProviderType::Assistant),
    ]
}

fn arb_locality() -> impl Strategy<Value = Locality> {
    prop_oneof![Just(Locality::Local), Just(Locality::Remote),]
}

fn arb_credential_source() -> impl Strategy<Value = CredentialSource> {
    prop_oneof![
        Just(CredentialSource::Vault),
        Just(CredentialSource::Env),
        Just(CredentialSource::AwsSecretsManager),
        Just(CredentialSource::File),
        Just(CredentialSource::None),
    ]
}

fn arb_health_check_method() -> impl Strategy<Value = HealthCheckMethod> {
    prop_oneof![
        "[a-z][a-z0-9:/.-]{1,30}"
            .prop_map(|url| HealthCheckMethod::HttpGet { url }),
        Just(HealthCheckMethod::ConnectionPing),
        Just(HealthCheckMethod::CapabilityProbe),
    ]
}

fn arb_safe_string() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_-]{0,30}"
}

fn arb_provider_manifest() -> impl Strategy<Value = ProviderManifest> {
    (
        arb_provider_type(),
        arb_safe_string(),
        arb_safe_string(),
        arb_locality(),
        arb_credential_source(),
        arb_health_check_method(),
        0u32..100,
    )
        .prop_map(
            |(ptype, instance_name, sdk_version, locality, cred_source, hc_method, priority)| {
                let connection = match ptype {
                    ProviderType::Llm => ConnectionConfig {
                        base_url: Some("http://localhost:11434".to_string()),
                        region: None,
                        database_url: None,
                        extension_path: None,
                        command: None,
                        args: None,
                        transport: None,
                    },
                    ProviderType::MemoryStore => ConnectionConfig {
                        base_url: None,
                        region: None,
                        database_url: Some("postgres://localhost/vectors".to_string()),
                        extension_path: None,
                        command: None,
                        args: None,
                        transport: None,
                    },
                    ProviderType::Skill => ConnectionConfig {
                        base_url: None,
                        region: None,
                        database_url: None,
                        extension_path: None,
                        command: Some("/usr/bin/skill".to_string()),
                        args: None,
                        transport: Some("stdio".to_string()),
                    },
                    ProviderType::Assistant => ConnectionConfig {
                        base_url: None,
                        region: None,
                        database_url: None,
                        extension_path: None,
                        command: None,
                        args: None,
                        transport: None,
                    },
                };

                ProviderManifest {
                    provider_type: ptype,
                    instance_name,
                    sdk_version,
                    connection,
                    auth: AuthConfig {
                        source: cred_source,
                        key_name: None,
                        env_var: None,
                        secret_arn: None,
                        file_path: None,
                    },
                    health_check: HealthCheckConfig {
                        interval_seconds: 30,
                        timeout_seconds: 5,
                        consecutive_failures_threshold: 3,
                        method: hc_method,
                    },
                    priority,
                    locality,
                    depends_on: vec![],
                    capabilities: None,
                    cost_limits: None,
                    sandbox: None,
                    vault_keys: vec![],
                    model_aliases: HashMap::new(),
                }
            },
        )
}

// ---------------------------------------------------------------------------
// Property tests
// ---------------------------------------------------------------------------

proptest! {
    /// **Property 1: ProviderManifest TOML round-trip**
    ///
    /// **Validates: Requirements 6.4, 17.1**
    ///
    /// For any valid ProviderManifest, serializing to TOML then parsing back
    /// SHALL produce an equivalent ProviderManifest.
    #[test]
    fn provider_manifest_toml_round_trip(manifest in arb_provider_manifest()) {
        let toml_str = toml::to_string(&manifest)
            .expect("ProviderManifest should serialize to TOML");
        let deserialized: ProviderManifest = toml::from_str(&toml_str)
            .expect("TOML should deserialize back to ProviderManifest");
        prop_assert_eq!(&manifest, &deserialized);
    }
}
