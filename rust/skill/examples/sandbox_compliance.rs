//! Sandbox compliance example for the Skill SDK.
//!
//! Demonstrates sandbox constraint declaration in a `ProviderManifest`,
//! compliance checking via `SandboxComplianceChecker`, permission graph
//! integration by handling `SkillProviderError::PermissionDenied`, and
//! security surface report generation.
//!
//! Run with: `cargo run --example sandbox_compliance`

use std::collections::HashMap;
use std::path::PathBuf;

use lifesavor_skill_sdk::prelude::*;
use lifesavor_skill_sdk::sandbox_compliance::{SandboxComplianceChecker, ComplianceViolation};
use lifesavor_skill_sdk::security_surface::generate_security_report;
use lifesavor_skill_sdk::{
    ConnectionConfig, HealthCheckConfig, HealthCheckMethod, Locality,
};

/// Build a manifest with sandbox constraints for a skill provider.
#[instrument]
fn build_sandboxed_manifest() -> ProviderManifest {
    ProviderManifest {
        provider_type: ProviderType::Skill,
        instance_name: "sandboxed-skill".to_string(),
        sdk_version: "0.1.0".to_string(),
        connection: ConnectionConfig {
            base_url: None,
            region: None,
            database_url: None,
            extension_path: None,
            command: Some("/usr/local/bin/sandboxed-skill".to_string()),
            args: None,
            transport: None,
        },
        auth: AuthConfig {
            source: CredentialSource::Vault,
            key_name: Some("SKILL_API_KEY".to_string()),
            env_var: None,
            secret_arn: None,
            file_path: None,
        },
        health_check: HealthCheckConfig {
            interval_seconds: 30,
            timeout_seconds: 5,
            consecutive_failures_threshold: 3,
            method: HealthCheckMethod::CapabilityProbe,
        },
        priority: 50,
        locality: Locality::Local,
        depends_on: vec![],
        capabilities: None,
        cost_limits: None,
        sandbox: Some(SandboxConfig {
            enabled: true,
            allowed_env_vars: vec![
                "HOME".to_string(),
                "PATH".to_string(),
                "SKILL_API_KEY".to_string(),
            ],
            allowed_paths: vec![
                "/tmp/skill-data".to_string(),
                "/var/cache/skill".to_string(),
            ],
            max_memory_mb: None,
            max_cpu_seconds: None,
            max_output_bytes: Some(2_097_152), // 2 MiB
        }),
        vault_keys: vec!["SKILL_API_KEY".to_string()],
        model_aliases: HashMap::new(),
    }
}

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("Sandbox compliance example — Skill SDK");

    let manifest = build_sandboxed_manifest();
    let sandbox_config = manifest.sandbox.clone().expect("sandbox configured");

    info!(
        instance = %manifest.instance_name,
        sandbox_enabled = sandbox_config.enabled,
        "Manifest constructed with sandbox constraints"
    );

    // --- SandboxComplianceChecker ---
    let checker = SandboxComplianceChecker::new(sandbox_config);

    // Check allowed env vars — no violations expected.
    let allowed_vars = vec!["HOME".to_string(), "PATH".to_string()];
    let violations = checker.check_env_vars(&allowed_vars);
    assert!(violations.is_empty());
    info!("Allowed env vars: no violations");

    // Check disallowed env var — violation expected.
    let bad_vars = vec!["HOME".to_string(), "SECRET_TOKEN".to_string()];
    let violations = checker.check_env_vars(&bad_vars);
    assert_eq!(violations.len(), 1);
    assert!(matches!(
        &violations[0],
        ComplianceViolation::UndeclaredEnvVar { var_name } if var_name == "SECRET_TOKEN"
    ));
    info!(var = "SECRET_TOKEN", "Env var violation detected");

    // Check allowed filesystem paths — no violations expected.
    let allowed_paths = vec![
        PathBuf::from("/tmp/skill-data/output.json"),
        PathBuf::from("/var/cache/skill/state.db"),
    ];
    let violations = checker.check_filesystem(&allowed_paths);
    assert!(violations.is_empty());
    info!("Allowed paths: no violations");

    // Check disallowed filesystem path — violation expected.
    let bad_paths = vec![PathBuf::from("/etc/passwd")];
    let violations = checker.check_filesystem(&bad_paths);
    assert_eq!(violations.len(), 1);
    assert!(matches!(
        &violations[0],
        ComplianceViolation::DisallowedPath { path } if path == &PathBuf::from("/etc/passwd")
    ));
    info!(path = "/etc/passwd", "Filesystem violation detected");

    // Check output size — within limit.
    assert!(checker.check_output_size(1_000_000).is_none());
    info!("Output size 1MB: within limit");

    // Check output size — exceeds limit.
    let violation = checker.check_output_size(3_000_000);
    assert!(matches!(
        violation,
        Some(ComplianceViolation::OutputSizeExceeded { actual: 3_000_000, limit: 2_097_152 })
    ));
    info!("Output size 3MB: exceeds 2MiB limit");

    // --- Permission graph integration ---
    // Skills declare required permissions in the manifest. When a permission
    // is denied at runtime, handle SkillProviderError::PermissionDenied.
    let permission_error = SkillProviderError::PermissionDenied(
        "Skill 'sandboxed-skill' lacks 'device_control:write' permission".to_string(),
    );
    warn!(error = %permission_error, "Permission denied — handle gracefully");

    // --- Security surface report ---
    let report = generate_security_report(&manifest);

    let json = report.to_json();
    info!("Security surface report (JSON):");
    println!("{json}");

    let markdown = report.to_markdown();
    info!("Security surface report (Markdown):");
    println!("{markdown}");

    // Verify expected fields.
    assert_eq!(report.vault_keys, vec!["SKILL_API_KEY"]);
    assert_eq!(report.env_vars, vec!["HOME", "PATH", "SKILL_API_KEY"]);
    assert_eq!(
        report.filesystem_paths,
        vec!["/tmp/skill-data", "/var/cache/skill"]
    );
    assert_eq!(report.max_output_bytes, Some(2_097_152));

    info!("All sandbox compliance assertions passed");
    info!("Sandbox compliance example complete");
}
