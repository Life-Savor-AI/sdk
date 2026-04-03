// System SDK security surface
//!
//! Security surface report generation for QA review of marketplace submissions.
//! Extracts security-relevant declarations from a [`ProviderManifest`] into a
//! structured [`SecuritySurfaceReport`] that serializes to JSON and Markdown.

use crate::ProviderManifest;

/// A structured report of all security-relevant declarations extracted from a
/// component's [`ProviderManifest`] and [`crate::SandboxConfig`].
///
/// Used by QA reviewers and automated tooling to assess a component's access
/// surface without reading all source code.
#[derive(Debug, Clone, PartialEq)]
pub struct SecuritySurfaceReport {
    /// Vault keys the component is allowed to resolve via [`crate::CredentialManager`].
    pub vault_keys: Vec<String>,
    /// Environment variables the component may read (from sandbox config).
    pub env_vars: Vec<String>,
    /// Filesystem paths the component may access (from sandbox config).
    pub filesystem_paths: Vec<String>,
    /// Network endpoints the component connects to (from connection config).
    pub network_endpoints: Vec<String>,
    /// System component bridge calls declared by the component.
    pub bridge_calls: Vec<String>,
    /// Maximum output size in bytes (from sandbox config).
    pub max_output_bytes: Option<u64>,
}

impl SecuritySurfaceReport {
    /// Serialize the report to a JSON string for automated QA tooling.
    pub fn to_json(&self) -> String {
        serde_json::json!({
            "vault_keys": self.vault_keys,
            "env_vars": self.env_vars,
            "filesystem_paths": self.filesystem_paths,
            "network_endpoints": self.network_endpoints,
            "bridge_calls": self.bridge_calls,
            "max_output_bytes": self.max_output_bytes,
        })
        .to_string()
    }

    /// Serialize the report to a human-readable Markdown string for QA review.
    pub fn to_markdown(&self) -> String {
        let mut md = String::from("# Security Surface Report\n\n");

        md.push_str("## Vault Keys\n\n");
        if self.vault_keys.is_empty() {
            md.push_str("_None declared._\n\n");
        } else {
            for key in &self.vault_keys {
                md.push_str(&format!("- `{}`\n", key));
            }
            md.push('\n');
        }

        md.push_str("## Environment Variables\n\n");
        if self.env_vars.is_empty() {
            md.push_str("_None declared._\n\n");
        } else {
            for var in &self.env_vars {
                md.push_str(&format!("- `{}`\n", var));
            }
            md.push('\n');
        }

        md.push_str("## Filesystem Paths\n\n");
        if self.filesystem_paths.is_empty() {
            md.push_str("_None declared._\n\n");
        } else {
            for path in &self.filesystem_paths {
                md.push_str(&format!("- `{}`\n", path));
            }
            md.push('\n');
        }

        md.push_str("## Network Endpoints\n\n");
        if self.network_endpoints.is_empty() {
            md.push_str("_None declared._\n\n");
        } else {
            for endpoint in &self.network_endpoints {
                md.push_str(&format!("- `{}`\n", endpoint));
            }
            md.push('\n');
        }

        md.push_str("## Bridge Calls\n\n");
        if self.bridge_calls.is_empty() {
            md.push_str("_None declared._\n\n");
        } else {
            for call in &self.bridge_calls {
                md.push_str(&format!("- `{}`\n", call));
            }
            md.push('\n');
        }

        md.push_str("## Max Output Bytes\n\n");
        match self.max_output_bytes {
            Some(bytes) => md.push_str(&format!("{}\n", bytes)),
            None => md.push_str("_No limit declared._\n"),
        }

        md
    }
}

