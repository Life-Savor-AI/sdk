//! Workspace-level cross-SDK property tests.
//!
//! **Property 2: Shared type identity across SDK boundaries — ProviderManifest,
//! ErrorChain, StreamingEnvelope are identical Rust types**
//! **Validates: Requirements 1.3, 18.1, 18.2, 18.3, 18.4**
//!
//! **Property 7: Manifest validation rejects invalid manifests (empty
//! instance_name, empty sdk_version, missing connection params)**
//! **Validates: Requirements 6.2**
//!
//! **Property 9: Vault key allowlist enforcement — resolve with key not in
//! allowlist returns AccessDenied**
//! **Validates: Requirements 8.4**

use std::collections::HashMap;
use std::sync::Arc;

use proptest::prelude::*;
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// Property 2: Shared type identity across SDK boundaries
// ---------------------------------------------------------------------------
//
// These are compile-time assertions. If the types re-exported by different
// SDKs were NOT the same Rust type, the assignments below would fail to
// compile. No runtime proptest needed — compilation IS the proof.

/// **Property 2: ProviderManifest is the same Rust type across all four SDKs.**
///
/// **Validates: Requirements 1.3, 18.1, 18.4**
#[test]
fn provider_manifest_type_identity_across_sdks() {
    // Construct via system SDK types
    let manifest = lifesavor_system_sdk::ProviderManifest {
        provider_type: lifesavor_system_sdk::ProviderType::Skill,
        instance_name: "cross-sdk-test".to_string(),
        sdk_version: "0.1.0".to_string(),
        connection: lifesavor_system_sdk::ConnectionConfig {
            base_url: None,
            region: None,
            database_url: None,
            extension_path: None,
            command: Some("/bin/test".to_string()),
            args: None,
            transport: None,
        },
        auth: lifesavor_system_sdk::AuthConfig {
            source: lifesavor_system_sdk::CredentialSource::None,
            key_name: None,
            env_var: None,
            secret_arn: None,
            file_path: None,
        },
        health_check: lifesavor_system_sdk::HealthCheckConfig {
            interval_seconds: 30,
            timeout_seconds: 5,
            consecutive_failures_threshold: 3,
            method: lifesavor_system_sdk::HealthCheckMethod::ConnectionPing,
        },
        priority: 1,
        locality: lifesavor_system_sdk::Locality::Local,
        depends_on: vec![],
        capabilities: None,
        cost_limits: None,
        sandbox: None,
        vault_keys: vec![],
        model_aliases: HashMap::new(),
    };

    // Assign to variables typed by each SDK — compiles only if same type.
    let _model: &lifesavor_model_sdk::ProviderManifest = &manifest;
    let _assistant: &lifesavor_assistant_sdk::ProviderManifest = &manifest;
    let _skill: &lifesavor_skill_sdk::ProviderManifest = &manifest;
    let _system: &lifesavor_system_sdk::ProviderManifest = &manifest;
}

/// **Property 2: ErrorChain is the same Rust type across all four SDKs.**
///
/// **Validates: Requirements 1.3, 18.1, 18.2**
#[test]
fn error_chain_type_identity_across_sdks() {
    let chain = lifesavor_system_sdk::ErrorChain {
        correlation_id: "test-corr".to_string(),
        timestamp: chrono::Utc::now(),
        contexts: vec![],
    };

    let _model: &lifesavor_model_sdk::ErrorChain = &chain;
    let _assistant: &lifesavor_assistant_sdk::ErrorChain = &chain;
    let _skill: &lifesavor_skill_sdk::ErrorChain = &chain;
}

/// **Property 2: StreamingEnvelope is the same Rust type across System and
/// Model SDKs.**
///
/// **Validates: Requirements 1.3, 18.1, 18.3**
#[test]
fn streaming_envelope_type_identity_across_sdks() {
    let envelope = lifesavor_system_sdk::StreamingEnvelope {
        stream_id: "s-1".to_string(),
        sequence: 0,
        content_type: "text/plain".to_string(),
        payload: "hello".to_string(),
        status: lifesavor_system_sdk::StreamStatus::Data,
        metadata: lifesavor_system_sdk::StreamMetadata {
            source_component: "test".to_string(),
            correlation_id: "corr-1".to_string(),
            total_chunks: None,
            extra: HashMap::new(),
        },
    };

    let _model: &lifesavor_model_sdk::StreamingEnvelope = &envelope;
    let _system: &lifesavor_system_sdk::StreamingEnvelope = &envelope;
}

