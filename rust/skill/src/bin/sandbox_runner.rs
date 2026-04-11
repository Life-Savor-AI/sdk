//! SandboxRunner — local sandbox testing binary for skill providers.
//!
//! Spawns a skill as a child process with the same `ProcessSandbox`
//! restrictions the agent would apply, enabling local sandbox compliance
//! testing without a running agent.
//!
//! # Usage
//!
//! ```sh
//! cargo run --bin sandbox-runner -- --manifest my-skill.toml
//! cargo run --bin sandbox-runner -- --manifest my-skill.toml --payload '{"input":"hello"}'
//! cargo run --bin sandbox-runner -- --manifest my-skill.toml --mcp
//! ```

use std::io::Write;
use std::path::PathBuf;
use std::process::{self, Stdio};

use lifesavor_skill_sdk::sandbox_compliance::{ComplianceViolation, SandboxComplianceChecker};
use lifesavor_skill_sdk::{ProviderManifest, ProviderType, SandboxConfig};

// ProcessSandbox requires the agent-runtime feature.
#[cfg(feature = "agent-runtime")]
use lifesavor_skill_sdk::ProcessSandbox;

// ── CLI argument parsing (manual, no clap dependency) ────────────────────

struct Args {
    manifest_path: PathBuf,
    mcp: bool,
    payload: String,
}

fn print_usage() {
    eprintln!("Usage: sandbox-runner --manifest <path> [--mcp] [--payload <json>]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --manifest <path>   Path to the ProviderManifest TOML file (required)");
    eprintln!("  --mcp               Run MCP capability handshake + test invocation");
    eprintln!("  --payload <json>    JSON payload to send via stdin (default: {{}})");
}

fn parse_args() -> Result<Args, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut manifest_path: Option<PathBuf> = None;
    let mut mcp = false;
    let mut payload = "{}".to_string();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--manifest" => {
                i += 1;
                if i >= args.len() {
                    return Err("--manifest requires a value".to_string());
                }
                manifest_path = Some(PathBuf::from(&args[i]));
            }
            "--mcp" => {
                mcp = true;
            }
            "--payload" => {
                i += 1;
                if i >= args.len() {
                    return Err("--payload requires a value".to_string());
                }
                payload = args[i].clone();
            }
            "--help" | "-h" => {
                print_usage();
                process::exit(0);
            }
            other => {
                return Err(format!("Unknown argument: {other}"));
            }
        }
        i += 1;
    }

    let manifest_path = manifest_path.ok_or_else(|| "--manifest <path> is required".to_string())?;

    Ok(Args {
        manifest_path,
        mcp,
        payload,
    })
}

// ── Structured violation report ──────────────────────────────────────────

/// JSON-serialisable violation report emitted by the runner.
#[derive(serde::Serialize)]
struct ViolationReport {
    violation_type: String,
    detail: String,
}

impl From<&ComplianceViolation> for ViolationReport {
    fn from(v: &ComplianceViolation) -> Self {
        match v {
            ComplianceViolation::UndeclaredEnvVar { var_name } => ViolationReport {
                violation_type: "UndeclaredEnvVar".to_string(),
                detail: format!("Environment variable '{var_name}' accessed but not declared in sandbox allowlist"),
            },
            ComplianceViolation::DisallowedPath { path } => ViolationReport {
                violation_type: "DisallowedPath".to_string(),
                detail: format!("Filesystem path '{}' accessed but not under any allowed path", path.display()),
            },
            ComplianceViolation::OutputSizeExceeded { actual, limit } => ViolationReport {
                violation_type: "OutputSizeExceeded".to_string(),
                detail: format!("Output size {actual} bytes exceeds limit of {limit} bytes"),
            },
        }
    }
}