/// Extract a [`SecuritySurfaceReport`] from a parsed [`ProviderManifest`].
///
/// Gathers all security-relevant fields:
/// - `vault_keys` from `manifest.vault_keys`
/// - `env_vars` from `manifest.sandbox.allowed_env_vars`
/// - `filesystem_paths` from `manifest.sandbox.allowed_paths`
/// - `network_endpoints` from `manifest.connection.base_url`
/// - `bridge_calls` — empty (bridge calls are not declared in the manifest)
/// - `max_output_bytes` from `manifest.sandbox.max_output_bytes`
pub fn generate_security_report(manifest: &ProviderManifest) -> SecuritySurfaceReport {
    let vault_keys = manifest.vault_keys.clone();

    let (env_vars, filesystem_paths, max_output_bytes) = match &manifest.sandbox {
        Some(sandbox) => (
            sandbox.allowed_env_vars.clone(),
            sandbox.allowed_paths.clone(),
            sandbox.max_output_bytes,
        ),
        None => (Vec::new(), Vec::new(), None),
    };

    let network_endpoints = match &manifest.connection.base_url {
        Some(url) => vec![url.clone()],
        None => Vec::new(),
    };

    // Bridge calls are not declared in the manifest; left empty for now.
    let bridge_calls = Vec::new();

    SecuritySurfaceReport {
        vault_keys,
        env_vars,
        filesystem_paths,
        network_endpoints,
        bridge_calls,
        max_output_bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AuthConfig, ConnectionConfig, CredentialSource, HealthCheckConfig,
        HealthCheckMethod, Locality, ProviderType, SandboxConfig,
    };

    fn minimal_manifest() -> ProviderManifest {
        ProviderManifest {
            provider_type: ProviderType::Skill,
            instance_name: "test-provider".to_string(),
            sdk_version: "0.1.0".to_string(),
            connection: ConnectionConfig {
                base_url: None,
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
            sandbox: None,
            vault_keys: vec![],
            model_aliases: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_empty_manifest_produces_empty_report() {
        let manifest = minimal_manifest();
        let report = generate_security_report(&manifest);
        assert!(report.vault_keys.is_empty());
        assert!(report.env_vars.is_empty());
        assert!(report.filesystem_paths.is_empty());
        assert!(report.network_endpoints.is_empty());
        assert!(report.bridge_calls.is_empty());
        assert_eq!(report.max_output_bytes, None);
    }

    #[test]
    fn test_full_manifest_extracts_all_fields() {
        let mut manifest = minimal_manifest();
        manifest.vault_keys = vec!["API_KEY".to_string(), "DB_PASS".to_string()];
        manifest.connection.base_url = Some("https://api.example.com".to_string());
        manifest.sandbox = Some(SandboxConfig {
            enabled: true,
            allowed_env_vars: vec!["HOME".to_string(), "PATH".to_string()],
            allowed_paths: vec!["/tmp".to_string(), "/data".to_string()],
            max_memory_mb: None,
            max_cpu_seconds: None,
            max_output_bytes: Some(1024),
        });

        let report = generate_security_report(&manifest);
        assert_eq!(report.vault_keys, vec!["API_KEY", "DB_PASS"]);
        assert_eq!(report.env_vars, vec!["HOME", "PATH"]);
        assert_eq!(report.filesystem_paths, vec!["/tmp", "/data"]);
        assert_eq!(report.network_endpoints, vec!["https://api.example.com"]);
        assert!(report.bridge_calls.is_empty());
        assert_eq!(report.max_output_bytes, Some(1024));
    }

    #[test]
    fn test_to_json_round_trips() {
        let mut manifest = minimal_manifest();
        manifest.vault_keys = vec!["SECRET".to_string()];
        manifest.connection.base_url = Some("https://example.com".to_string());
        manifest.sandbox = Some(SandboxConfig {
            enabled: true,
            allowed_env_vars: vec!["HOME".to_string()],
            allowed_paths: vec!["/tmp".to_string()],
            max_memory_mb: None,
            max_cpu_seconds: None,
            max_output_bytes: Some(2048),
        });

        let report = generate_security_report(&manifest);
        let json = report.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["vault_keys"], serde_json::json!(["SECRET"]));
        assert_eq!(parsed["env_vars"], serde_json::json!(["HOME"]));
        assert_eq!(parsed["filesystem_paths"], serde_json::json!(["/tmp"]));
        assert_eq!(
            parsed["network_endpoints"],
            serde_json::json!(["https://example.com"])
        );
        assert_eq!(parsed["bridge_calls"], serde_json::json!([]));
        assert_eq!(parsed["max_output_bytes"], serde_json::json!(2048));
    }

    #[test]
    fn test_to_markdown_contains_sections() {
        let mut manifest = minimal_manifest();
        manifest.vault_keys = vec!["KEY1".to_string()];
        manifest.sandbox = Some(SandboxConfig {
            enabled: true,
            allowed_env_vars: vec!["VAR1".to_string()],
            allowed_paths: vec!["/opt".to_string()],
            max_memory_mb: None,
            max_cpu_seconds: None,
            max_output_bytes: Some(512),
        });

        let report = generate_security_report(&manifest);
        let md = report.to_markdown();

        assert!(md.contains("# Security Surface Report"));
        assert!(md.contains("## Vault Keys"));
        assert!(md.contains("`KEY1`"));
        assert!(md.contains("## Environment Variables"));
        assert!(md.contains("`VAR1`"));
        assert!(md.contains("## Filesystem Paths"));
        assert!(md.contains("`/opt`"));
        assert!(md.contains("## Network Endpoints"));
        assert!(md.contains("_None declared._"));
        assert!(md.contains("## Bridge Calls"));
        assert!(md.contains("## Max Output Bytes"));
        assert!(md.contains("512"));
    }

    #[test]
    fn test_to_markdown_empty_report() {
        let manifest = minimal_manifest();
        let report = generate_security_report(&manifest);
        let md = report.to_markdown();

        assert!(md.contains("_None declared._"));
        assert!(md.contains("_No limit declared._"));
    }
}
