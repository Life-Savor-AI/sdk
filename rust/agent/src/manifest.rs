//! Provider manifest types and parsing functions.
//!
//! This module defines the `ProviderManifest` and all supporting types used
//! to describe pluggable providers (LLM, VectorStore, Skill, Assistant).
//! It also provides `parse_manifest`, `parse_manifest_file`, and
//! `validate_manifest` for loading and validating TOML manifests.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

// ---------------------------------------------------------------------------
// Provider Manifest types
// ---------------------------------------------------------------------------

/// The top-level provider manifest parsed from a TOML file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderManifest {
    pub provider_type: ProviderType,
    pub instance_name: String,
    pub sdk_version: String,
    pub connection: ConnectionConfig,
    pub auth: AuthConfig,
    pub health_check: HealthCheckConfig,
    #[serde(default)]
    pub priority: u32,
    pub locality: Locality,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub capabilities: Option<CapabilityOverrides>,
    #[serde(default)]
    pub cost_limits: Option<CostLimits>,
    #[serde(default)]
    pub sandbox: Option<SandboxConfig>,
    #[serde(default)]
    pub vault_keys: Vec<String>,
    #[serde(default)]
    pub model_aliases: HashMap<String, String>,
}

/// Identifies the category of a pluggable provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Llm,
    VectorStore,
    Skill,
    Assistant,
}

impl fmt::Display for ProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderType::Llm => write!(f, "llm"),
            ProviderType::VectorStore => write!(f, "vector_store"),
            ProviderType::Skill => write!(f, "skill"),
            ProviderType::Assistant => write!(f, "assistant"),
        }
    }
}

/// Whether the provider runs locally or connects to a remote service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Locality {
    Local,
    Remote,
}

/// Connection parameters — which fields are required depends on `ProviderType`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionConfig {
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub database_url: Option<String>,
    #[serde(default)]
    pub extension_path: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub transport: Option<String>,
}

/// How the provider authenticates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthConfig {
    pub source: CredentialSource,
    #[serde(default)]
    pub key_name: Option<String>,
    #[serde(default)]
    pub env_var: Option<String>,
    #[serde(default)]
    pub secret_arn: Option<String>,
    #[serde(default)]
    pub file_path: Option<String>,
}

/// Where credentials are sourced from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialSource {
    Vault,
    Env,
    AwsSecretsManager,
    File,
    None,
}

/// Health-check configuration for a provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    #[serde(default = "default_health_interval")]
    pub interval_seconds: u64,
    #[serde(default = "default_health_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_failure_threshold")]
    pub consecutive_failures_threshold: u32,
    pub method: HealthCheckMethod,
}

fn default_health_interval() -> u64 {
    30
}
fn default_health_timeout() -> u64 {
    5
}
fn default_failure_threshold() -> u32 {
    3
}

/// The mechanism used to check provider health.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthCheckMethod {
    HttpGet { url: String },
    ConnectionPing,
    CapabilityProbe,
}

/// Optional cost limits for a provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostLimits {
    #[serde(default)]
    pub max_requests_per_hour: Option<u64>,
    #[serde(default)]
    pub max_tokens_per_hour: Option<u64>,
    #[serde(default)]
    pub max_cost_per_day_usd: Option<f64>,
    #[serde(default = "default_warning_threshold")]
    pub warning_threshold_pct: u8,
}

fn default_warning_threshold() -> u8 {
    80
}

/// Optional capability overrides declared in the manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityOverrides {
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub max_context_window: Option<u64>,
    #[serde(default)]
    pub supported_models: Vec<String>,
}

/// Optional sandbox configuration for child-process providers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SandboxConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub allowed_env_vars: Vec<String>,
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub max_memory_mb: Option<u64>,
    #[serde(default)]
    pub max_cpu_seconds: Option<u64>,
    #[serde(default)]
    pub max_output_bytes: Option<u64>,
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Structured validation errors
// ---------------------------------------------------------------------------

