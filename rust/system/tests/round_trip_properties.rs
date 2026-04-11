//! Round-trip property tests for System SDK types.
//!
//! **Property 1: Serialization round-trip for ProviderManifest (TOML),
//! StreamingEnvelope (JSON)**
//!
//! **Validates: Requirements 6.4, 17.1, 17.4**

use std::collections::HashMap;

use lifesavor_system_sdk::{
    AuthConfig, ConnectionConfig, CredentialSource, HealthCheckConfig, HealthCheckMethod,
    Locality, ProviderManifest, ProviderType, StreamMetadata, StreamStatus, StreamingEnvelope,
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

/// Generate a non-empty alphanumeric string (safe for TOML keys/values).
fn arb_safe_string() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_-]{0,30}"
}

/// Build a valid ProviderManifest with connection params matching the type.
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
                // Ensure connection params satisfy validation for the provider type
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

fn arb_stream_status() -> impl Strategy<Value = StreamStatus> {
    prop_oneof![
        Just(StreamStatus::Data),
        Just(StreamStatus::Complete),
        Just(StreamStatus::Error),
    ]
}

fn arb_streaming_envelope() -> impl Strategy<Value = StreamingEnvelope> {
    (
        arb_safe_string(),
        0u64..10000,
        prop_oneof![
            Just("text/plain".to_string()),
            Just("audio/mpeg".to_string()),
            Just("application/json".to_string()),
        ],
        "[a-zA-Z0-9+/=]{0,100}",
        arb_stream_status(),
        arb_safe_string(),
        arb_safe_string(),
    )
        .prop_map(
            |(stream_id, sequence, content_type, payload, status, source, corr_id)| {
                StreamingEnvelope {
                    stream_id,
                    sequence,
                    content_type,
                    payload,
                    status,
                    metadata: StreamMetadata {
                        source_component: source,
                        correlation_id: corr_id,
                        total_chunks: None,
                        extra: HashMap::new(),
                    },
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

    /// **Property 1: StreamingEnvelope JSON round-trip**
    ///
    /// **Validates: Requirements 17.4**
    ///
    /// For any valid StreamingEnvelope, serializing to JSON then parsing back
    /// SHALL produce an equivalent StreamingEnvelope.
    #[test]
    fn streaming_envelope_json_round_trip(envelope in arb_streaming_envelope()) {
        let json_str = serde_json::to_string(&envelope)
            .expect("StreamingEnvelope should serialize to JSON");
        let deserialized: StreamingEnvelope = serde_json::from_str(&json_str)
            .expect("JSON should deserialize back to StreamingEnvelope");
        prop_assert_eq!(&envelope, &deserialized);
    }
}
