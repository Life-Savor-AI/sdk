//! Sandbox compliance example for the Model SDK.
//!
//! Demonstrates how to declare sandbox constraints in a `ProviderManifest`
//! for a child-process-based LLM provider and generate a
//! `SecuritySurfaceReport` for QA review.
//!
//! Child-process LLM providers are sandboxed by the agent's `ProcessSandbox`.
//! The manifest's `sandbox` section declares which environment variables,
//! filesystem paths, and output limits the provider requires.
//!
//! Run with: `cargo run --example sandbox_compliance`

use std::collections::HashMap;

use lifesavor_model_sdk::prelude::*;
use lifesavor_model_sdk::security_surface::generate_security_report;
use lifesavor_model_sdk::{
    ConnectionConfig, HealthCheckConfig, HealthCheckMethod, Locality,
    AuthConfig, CredentialSource, SandboxConfig,
};

/// Build a manifest with sandbox constraints for a child-process LLM provider.
#[instrument]
fn build_sandboxed_manifest() -> ProviderManifest {
    ProviderManifest {
        provider_type: ProviderType::Llm,
        instance_name: "sandboxed-llm".to_string(),
        sdk_version: "0.1.0".to_string(),
        connection: ConnectionConfig {
            base_url: Some("http://localhost:8080".to_string()),
            region: None,
            database_url: None,
            extension_path: None,
            command: Some("/opt/llm-provider/serve".to_string()),
            args: Some(vec!["--port".to_string(), "8080".to_string()]),
            transport: None,
        },
        auth: AuthConfig {
            source: CredentialSource::Vault,
            key_name: Some("LLM_API_KEY".to_string()),
            env_var: None,
            secret_arn: None,
            file_path: None,
        },
        health_check: HealthCheckConfig {
            interval_seconds: 30,
            timeout_seconds: 5,
            consecutive_failures_threshold: 3,
            method: HealthCheckMethod::HttpGet {
                url: "http://localhost:8080/health".to_string(),
            },
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
                "LLM_API_KEY".to_string(),
                "CUDA_VISIBLE_DEVICES".to_string(),
            ],
            allowed_paths: vec![
                "/opt/llm-provider".to_string(),
                "/tmp/llm-cache".to_string(),
                "/var/lib/models".to_string(),
            ],
            max_memory_mb: None,
            max_cpu_seconds: None,
            max_output_bytes: Some(10_485_760), // 10 MiB
        }),
        vault_keys: vec!["LLM_API_KEY".to_string()],
        model_aliases: HashMap::new(),
    }
}

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("Sandbox compliance example — Model SDK");

    let manifest = build_sandboxed_manifest();
    info!(
        instance = %manifest.instance_name,
        sandbox_enabled = manifest.sandbox.as_ref().map_or(false, |s| s.enabled),
        "Manifest constructed with sandbox constraints"
    );

    // Generate the security surface report for QA review.
    let report = generate_security_report(&manifest);

    // JSON format for automated tooling.
    let json = report.to_json();
    info!("Security surface report (JSON):");
    println!("{json}");

    // Markdown format for human review.
    let markdown = report.to_markdown();
    info!("Security surface report (Markdown):");
    println!("{markdown}");

    // Verify expected declarations.
    assert_eq!(report.vault_keys, vec!["LLM_API_KEY"]);
    assert_eq!(
        report.env_vars,
        vec!["HOME", "PATH", "LLM_API_KEY", "CUDA_VISIBLE_DEVICES"]
    );
    assert_eq!(
        report.filesystem_paths,
        vec!["/opt/llm-provider", "/tmp/llm-cache", "/var/lib/models"]
    );
    assert_eq!(report.network_endpoints, vec!["http://localhost:8080"]);
    assert_eq!(report.max_output_bytes, Some(10_485_760));

    info!("All sandbox constraint assertions passed");
    info!("Sandbox compliance example complete");
}