/// A structured validation error for a provider manifest.
#[derive(Debug, Clone, PartialEq)]
pub struct ManifestValidationError {
    /// Path to the manifest file that failed validation.
    pub file_path: String,
    /// The field that caused the error (dot-separated for nested fields).
    pub field_name: String,
    /// Human-readable description of the problem.
    pub description: String,
}

impl fmt::Display for ManifestValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: field '{}' — {}",
            self.file_path, self.field_name, self.description
        )
    }
}

impl std::error::Error for ManifestValidationError {}

// ---------------------------------------------------------------------------
// Parsing and validation
// ---------------------------------------------------------------------------

/// Parse a TOML string into a `ProviderManifest`, returning structured errors
/// on failure.
pub fn parse_manifest(
    toml_content: &str,
    file_path: &str,
) -> Result<ProviderManifest, Vec<ManifestValidationError>> {
    let manifest: ProviderManifest =
        toml::from_str(toml_content).map_err(|e| toml_parse_error(e, file_path))?;

    validate_manifest(&manifest, file_path)?;
    Ok(manifest)
}

/// Parse a TOML file from disk into a `ProviderManifest`.
pub fn parse_manifest_file(
    path: &Path,
) -> Result<ProviderManifest, Vec<ManifestValidationError>> {
    let file_path = path.display().to_string();
    let content = std::fs::read_to_string(path).map_err(|e| {
        vec![ManifestValidationError {
            file_path: file_path.clone(),
            field_name: String::new(),
            description: format!("failed to read file: {e}"),
        }]
    })?;
    parse_manifest(&content, &file_path)
}