/// **Property 2: ErrorContext and Subsystem are the same Rust type across all
/// four SDKs.**
///
/// **Validates: Requirements 1.3, 18.1, 18.2**
#[test]
fn error_context_and_subsystem_type_identity() {
    let ctx = lifesavor_system_sdk::ErrorContext::new(
        lifesavor_system_sdk::Subsystem::Bridge,
        "TEST_CODE",
        "test message",
    );

    let _model: &lifesavor_model_sdk::ErrorContext = &ctx;
    let _assistant: &lifesavor_assistant_sdk::ErrorContext = &ctx;
    let _skill: &lifesavor_skill_sdk::ErrorContext = &ctx;

    let sub = lifesavor_system_sdk::Subsystem::Provider;
    let _model_sub: &lifesavor_model_sdk::Subsystem = &sub;
    let _skill_sub: &lifesavor_skill_sdk::Subsystem = &sub;
}

// ---------------------------------------------------------------------------
// Property 7: Manifest validation rejects invalid manifests
// ---------------------------------------------------------------------------

/// Strategy for a ProviderType.
fn arb_provider_type() -> impl Strategy<Value = lifesavor_system_sdk::ProviderType> {
    prop::sample::select(vec![
        lifesavor_system_sdk::ProviderType::Llm,
        lifesavor_system_sdk::ProviderType::MemoryStore,
        lifesavor_system_sdk::ProviderType::Skill,
        lifesavor_system_sdk::ProviderType::Assistant,
    ])
}

/// Strategy for a valid base manifest (non-empty required fields, correct
/// connection params for the provider type).
fn arb_valid_manifest() -> impl Strategy<Value = lifesavor_system_sdk::ProviderManifest> {
    (
        arb_provider_type(),
        "[a-zA-Z][a-zA-Z0-9_-]{1,20}",  // instance_name (non-empty)
        "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}",  // sdk_version (non-empty)
        1u32..=1000,
    )
        .prop_map(|(pt, instance_name, sdk_version, priority)| {
            let connection = match pt {
                lifesavor_system_sdk::ProviderType::Llm => lifesavor_system_sdk::ConnectionConfig {
                    base_url: Some("http://localhost:11434".to_string()),
                    region: None,
                    database_url: None,
                    extension_path: None,
                    command: None,
                    args: None,
                    transport: None,
                },
                lifesavor_system_sdk::ProviderType::MemoryStore => lifesavor_system_sdk::ConnectionConfig {
                    base_url: None,
                    region: None,
                    database_url: Some("postgres://localhost/test".to_string()),
                    extension_path: None,
                    command: None,
                    args: None,
                    transport: None,
                },
                lifesavor_system_sdk::ProviderType::Skill => lifesavor_system_sdk::ConnectionConfig {
                    base_url: None,
                    region: None,
                    database_url: None,
                    extension_path: None,
                    command: Some("/bin/skill".to_string()),
                    args: None,
                    transport: None,
                },
                lifesavor_system_sdk::ProviderType::Assistant => lifesavor_system_sdk::ConnectionConfig {
                    base_url: None,
                    region: None,
                    database_url: None,
                    extension_path: None,
                    command: None,
                    args: None,
                    transport: None,
                },
            };

            lifesavor_system_sdk::ProviderManifest {
                provider_type: pt,
                instance_name,
                sdk_version,
                connection,
                auth: lifesavor_system_sdk::AuthConfig {
                    source: lifesavor_system_sdk::CredentialSource::None,
                    key_name: None,
                    env_var: None,
                    secret_arn: None,
                    file_path: None,
                },
                health_check: lifesavor_system_sdk::HealthCheckConfig {
                    interval_seconds: 30,
                    timeout_seconds: 5,
                    consecutive_failures_threshold: 3,
                    method: lifesavor_system_sdk::HealthCheckMethod::ConnectionPing,
                },
                priority,
                locality: lifesavor_system_sdk::Locality::Local,
                depends_on: vec![],
                capabilities: None,
                cost_limits: None,
                sandbox: None,
                vault_keys: vec![],
                model_aliases: HashMap::new(),
            }
        })
}

