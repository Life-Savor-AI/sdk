//! Property-based tests for SandboxRunner compliance logic.
//!
//! **Property 22: SandboxRunner applies manifest restrictions and reports
//! specific violation types**
//!
//! **Validates: Requirements 34.2, 34.3, 34.4**
//!
//! Since the SandboxRunner is a binary that spawns child processes, direct
//! property testing of the binary is complex. Instead we test the underlying
//! compliance checking logic (`SandboxComplianceChecker`) with various
//! manifest configurations and verify that the `ViolationReport` JSON
//! serialization produces the correct `violation_type` strings.

use std::path::PathBuf;

use lifesavor_skill_sdk::sandbox_compliance::{ComplianceViolation, SandboxComplianceChecker};
use lifesavor_skill_sdk::SandboxConfig;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// ViolationReport — mirrors the struct in sandbox_runner.rs for testing
// serialization of ComplianceViolation → JSON.
// ---------------------------------------------------------------------------

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct ViolationReport {
    violation_type: String,
    detail: String,
}

impl From<&ComplianceViolation> for ViolationReport {
    fn from(v: &ComplianceViolation) -> Self {
        match v {
            ComplianceViolation::UndeclaredEnvVar { var_name } => ViolationReport {
                violation_type: "UndeclaredEnvVar".to_string(),
                detail: format!(
                    "Environment variable '{var_name}' accessed but not declared in sandbox allowlist"
                ),
            },
            ComplianceViolation::DisallowedPath { path } => ViolationReport {
                violation_type: "DisallowedPath".to_string(),
                detail: format!(
                    "Filesystem path '{}' accessed but not under any allowed path",
                    path.display()
                ),
            },
            ComplianceViolation::OutputSizeExceeded { actual, limit } => ViolationReport {
                violation_type: "OutputSizeExceeded".to_string(),
                detail: format!("Output size {actual} bytes exceeds limit of {limit} bytes"),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Property 22 tests
// ---------------------------------------------------------------------------

proptest! {
    /// **Property 22 (output within limit): For any SandboxConfig with a
    /// max_output_bytes limit and any output size <= that limit, the checker
    /// reports no OutputSizeExceeded violation.**
    ///
    /// **Validates: Requirements 34.2, 34.3**
    #[test]
    fn no_violation_when_output_within_limit(
        limit in 1u64..=10_000_000u64,
        // Generate a fraction [0.0, 1.0] to pick a size at or below the limit
        fraction in 0.0f64..=1.0f64,
    ) {
        let size = (limit as f64 * fraction) as usize;
        let config = SandboxConfig {
            enabled: true,
            allowed_env_vars: vec![],
            allowed_paths: vec![],
            max_memory_mb: None,
            max_cpu_seconds: None,
            max_output_bytes: Some(limit),
        };
        let checker = SandboxComplianceChecker::new(config);
        let violation = checker.check_output_size(size);
        prop_assert!(
            violation.is_none(),
            "Expected no violation for size={size} with limit={limit}, got: {:?}",
            violation
        );
    }

    /// **Property 22 (output exceeds limit): For any SandboxConfig with a
    /// max_output_bytes limit and any output size > that limit, the checker
    /// reports an OutputSizeExceeded violation with correct actual/limit values.**
    ///
    /// **Validates: Requirements 34.2, 34.4**
    #[test]
    fn violation_when_output_exceeds_limit(
        limit in 1u64..=10_000_000u64,
        excess in 1usize..=10_000usize,
    ) {
        let size = limit as usize + excess;
        let config = SandboxConfig {
            enabled: true,
            allowed_env_vars: vec![],
            allowed_paths: vec![],
            max_memory_mb: None,
            max_cpu_seconds: None,
            max_output_bytes: Some(limit),
        };
        let checker = SandboxComplianceChecker::new(config);
        let violation = checker.check_output_size(size);
        prop_assert!(violation.is_some(), "Expected OutputSizeExceeded violation");
        match violation.unwrap() {
            ComplianceViolation::OutputSizeExceeded { actual, limit: l } => {
                prop_assert_eq!(actual, size);
                prop_assert_eq!(l, limit);
            }
            other => {
                prop_assert!(false, "Expected OutputSizeExceeded, got: {:?}", other);
            }
        }
    }

    /// **Property 22 (no limit configured): When max_output_bytes is None,
    /// the checker never reports an OutputSizeExceeded violation regardless
    /// of output size.**
    ///
    /// **Validates: Requirements 34.2**
    #[test]
    fn no_violation_when_no_limit_configured(
        size in 0usize..=100_000_000usize,
    ) {
        let config = SandboxConfig {
            enabled: true,
            allowed_env_vars: vec![],
            allowed_paths: vec![],
            max_memory_mb: None,
            max_cpu_seconds: None,
            max_output_bytes: None,
        };
        let checker = SandboxComplianceChecker::new(config);
        let violation = checker.check_output_size(size);
        prop_assert!(
            violation.is_none(),
            "Expected no violation when no limit is configured, got: {:?}",
            violation
        );
    }

    /// **Property 22 (ViolationReport serialization — OutputSizeExceeded):
    /// For any OutputSizeExceeded violation, the ViolationReport JSON
    /// contains violation_type "OutputSizeExceeded" and a non-empty detail.**
    ///
    /// **Validates: Requirements 34.4**
    #[test]
    fn violation_report_output_size_exceeded_serializes_correctly(
        actual in 1usize..=100_000_000usize,
        limit in 1u64..=100_000_000u64,
    ) {
        let violation = ComplianceViolation::OutputSizeExceeded { actual, limit };
        let report = ViolationReport::from(&violation);
        let json = serde_json::to_string(&report).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(
            parsed["violation_type"].as_str().unwrap(),
            "OutputSizeExceeded"
        );
        prop_assert!(
            !parsed["detail"].as_str().unwrap().is_empty(),
            "detail should be non-empty"
        );
    }

    /// **Property 22 (ViolationReport serialization — UndeclaredEnvVar):
    /// For any UndeclaredEnvVar violation, the ViolationReport JSON
    /// contains violation_type "UndeclaredEnvVar" and a non-empty detail.**
    ///
    /// **Validates: Requirements 34.4**
    #[test]
    fn violation_report_undeclared_env_var_serializes_correctly(
        var_name in "[A-Z][A-Z0-9_]{0,20}",
    ) {
        let violation = ComplianceViolation::UndeclaredEnvVar { var_name: var_name.clone() };
        let report = ViolationReport::from(&violation);
        let json = serde_json::to_string(&report).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(
            parsed["violation_type"].as_str().unwrap(),
            "UndeclaredEnvVar"
        );
        prop_assert!(
            !parsed["detail"].as_str().unwrap().is_empty(),
            "detail should be non-empty"
        );
    }

    /// **Property 22 (ViolationReport serialization — DisallowedPath):
    /// For any DisallowedPath violation, the ViolationReport JSON
    /// contains violation_type "DisallowedPath" and a non-empty detail.**
    ///
    /// **Validates: Requirements 34.4**
    #[test]
    fn violation_report_disallowed_path_serializes_correctly(
        path in "/[a-z]{1,10}(/[a-z]{1,10}){0,3}",
    ) {
        let violation = ComplianceViolation::DisallowedPath { path: PathBuf::from(&path) };
        let report = ViolationReport::from(&violation);
        let json = serde_json::to_string(&report).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(
            parsed["violation_type"].as_str().unwrap(),
            "DisallowedPath"
        );
        prop_assert!(
            !parsed["detail"].as_str().unwrap().is_empty(),
            "detail should be non-empty"
        );
    }
}
