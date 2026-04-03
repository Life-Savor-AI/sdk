//! Sandbox compliance example.
//!
//! Demonstrates how to declare sandbox constraints in a `ProviderManifest`
//! and verify compliance using the `SecuritySurfaceReport`. System components
//! run in-process with privileged access and are NOT sandboxed, but they
//! interact with sandboxed providers that must declare their constraints.
//!
//! This example shows:
//! - Constructing a manifest with sandbox configuration
//! - Generating a security surface report for QA review
//! - Inspecting the report in JSON and Markdown formats
//!
//! Run with: `cargo run --example sandbox_compliance`

use std::collections::HashMap;

use lifesavor_system_sdk::prelude::*;
use lifesavor_system_sdk::security_surface::generate_security_report;
use lifesavor_system_sdk::{
    ConnectionConfig, HealthCheckConfig, HealthCheckMethod, Locality,
};

/// Build a sample `ProviderManifest` with sandbox constraints declared.
#[instrument]
fn build_manifest_with_sandbox() -> ProviderManifest {
    ProviderManifest {
        provider_type: ProviderType::Skill,
        instance_name: "example-sandboxed-skill".to_string(),
        sdk_version: "0.1.0".to_string(),
        connection: ConnectionConfig {
            base_url: Some("https://api.example.com".to_string()),
            region: None,
            database_url: None,
            extension_path: None,
            command: Some("/usr/local/bin/my-skill".to_string()),
            args: None,
            transport: None,
        },
        auth: AuthConfig {
            source: CredentialSource::Vault,
            key_name: Some("MY_API_KEY".to_string()),
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
        sandbox: Some(SandboxConfig {
            enabled: true,
            allowed_env_vars: vec![
                "HOME".to_string(),
                "PATH".to_string(),
                "MY_API_KEY".to_string(),
            ],
            allowed_paths: vec![
                "/tmp/my-skill".to_string(),
                "/var/data/cache".to_string(),
            ],
            max_memory_mb: None,
            max_cpu_seconds: None,
            max_output_bytes: Some(1_048_576), // 1 MiB
        }),
        vault_keys: vec!["MY_API_KEY".to_string(), "DB_PASSWORD".to_string()],
        model_aliases: HashMap::new(),
    }
}

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("Sandbox compliance example — security surface report generation");

    // Build a manifest with sandbox constraints.
    let manifest = build_manifest_with_sandbox();
    info!(
        instance = %manifest.instance_name,
        sandbox_enabled = manifest.sandbox.as_ref().map_or(false, |s| s.enabled),
        "Manifest constructed"
    );

    // Generate the security surface report.
    let report = generate_security_report(&manifest);

    // Display the report in JSON format (for automated QA tooling).
    let json = report.to_json();
    info!("Security surface report (JSON):");
    println!("{json}");

    // Display the report in Markdown format (for human review).
    let markdown = report.to_markdown();
    info!("Security surface report (Markdown):");
    println!("{markdown}");

    // Verify expected fields are present.
    assert_eq!(report.vault_keys, vec!["MY_API_KEY", "DB_PASSWORD"]);
    assert_eq!(report.env_vars, vec!["HOME", "PATH", "MY_API_KEY"]);
    assert_eq!(
        report.filesystem_paths,
        vec!["/tmp/my-skill", "/var/data/cache"]
    );
    assert_eq!(
        report.network_endpoints,
        vec!["https://api.example.com"]
    );
    assert_eq!(report.max_output_bytes, Some(1_048_576));

    info!("All sandbox constraint assertions passed");

    // Note: System components run in-process with privileged access and are
    // NOT sandboxed. The sandbox configuration above applies to third-party
    // providers (skills, model providers, assistant providers) that run as
    // child processes under ProcessSandbox restrictions.
    info!("Sandbox compliance example complete");
}