proptest! {
    /// **Property 7 (empty instance_name): For any manifest with an empty
    /// instance_name, `validate_manifest` returns errors mentioning
    /// "instance_name".**
    ///
    /// **Validates: Requirements 6.2**
    #[test]
    fn validate_manifest_rejects_empty_instance_name(
        mut manifest in arb_valid_manifest(),
    ) {
        manifest.instance_name = String::new();

        let result = lifesavor_system_sdk::validate_manifest(&manifest, "test.toml");
        prop_assert!(result.is_err(), "Should reject empty instance_name");

        let errors = result.unwrap_err();
        prop_assert!(
            errors.iter().any(|e| e.field_name.contains("instance_name")),
            "Errors should mention instance_name, got: {:?}",
            errors
        );
    }

    /// **Property 7 (empty sdk_version): For any manifest with an empty
    /// sdk_version, `validate_manifest` returns errors mentioning
    /// "sdk_version".**
    ///
    /// **Validates: Requirements 6.2**
    #[test]
    fn validate_manifest_rejects_empty_sdk_version(
        mut manifest in arb_valid_manifest(),
    ) {
        manifest.sdk_version = String::new();

        let result = lifesavor_system_sdk::validate_manifest(&manifest, "test.toml");
        prop_assert!(result.is_err(), "Should reject empty sdk_version");

        let errors = result.unwrap_err();
        prop_assert!(
            errors.iter().any(|e| e.field_name.contains("sdk_version")),
            "Errors should mention sdk_version, got: {:?}",
            errors
        );
    }

    /// **Property 7 (missing connection params for LLM): An LLM manifest
    /// without base_url is rejected.**
    ///
    /// **Validates: Requirements 6.2**
    #[test]
    fn validate_manifest_rejects_llm_without_base_url(
        instance_name in "[a-zA-Z][a-zA-Z0-9_-]{1,20}",
        sdk_version in "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}",
    ) {
        let manifest = lifesavor_system_sdk::ProviderManifest {
            provider_type: lifesavor_system_sdk::ProviderType::Llm,
            instance_name,
            sdk_version,
            connection: lifesavor_system_sdk::ConnectionConfig {
                base_url: None, // missing!
                region: None,
                database_url: None,
                extension_path: None,
                command: None,
                args: None,
                transport: None,
            },
            auth: lifesavor_system_sdk::AuthConfig {
                source: lifesavor_system_sdk::CredentialSource::None,
                key_name: None,
                env_var: None,
                secret_arn: None,
                file_path: None,
            },
            health_check: lifesavor_system_sdk::HealthCheckConfig {
                interval_seconds: 30,
                timeout_seconds: 5,
                consecutive_failures_threshold: 3,
                method: lifesavor_system_sdk::HealthCheckMethod::ConnectionPing,
            },
            priority: 1,
            locality: lifesavor_system_sdk::Locality::Local,
            depends_on: vec![],
            capabilities: None,
            cost_limits: None,
            sandbox: None,
            vault_keys: vec![],
            model_aliases: HashMap::new(),
        };

        let result = lifesavor_system_sdk::validate_manifest(&manifest, "test.toml");
        prop_assert!(result.is_err(), "Should reject LLM manifest without base_url");

        let errors = result.unwrap_err();
        prop_assert!(
            errors.iter().any(|e| e.field_name.contains("base_url")),
            "Errors should mention base_url, got: {:?}",
            errors
        );
    }

    /// **Property 7 (missing connection params for Skill): A Skill manifest
    /// without command is rejected.**
    ///
    /// **Validates: Requirements 6.2**
    #[test]
    fn validate_manifest_rejects_skill_without_command(
        instance_name in "[a-zA-Z][a-zA-Z0-9_-]{1,20}",
        sdk_version in "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}",
    ) {
        let manifest = lifesavor_system_sdk::ProviderManifest {
            provider_type: lifesavor_system_sdk::ProviderType::Skill,
            instance_name,
            sdk_version,
            connection: lifesavor_system_sdk::ConnectionConfig {
                base_url: None,
                region: None,
                database_url: None,
                extension_path: None,
                command: None, // missing!
                args: None,
                transport: None,
            },
            auth: lifesavor_system_sdk::AuthConfig {
                source: lifesavor_system_sdk::CredentialSource::None,
                key_name: None,
                env_var: None,
                secret_arn: None,
                file_path: None,
            },
            health_check: lifesavor_system_sdk::HealthCheckConfig {
                interval_seconds: 30,
                timeout_seconds: 5,
                consecutive_failures_threshold: 3,
                method: lifesavor_system_sdk::HealthCheckMethod::ConnectionPing,
            },
            priority: 1,
            locality: lifesavor_system_sdk::Locality::Local,
            depends_on: vec![],
            capabilities: None,
            cost_limits: None,
            sandbox: None,
            vault_keys: vec![],
            model_aliases: HashMap::new(),
        };

        let result = lifesavor_system_sdk::validate_manifest(&manifest, "test.toml");
        prop_assert!(result.is_err(), "Should reject Skill manifest without command");

        let errors = result.unwrap_err();
        prop_assert!(
            errors.iter().any(|e| e.field_name.contains("command")),
            "Errors should mention command, got: {:?}",
            errors
        );
    }

    /// **Property 7 (valid manifests pass): For any valid manifest with
    /// non-empty required fields and correct connection params,
    /// `validate_manifest` succeeds.**
    ///
    /// **Validates: Requirements 6.2**
    #[test]
    fn validate_manifest_accepts_valid_manifests(
        manifest in arb_valid_manifest(),
    ) {
        let result = lifesavor_system_sdk::validate_manifest(&manifest, "test.toml");
        prop_assert!(
            result.is_ok(),
            "Should accept valid manifest, got errors: {:?}",
            result.unwrap_err()
        );
    }
}

