//! Sandbox compliance checking for skill providers.
//!
//! The [`SandboxComplianceChecker`] validates that a skill implementation
//! respects the constraints declared in its [`SandboxConfig`]: environment
//! variable allowlists, filesystem path restrictions, and output size limits.

use std::path::PathBuf;

use crate::SandboxConfig;

/// A violation detected by the [`SandboxComplianceChecker`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComplianceViolation {
    /// An environment variable was accessed that is not in the sandbox
    /// allowlist.
    UndeclaredEnvVar {
        /// The name of the undeclared variable.
        var_name: String,
    },
    /// A filesystem path was accessed that is not under any allowed path
    /// prefix.
    DisallowedPath {
        /// The path that was accessed.
        path: PathBuf,
    },
    /// The output size exceeded the configured maximum.
    OutputSizeExceeded {
        /// Actual output size in bytes.
        actual: usize,
        /// Maximum allowed output size in bytes.
        limit: u64,
    },
}

/// Validates skill behaviour against a [`SandboxConfig`].
///
/// Construct a checker from the sandbox section of a provider manifest and
/// then call the individual `check_*` methods to detect violations.
pub struct SandboxComplianceChecker {
    config: SandboxConfig,
}

impl SandboxComplianceChecker {
    /// Create a new checker from the given sandbox configuration.
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }

    /// Check that every accessed environment variable is declared in the
    /// sandbox allowlist.
    ///
    /// Returns a [`ComplianceViolation::UndeclaredEnvVar`] for each variable
    /// in `accessed` that does not appear in `allowed_env_vars`.
    pub fn check_env_vars(&self, accessed: &[String]) -> Vec<ComplianceViolation> {
        accessed
            .iter()
            .filter(|var| !self.config.allowed_env_vars.contains(var))
            .map(|var| ComplianceViolation::UndeclaredEnvVar {
                var_name: var.clone(),
            })
            .collect()
    }

    /// Check that every accessed filesystem path falls under one of the
    /// allowed path prefixes.
    ///
    /// Returns a [`ComplianceViolation::DisallowedPath`] for each path that
    /// is not a descendant of any entry in `allowed_paths`.
    pub fn check_filesystem(&self, paths: &[PathBuf]) -> Vec<ComplianceViolation> {
        paths
            .iter()
            .filter(|path| {
                !self
                    .config
                    .allowed_paths
                    .iter()
                    .any(|allowed| path.starts_with(allowed))
            })
            .map(|path| ComplianceViolation::DisallowedPath { path: path.clone() })
            .collect()
    }

    /// Check whether the output size exceeds the configured maximum.
    ///
    /// Returns `Some(ComplianceViolation::OutputSizeExceeded)` when
    /// `max_output_bytes` is set and `bytes` exceeds it, or `None` if the
    /// output is within limits (or no limit is configured).
    pub fn check_output_size(&self, bytes: usize) -> Option<ComplianceViolation> {
        self.config.max_output_bytes.and_then(|limit| {
            if bytes as u64 > limit {
                Some(ComplianceViolation::OutputSizeExceeded {
                    actual: bytes,
                    limit,
                })
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sandbox_config(
        env_vars: Vec<&str>,
        paths: Vec<&str>,
        max_output: Option<u64>,
    ) -> SandboxConfig {
        SandboxConfig {
            enabled: true,
            allowed_env_vars: env_vars.into_iter().map(String::from).collect(),
            allowed_paths: paths.into_iter().map(String::from).collect(),
            max_memory_mb: None,
            max_cpu_seconds: None,
            max_output_bytes: max_output,
        }
    }

    #[test]
    fn env_vars_all_allowed() {
        let checker = SandboxComplianceChecker::new(sandbox_config(
            vec!["HOME", "PATH"],
            vec![],
            None,
        ));
        let violations = checker.check_env_vars(&["HOME".into(), "PATH".into()]);
        assert!(violations.is_empty());
    }

    #[test]
    fn env_vars_undeclared() {
        let checker = SandboxComplianceChecker::new(sandbox_config(
            vec!["HOME"],
            vec![],
            None,
        ));
        let violations = checker.check_env_vars(&["HOME".into(), "SECRET".into()]);
        assert_eq!(violations.len(), 1);
        assert_eq!(
            violations[0],
            ComplianceViolation::UndeclaredEnvVar {
                var_name: "SECRET".into()
            }
        );
    }

    #[test]
    fn filesystem_all_allowed() {
        let checker = SandboxComplianceChecker::new(sandbox_config(
            vec![],
            vec!["/tmp", "/home/user"],
            None,
        ));
        let paths = vec![
            PathBuf::from("/tmp/data.txt"),
            PathBuf::from("/home/user/file.rs"),
        ];
        let violations = checker.check_filesystem(&paths);
        assert!(violations.is_empty());
    }

    #[test]
    fn filesystem_disallowed() {
        let checker = SandboxComplianceChecker::new(sandbox_config(
            vec![],
            vec!["/tmp"],
            None,
        ));
        let paths = vec![PathBuf::from("/etc/passwd")];
        let violations = checker.check_filesystem(&paths);
        assert_eq!(violations.len(), 1);
        assert_eq!(
            violations[0],
            ComplianceViolation::DisallowedPath {
                path: PathBuf::from("/etc/passwd")
            }
        );
    }

    #[test]
    fn output_size_within_limit() {
        let checker = SandboxComplianceChecker::new(sandbox_config(
            vec![],
            vec![],
            Some(1024),
        ));
        assert!(checker.check_output_size(512).is_none());
        assert!(checker.check_output_size(1024).is_none());
    }

    #[test]
    fn output_size_exceeded() {
        let checker = SandboxComplianceChecker::new(sandbox_config(
            vec![],
            vec![],
            Some(1024),
        ));
        let violation = checker.check_output_size(2048);
        assert_eq!(
            violation,
            Some(ComplianceViolation::OutputSizeExceeded {
                actual: 2048,
                limit: 1024,
            })
        );
    }

    #[test]
    fn output_size_no_limit() {
        let checker = SandboxComplianceChecker::new(sandbox_config(
            vec![],
            vec![],
            None,
        ));
        assert!(checker.check_output_size(999_999_999).is_none());
    }
}