/// Top-level structured output from the runner.
#[derive(serde::Serialize)]
struct RunnerOutput {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    response: Option<String>,
    violations: Vec<ViolationReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

// ── Main ─────────────────────────────────────────────────────────────────

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error: {e}");
            print_usage();
            process::exit(1);
        }
    };

    // 1. Read and parse the manifest TOML.
    let manifest_str = match std::fs::read_to_string(&args.manifest_path) {
        Ok(s) => s,
        Err(e) => {
            emit_error(&format!(
                "Failed to read manifest '{}': {e}",
                args.manifest_path.display()
            ));
            process::exit(1);
        }
    };

    let manifest: ProviderManifest = match toml::from_str(&manifest_str) {
        Ok(m) => m,
        Err(e) => {
            emit_error(&format!("Failed to parse manifest TOML: {e}"));
            process::exit(1);
        }
    };

    // Validate provider type is Skill.
    if manifest.provider_type != ProviderType::Skill {
        emit_error(&format!(
            "Manifest provider_type is '{}', expected 'skill'",
            manifest.provider_type
        ));
        process::exit(1);
    }

    // 2. Extract sandbox config (use defaults if absent).
    let sandbox_config = manifest.sandbox.clone().unwrap_or(SandboxConfig {
        enabled: true,
        allowed_env_vars: vec![],
        allowed_paths: vec![],
        max_memory_mb: None,
        max_cpu_seconds: None,
        max_output_bytes: None,
    });

    // 3. Determine the skill command from the manifest.
    let command = match &manifest.connection.command {
        Some(cmd) => cmd.clone(),
        None => {
            emit_error("Manifest connection.command is required for skill providers");
            process::exit(1);
        }
    };

    let cmd_args: Vec<String> = manifest.connection.args.clone().unwrap_or_default();

    // 4. Build ProcessSandbox from config.
    let sandbox = ProcessSandbox::from_config(&sandbox_config, &manifest.instance_name);

    // 5. Build the payload based on mode.
    let stdin_payload = if args.mcp {
        build_mcp_payload(&args.payload)
    } else {
        format!("{}\n", args.payload)
    };

    // 6. Spawn the skill as a child process with sandbox restrictions.
    let mut cmd = std::process::Command::new(&command);
    for arg in &cmd_args {
        cmd.arg(arg);
    }

    // Apply sandbox environment restrictions.
    if sandbox.enabled {
        cmd.env_clear();
        let env = sandbox.build_env();
        for (key, val) in &env {
            cmd.env(key, val);
        }
        if let Some(work_dir) = sandbox.allowed_paths.first() {
            cmd.current_dir(work_dir);
        }
    }

    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            emit_error(&format!("Failed to spawn skill process '{command}': {e}"));
            process::exit(1);
        }
    };

    // 7. Send payload via stdin.
    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(stdin_payload.as_bytes()) {
            emit_error(&format!("Failed to write to skill stdin: {e}"));
            process::exit(1);
        }
        // Drop stdin to signal EOF.
    }

    // 8. Wait for the child and capture output.
    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            emit_error(&format!("Failed to wait for skill process: {e}"));
            process::exit(1);
        }
    };

    let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();

    // 9. Run compliance checks.
    let checker = SandboxComplianceChecker::new(sandbox_config);
    let mut violations = Vec::new();

    // Check output size.
    if let Some(v) = checker.check_output_size(output.stdout.len()) {
        violations.push(v);
    }

    // Convert violations to structured reports.
    let violation_reports: Vec<ViolationReport> =
        violations.iter().map(ViolationReport::from).collect();

    // 10. Emit structured result.
    let success = output.status.success() && violation_reports.is_empty();

    let result = RunnerOutput {
        success,
        response: if stdout_str.is_empty() {
            None
        } else {
            Some(stdout_str.clone())
        },
        violations: violation_reports,
        error: if !stderr_str.is_empty() {
            Some(stderr_str)
        } else if !output.status.success() {
            Some(format!("Process exited with status: {}", output.status))
        } else {
            None
        },
    };

    let json = serde_json::to_string_pretty(&result).unwrap_or_else(|e| {
        format!("{{\"success\":false,\"error\":\"Failed to serialize output: {e}\"}}")
    });

    println!("{json}");

    if !success {
        process::exit(1);
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Build MCP handshake + test invocation payload.
///
/// The MCP protocol starts with an `initialize` request, followed by a
/// `tools/list` request, and optionally a tool invocation.
fn build_mcp_payload(user_payload: &str) -> String {
    let init_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "sandbox-runner",
                "version": "0.1.0"
            }
        }
    });

    let list_tools_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let invoke_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": serde_json::from_str::<serde_json::Value>(user_payload).unwrap_or(serde_json::json!({}))
    });

    format!(
        "{}\n{}\n{}\n",
        serde_json::to_string(&init_request).unwrap(),
        serde_json::to_string(&list_tools_request).unwrap(),
        serde_json::to_string(&invoke_request).unwrap(),
    )
}

/// Emit a structured error to stdout and exit.
fn emit_error(message: &str) {
    let result = RunnerOutput {
        success: false,
        response: None,
        violations: vec![],
        error: Some(message.to_string()),
    };
    let json = serde_json::to_string_pretty(&result).unwrap_or_else(|e| {
        format!("{{\"success\":false,\"error\":\"{e}\"}}")
    });
    eprintln!("{json}");
}