// ---------------------------------------------------------------------------
// Property 9: Vault key allowlist enforcement
// ---------------------------------------------------------------------------

/// **Property 9: When a CredentialManager is configured with a vault key
/// allowlist for an actor, resolving a key NOT in the allowlist returns
/// `CredentialError::VaultAccessDenied`.**
///
/// **Validates: Requirements 8.4**
///
/// We test the allowlist enforcement through the CredentialManager + VaultAccessControl
/// integration. The CredentialManager delegates to VaultAccessControl::check_access
/// which enforces the allowlist before any vault I/O.
#[test]
fn vault_key_allowlist_rejects_forbidden_keys() {
    // Use a multi-threaded tokio runtime for the async resolve call.
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        use lifesavor_agent::vault::access_control::{VaultAccessControl, VaultAccessPolicy};
        use lifesavor_agent::providers::credential_manager::CredentialManager;

        // Set up an allowlist: actor "test-provider" may access ["allowed-key-1", "allowed-key-2"]
        let mut ac = VaultAccessControl::new();
        let policy = VaultAccessPolicy::new(
            "test-provider",
            vec!["allowed-key-1".to_string(), "allowed-key-2".to_string()],
        )
        .with_rate_limit(100);
        ac.register_policy(policy);
        let ac = Arc::new(RwLock::new(ac));

        let mgr = CredentialManager::new(None, ac);

        // Attempt to resolve a key NOT in the allowlist → should get VaultAccessDenied
        let auth = lifesavor_system_sdk::AuthConfig {
            source: lifesavor_system_sdk::CredentialSource::Vault,
            key_name: Some("forbidden-key".to_string()),
            env_var: None,
            secret_arn: None,
            file_path: None,
        };

        let err = mgr.resolve(&auth, "test-provider", "corr-cross-1").await.unwrap_err();
        assert!(
            matches!(err, lifesavor_agent::providers::credential_manager::CredentialError::VaultAccessDenied(_)),
            "Expected VaultAccessDenied for forbidden key, got: {err:?}"
        );
    });
}

/// **Property 9 (positive): When a key IS in the allowlist, the resolve call
/// does NOT return AccessDenied (it may return other errors like VaultKeyNotFound
/// since we don't have a real vault, but NOT AccessDenied).**
///
/// **Validates: Requirements 8.4**
#[test]
fn vault_key_allowlist_permits_allowed_keys() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        use lifesavor_agent::vault::access_control::{VaultAccessControl, VaultAccessPolicy};
        use lifesavor_agent::providers::credential_manager::CredentialManager;

        let mut ac = VaultAccessControl::new();
        let policy = VaultAccessPolicy::new(
            "test-provider",
            vec!["my-api-key".to_string()],
        )
        .with_rate_limit(100);
        ac.register_policy(policy);
        let ac = Arc::new(RwLock::new(ac));

        let mgr = CredentialManager::new(None, ac);

        // Attempt to resolve a key that IS in the allowlist
        let auth = lifesavor_system_sdk::AuthConfig {
            source: lifesavor_system_sdk::CredentialSource::Vault,
            key_name: Some("my-api-key".to_string()),
            env_var: None,
            secret_arn: None,
            file_path: None,
        };

        let result = mgr.resolve(&auth, "test-provider", "corr-cross-2").await;
        // The key passes the allowlist check but the vault is None, so we get
        // a VaultError (not VaultAccessDenied).
        match result {
            Err(lifesavor_agent::providers::credential_manager::CredentialError::VaultAccessDenied(_)) => {
                panic!("Should NOT get VaultAccessDenied for an allowed key");
            }
            _ => {
                // Any other result (VaultError, Ok, etc.) is fine — the point
                // is that the allowlist did not block the request.
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Type identity: agent-types ↔ SDK re-export (Phase 3, Req 12.1)
// ---------------------------------------------------------------------------

/// **Validates: Requirements 12.1**
///
/// A `SystemComponentType::Tts` constructed via `lifesavor_agent_types` is the
/// exact same Rust type as one obtained through the `lifesavor_system_sdk`
/// re-export. This is a compile-time + runtime assertion: if the types were
/// different, the equality check would not compile.
#[test]
fn system_component_type_identity_agent_types_vs_sdk() {
    let from_agent_types = lifesavor_agent_types::system_component::SystemComponentType::Tts;
    let from_sdk: lifesavor_system_sdk::SystemComponentType = lifesavor_system_sdk::SystemComponentType::Tts;

    // Compile-time proof: assigning across crate boundaries only works if same type.
    let _cross: &lifesavor_agent_types::system_component::SystemComponentType = &from_sdk;
    let _cross2: &lifesavor_system_sdk::SystemComponentType = &from_agent_types;

    // Runtime equality check.
    assert_eq!(from_agent_types, from_sdk, "SystemComponentType::Tts must be identical across agent-types and SDK");
}