/// Validate a parsed manifest's fields according to provider-type-specific
/// rules. Returns `Ok(())` when valid, or a list of all detected errors.
pub fn validate_manifest(
    manifest: &ProviderManifest,
    file_path: &str,
) -> Result<(), Vec<ManifestValidationError>> {
    let mut errors = Vec::new();

    // Required top-level fields
    if manifest.instance_name.is_empty() {
        errors.push(ManifestValidationError {
            file_path: file_path.to_string(),
            field_name: "instance_name".to_string(),
            description: "instance_name must not be empty".to_string(),
        });
    }
    if manifest.sdk_version.is_empty() {
        errors.push(ManifestValidationError {
            file_path: file_path.to_string(),
            field_name: "sdk_version".to_string(),
            description: "sdk_version must not be empty".to_string(),
        });
    }

    // Provider-type-specific connection requirements
    validate_connection(&manifest.provider_type, &manifest.connection, file_path, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate connection parameters based on provider type.
fn validate_connection(
    provider_type: &ProviderType,
    conn: &ConnectionConfig,
    file_path: &str,
    errors: &mut Vec<ManifestValidationError>,
) {
    match provider_type {
        ProviderType::Llm => {
            if conn.base_url.is_none() {
                errors.push(ManifestValidationError {
                    file_path: file_path.to_string(),
                    field_name: "connection.base_url".to_string(),
                    description: "LLM providers require a base_url".to_string(),
                });
            }
        }
        ProviderType::VectorStore => {
            if conn.database_url.is_none() && conn.extension_path.is_none() {
                errors.push(ManifestValidationError {
                    file_path: file_path.to_string(),
                    field_name: "connection.database_url".to_string(),
                    description:
                        "VectorStore providers require either database_url or extension_path"
                            .to_string(),
                });
            }
        }
        ProviderType::Skill => {
            if conn.command.is_none() {
                errors.push(ManifestValidationError {
                    file_path: file_path.to_string(),
                    field_name: "connection.command".to_string(),
                    description: "Skill providers require a command".to_string(),
                });
            }
        }
        ProviderType::Assistant => {
            // No required connection params for assistant providers.
        }
    }
}

/// Convert a TOML deserialization error into a structured validation error.
fn toml_parse_error(
    err: toml::de::Error,
    file_path: &str,
) -> Vec<ManifestValidationError> {
    let err_msg = err.message().to_string();
    let description = if err_msg.contains("unknown variant") {
        format!(
            "{} — valid provider types are: llm, vector_store, skill, assistant",
            err_msg
        )
    } else {
        err_msg
    };

    let span = err.span();
    let field_name = span
        .map(|s| format!("byte range {}..{}", s.start, s.end))
        .unwrap_or_default();

    vec![ManifestValidationError {
        file_path: file_path.to_string(),
        field_name,
        description,
    }]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // -- Test helpers -----------------------------------------------------

    /// Helper: minimal valid LLM manifest TOML.
    fn valid_llm_toml() -> String {
        r#"
            provider_type = "llm"
            instance_name = "ollama-default"
            sdk_version = "0.5.0"
            priority = 10
            locality = "local"
            depends_on = []
            vault_keys = []

            [connection]
            base_url = "http://localhost:11434"

            [auth]
            source = "none"

            [health_check]
            interval_seconds = 30
            timeout_seconds = 5
            consecutive_failures_threshold = 3
            method = "connection_ping"
        "#
        .to_string()
    }

    /// Helper: minimal valid VectorStore manifest TOML.
    fn valid_vector_store_toml() -> String {
        r#"
            provider_type = "vector_store"
            instance_name = "sqlite-vec-default"
            sdk_version = "0.5.0"
            priority = 5
            locality = "local"

            [connection]
            extension_path = "/usr/lib/sqlite-vec.so"

            [auth]
            source = "none"

            [health_check]
            method = "connection_ping"
        "#
        .to_string()
    }

    /// Helper: minimal valid Skill manifest TOML.
    fn valid_skill_toml() -> String {
        r#"
            provider_type = "skill"
            instance_name = "weather-skill"
            sdk_version = "0.5.0"
            priority = 1
            locality = "local"

            [connection]
            command = "/usr/local/bin/weather-skill"
            args = ["--port", "8080"]
            transport = "stdio"

            [auth]
            source = "env"
            env_var = "WEATHER_API_KEY"

            [health_check]
            method = "capability_probe"
        "#
        .to_string()
    }

    /// Helper: minimal valid Assistant manifest TOML.
    fn valid_assistant_toml() -> String {
        r#"
            provider_type = "assistant"
            instance_name = "local-fs-assistant"
            sdk_version = "0.5.0"
            priority = 1
            locality = "local"

            [connection]

            [auth]
            source = "none"

            [health_check]
            method = "capability_probe"
        "#
        .to_string()
    }

    // -- Unit tests: parsing valid manifests ------------------------------

    #[test]
    fn parse_valid_llm_manifest() {
        let manifest = parse_manifest(&valid_llm_toml(), "ollama.toml").unwrap();
        assert_eq!(manifest.provider_type, ProviderType::Llm);
        assert_eq!(manifest.instance_name, "ollama-default");
        assert_eq!(manifest.connection.base_url.as_deref(), Some("http://localhost:11434"));
        assert_eq!(manifest.locality, Locality::Local);
        assert_eq!(manifest.priority, 10);
    }

    #[test]
    fn parse_valid_vector_store_manifest() {
        let manifest = parse_manifest(&valid_vector_store_toml(), "sqlite-vec.toml").unwrap();
        assert_eq!(manifest.provider_type, ProviderType::VectorStore);
        assert_eq!(
            manifest.connection.extension_path.as_deref(),
            Some("/usr/lib/sqlite-vec.so")
        );
    }

    #[test]
    fn parse_valid_skill_manifest() {
        let manifest = parse_manifest(&valid_skill_toml(), "weather.toml").unwrap();
        assert_eq!(manifest.provider_type, ProviderType::Skill);
        assert_eq!(
            manifest.connection.command.as_deref(),
            Some("/usr/local/bin/weather-skill")
        );
        assert_eq!(manifest.auth.source, CredentialSource::Env);
        assert_eq!(manifest.auth.env_var.as_deref(), Some("WEATHER_API_KEY"));
    }

    #[test]
    fn parse_valid_assistant_manifest() {
        let manifest = parse_manifest(&valid_assistant_toml(), "assistant.toml").unwrap();
        assert_eq!(manifest.provider_type, ProviderType::Assistant);
    }

    // -- Unit tests: validation errors ------------------------------------

    #[test]
    fn reject_unknown_provider_type() {
        let toml = r#"
            provider_type = "database"
            instance_name = "pg"
            sdk_version = "0.5.0"
            priority = 1
            locality = "local"

            [connection]
            base_url = "http://localhost"

            [auth]
            source = "none"

            [health_check]
            method = "connection_ping"
        "#;
        let errs = parse_manifest(toml, "bad.toml").unwrap_err();
        assert!(!errs.is_empty());
        let msg = &errs[0].description;
        assert!(msg.contains("unknown variant"), "expected unknown variant error, got: {msg}");
        assert!(msg.contains("llm"), "error should list valid types, got: {msg}");
    }

    #[test]
    fn reject_llm_missing_base_url() {
        let toml = r#"
            provider_type = "llm"
            instance_name = "bad-llm"
            sdk_version = "0.5.0"
            priority = 1
            locality = "remote"

            [connection]

            [auth]
            source = "none"

            [health_check]
            method = "connection_ping"
        "#;
        let errs = parse_manifest(toml, "bad-llm.toml").unwrap_err();
        assert!(errs.iter().any(|e| e.field_name == "connection.base_url"));
    }

    #[test]
    fn reject_vector_store_missing_db_and_ext() {
        let toml = r#"
            provider_type = "vector_store"
            instance_name = "bad-vs"
            sdk_version = "0.5.0"
            priority = 1
            locality = "local"

            [connection]

            [auth]
            source = "none"

            [health_check]
            method = "connection_ping"
        "#;
        let errs = parse_manifest(toml, "bad-vs.toml").unwrap_err();
        assert!(errs.iter().any(|e| e.field_name == "connection.database_url"));
    }

    #[test]
    fn reject_skill_missing_command() {
        let toml = r#"
            provider_type = "skill"
            instance_name = "bad-skill"
            sdk_version = "0.5.0"
            priority = 1
            locality = "local"

            [connection]

            [auth]
            source = "none"

            [health_check]
            method = "capability_probe"
        "#;
        let errs = parse_manifest(toml, "bad-skill.toml").unwrap_err();
        assert!(errs.iter().any(|e| e.field_name == "connection.command"));
    }

    #[test]
    fn validation_errors_contain_file_path_and_field() {
        let toml = r#"
            provider_type = "llm"
            instance_name = ""
            sdk_version = ""
            priority = 1
            locality = "local"

            [connection]

            [auth]
            source = "none"

            [health_check]
            method = "connection_ping"
        "#;
        let errs = parse_manifest(toml, "providers/broken.toml").unwrap_err();
        assert!(errs.len() >= 3, "expected at least 3 errors, got {}", errs.len());
        for err in &errs {
            assert_eq!(err.file_path, "providers/broken.toml");
            assert!(!err.description.is_empty());
        }
        assert!(errs.iter().any(|e| e.field_name == "instance_name"));
        assert!(errs.iter().any(|e| e.field_name == "sdk_version"));
        assert!(errs.iter().any(|e| e.field_name == "connection.base_url"));
    }

    // -- Unit tests: round-trip -------------------------------------------

    #[test]
    fn round_trip_llm_manifest() {
        let original = parse_manifest(&valid_llm_toml(), "test.toml").unwrap();
        let serialized = toml::to_string(&original).expect("serialize");
        let reparsed = parse_manifest(&serialized, "test.toml").unwrap();
        assert_eq!(original, reparsed);
    }

    #[test]
    fn round_trip_vector_store_manifest() {
        let original = parse_manifest(&valid_vector_store_toml(), "test.toml").unwrap();
        let serialized = toml::to_string(&original).expect("serialize");
        let reparsed = parse_manifest(&serialized, "test.toml").unwrap();
        assert_eq!(original, reparsed);
    }

    // -- Unit tests: optional fields --------------------------------------

    #[test]
    fn parse_manifest_with_cost_limits() {
        let toml = r#"
            provider_type = "llm"
            instance_name = "openai-cloud"
            sdk_version = "0.5.0"
            priority = 5
            locality = "remote"

            [connection]
            base_url = "https://api.openai.com/v1"

            [auth]
            source = "vault"
            key_name = "openai-api-key"

            [health_check]
            method = { http_get = { url = "https://api.openai.com/v1/models" } }

            [cost_limits]
            max_requests_per_hour = 1000
            max_tokens_per_hour = 500000
            max_cost_per_day_usd = 50.0
            warning_threshold_pct = 90
        "#;
        let manifest = parse_manifest(toml, "openai.toml").unwrap();
        let limits = manifest.cost_limits.unwrap();
        assert_eq!(limits.max_requests_per_hour, Some(1000));
        assert_eq!(limits.max_tokens_per_hour, Some(500000));
        assert_eq!(limits.warning_threshold_pct, 90);
    }

    #[test]
    fn parse_manifest_with_model_aliases() {
        let toml = r#"
            provider_type = "llm"
            instance_name = "ollama-aliased"
            sdk_version = "0.5.0"
            priority = 10
            locality = "local"

            [connection]
            base_url = "http://localhost:11434"

            [auth]
            source = "none"

            [health_check]
            method = "connection_ping"

            [model_aliases]
            fast = "llama3:8b"
            smart = "llama3:70b"
        "#;
        let manifest = parse_manifest(toml, "aliased.toml").unwrap();
        assert_eq!(manifest.model_aliases.get("fast").unwrap(), "llama3:8b");
        assert_eq!(manifest.model_aliases.get("smart").unwrap(), "llama3:70b");
    }

    #[test]
    fn display_for_manifest_validation_error() {
        let err = ManifestValidationError {
            file_path: "providers/bad.toml".to_string(),
            field_name: "connection.base_url".to_string(),
            description: "LLM providers require a base_url".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("providers/bad.toml"));
        assert!(display.contains("connection.base_url"));
        assert!(display.contains("LLM providers require a base_url"));
    }

    #[test]
    fn accept_vector_store_with_database_url() {
        let toml = r#"
            provider_type = "vector_store"
            instance_name = "pgvector"
            sdk_version = "0.5.0"
            priority = 1
            locality = "remote"

            [connection]
            database_url = "postgres://localhost/vectors"

            [auth]
            source = "vault"
            key_name = "pgvector-creds"

            [health_check]
            method = "connection_ping"
        "#;
        let manifest = parse_manifest(toml, "pgvector.toml").unwrap();
        assert_eq!(manifest.provider_type, ProviderType::VectorStore);
    }

    // -- Proptest strategies -----------------------------------------------

    fn arb_provider_type() -> impl Strategy<Value = ProviderType> {
        prop_oneof![
            Just(ProviderType::Llm),
            Just(ProviderType::VectorStore),
            Just(ProviderType::Skill),
            Just(ProviderType::Assistant),
        ]
    }

    fn arb_locality() -> impl Strategy<Value = Locality> {
        prop_oneof![Just(Locality::Local), Just(Locality::Remote)]
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

    fn arb_connection_config(pt: ProviderType) -> impl Strategy<Value = ConnectionConfig> {
        // Generate connection configs that satisfy provider-type requirements
        match pt {
            ProviderType::Llm => {
                (
                    "[a-z]{1,10}://[a-z]{1,10}",
                    proptest::option::of("[a-z]{1,10}"),
                )
                    .prop_map(|(url, region)| ConnectionConfig {
                        base_url: Some(url),
                        region,
                        database_url: None,
                        extension_path: None,
                        command: None,
                        args: None,
                        transport: None,
                    })
                    .boxed()
            }
            ProviderType::VectorStore => {
                prop_oneof![
                    "[a-z]{1,10}://[a-z]{1,10}".prop_map(|url| ConnectionConfig {
                        base_url: None,
                        region: None,
                        database_url: Some(url),
                        extension_path: None,
                        command: None,
                        args: None,
                        transport: None,
                    }),
                    "[a-z/.]{1,20}".prop_map(|path| ConnectionConfig {
                        base_url: None,
                        region: None,
                        database_url: None,
                        extension_path: Some(path),
                        command: None,
                        args: None,
                        transport: None,
                    }),
                ]
                .boxed()
            }
            ProviderType::Skill => {
                (
                    "[a-z/_-]{1,20}",
                    proptest::option::of(proptest::collection::vec("[a-z0-9]{1,5}", 0..3)),
                    proptest::option::of("[a-z]{1,10}"),
                )
                    .prop_map(|(cmd, args, transport)| ConnectionConfig {
                        base_url: None,
                        region: None,
                        database_url: None,
                        extension_path: None,
                        command: Some(cmd),
                        args,
                        transport,
                    })
                    .boxed()
            }
            ProviderType::Assistant => {
                Just(ConnectionConfig {
                    base_url: None,
                    region: None,
                    database_url: None,
                    extension_path: None,
                    command: None,
                    args: None,
                    transport: None,
                })
                .boxed()
            }
        }
    }

    fn arb_auth_config() -> impl Strategy<Value = AuthConfig> {
        (
            arb_credential_source(),
            proptest::option::of("[a-z_]{1,10}"),
            proptest::option::of("[A-Z_]{1,10}"),
            proptest::option::of("[a-z:/-]{1,20}"),
            proptest::option::of("[a-z/.]{1,15}"),
        )
            .prop_map(|(source, key_name, env_var, secret_arn, file_path)| AuthConfig {
                source,
                key_name,
                env_var,
                secret_arn,
                file_path,
            })
    }

    fn arb_health_check_method() -> impl Strategy<Value = HealthCheckMethod> {
        prop_oneof![
            "[a-z]{1,10}://[a-z]{1,10}".prop_map(|url| HealthCheckMethod::HttpGet { url }),
            Just(HealthCheckMethod::ConnectionPing),
            Just(HealthCheckMethod::CapabilityProbe),
        ]
    }

    fn arb_health_check_config() -> impl Strategy<Value = HealthCheckConfig> {
        (1u64..3600, 1u64..60, 1u32..20, arb_health_check_method()).prop_map(
            |(interval, timeout, threshold, method)| HealthCheckConfig {
                interval_seconds: interval,
                timeout_seconds: timeout,
                consecutive_failures_threshold: threshold,
                method,
            },
        )
    }

    fn arb_cost_limits() -> impl Strategy<Value = CostLimits> {
        (
            proptest::option::of(1u64..100000),
            proptest::option::of(1u64..1000000),
            proptest::option::of((1u32..10000).prop_map(|n| n as f64)),
            1u8..100,
        )
            .prop_map(
                |(max_req, max_tok, max_cost, warn)| CostLimits {
                    max_requests_per_hour: max_req,
                    max_tokens_per_hour: max_tok,
                    max_cost_per_day_usd: max_cost,
                    warning_threshold_pct: warn,
                },
            )
    }

    fn arb_capability_overrides() -> impl Strategy<Value = CapabilityOverrides> {
        (
            proptest::collection::vec("[a-z]{1,10}", 0..3),
            proptest::option::of(1u64..1000000),
            proptest::collection::vec("[a-z0-9]{1,10}", 0..3),
        )
            .prop_map(|(features, max_ctx, models)| CapabilityOverrides {
                features,
                max_context_window: max_ctx,
                supported_models: models,
            })
    }

    fn arb_sandbox_config() -> impl Strategy<Value = SandboxConfig> {
        (
            any::<bool>(),
            proptest::collection::vec("[A-Z_]{1,10}", 0..3),
            proptest::collection::vec("[a-z/]{1,10}", 0..3),
            proptest::option::of(1u64..8192),
            proptest::option::of(1u64..3600),
            proptest::option::of(1u64..1000000),
        )
            .prop_map(
                |(enabled, env_vars, paths, mem, cpu, output)| SandboxConfig {
                    enabled,
                    allowed_env_vars: env_vars,
                    allowed_paths: paths,
                    max_memory_mb: mem,
                    max_cpu_seconds: cpu,
                    max_output_bytes: output,
                },
            )
    }

    fn arb_provider_manifest() -> impl Strategy<Value = ProviderManifest> {
        arb_provider_type().prop_flat_map(|pt| {
            // Split into two groups to stay within proptest's 12-tuple limit.
            let core = (
                Just(pt),
                "[a-z][a-z0-9_-]{0,15}",  // instance_name (non-empty)
                "[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}", // sdk_version (non-empty)
                arb_connection_config(pt),
                arb_auth_config(),
                arb_health_check_config(),
                0u32..100,
                arb_locality(),
            );
            let extras = (
                proptest::collection::vec("[a-z]{1,10}", 0..3),
                proptest::option::of(arb_capability_overrides()),
                proptest::option::of(arb_cost_limits()),
                proptest::option::of(arb_sandbox_config()),
                proptest::collection::vec("[a-z]{1,10}", 0..3),
                proptest::collection::hash_map("[a-z]{1,8}", "[a-z0-9]{1,10}", 0..3),
            );
            (core, extras).prop_map(
                |(
                    (provider_type, instance_name, sdk_version, connection, auth, health_check, priority, locality),
                    (depends_on, capabilities, cost_limits, sandbox, vault_keys, model_aliases),
                )| {
                    ProviderManifest {
                        provider_type,
                        instance_name,
                        sdk_version,
                        connection,
                        auth,
                        health_check,
                        priority,
                        locality,
                        depends_on,
                        capabilities,
                        cost_limits,
                        sandbox,
                        vault_keys,
                        model_aliases,
                    }
                },
            )
        })
    }

    // -- Property tests ---------------------------------------------------

    proptest! {
        /// Property 2: Manifest TOML parse round-trip
        ///
        /// **Validates: Requirements 6.3**
        ///
        /// For any valid `ProviderManifest` value, serializing it to TOML
        /// and then calling `parse_manifest` on the resulting string produces
        /// a `ProviderManifest` equivalent to the original.
        #[test]
        fn toml_round_trip_manifest(manifest in arb_provider_manifest()) {
            let toml_str = toml::to_string(&manifest).expect("serialize to TOML");
            let reparsed = parse_manifest(&toml_str, "round-trip.toml")
                .expect("parse_manifest should succeed on serialized output");
            prop_assert_eq!(reparsed, manifest);
        }

        /// Property 3: Invalid manifest input produces descriptive error
        ///
        /// **Validates: Requirements 6.4**
        ///
        /// For any string that is not valid TOML or that is valid TOML but
        /// does not conform to the `ProviderManifest` schema, calling
        /// `parse_manifest` returns a `ManifestValidationError` rather than
        /// a valid `ProviderManifest`.
        #[test]
        fn invalid_input_produces_error(input in "\\PC{0,200}") {
            // Skip inputs that happen to be valid manifests
            if let Ok(_) = parse_manifest(&input, "fuzz.toml") {
                // Extremely unlikely but technically possible — not a failure
                return Ok(());
            }
            // The error path was taken — that's the property we're testing
            let errs = parse_manifest(&input, "fuzz.toml").unwrap_err();
            prop_assert!(!errs.is_empty(), "error list must not be empty");
            for err in &errs {
                prop_assert_eq!(&err.file_path, "fuzz.toml");
                prop_assert!(!err.description.is_empty(), "error description must not be empty");
            }
        }
    }
}
