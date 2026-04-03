//! Test harness for the Skill SDK.
//!
//! Provides [`MockSandbox`] for testing skill implementations against sandbox
//! restrictions without a running agent. The mock accepts a [`SandboxConfig`]
//! and checks environment variable access and filesystem path access against
//! the configured allowlists.
//!
//! Also provides assertion helpers for verifying error chains, health check
//! responses, and capability descriptors.

use std::path::PathBuf;

use crate::{ErrorContext, HealthStatus, SandboxConfig, Subsystem};

// ---------------------------------------------------------------------------
// SandboxViolation (mock-specific)
// ---------------------------------------------------------------------------

/// A violation detected by the [`MockSandbox`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MockSandboxViolation {
    /// An environment variable was accessed that is not in the allowlist.
    DisallowedEnvVar {
        /// The name of the disallowed variable.
        var_name: String,
    },
    /// A filesystem path was accessed that is not under any allowed path.
    DisallowedPath {
        /// The path that was accessed.
        path: PathBuf,
    },
}

// ---------------------------------------------------------------------------
// MockSandbox
// ---------------------------------------------------------------------------

/// Simulates the agent's process sandbox for testing skill implementations.
///
/// Accepts a [`SandboxConfig`] and provides methods to check whether
/// environment variable and filesystem accesses would be allowed.
///
/// # Example
///
/// ```rust,ignore
/// use lifesavor_skill_sdk::testing::MockSandbox;
/// use lifesavor_skill_sdk::SandboxConfig;
///
/// let config = SandboxConfig {
///     enabled: true,
///     allowed_env_vars: vec!["HOME".into(), "PATH".into()],
///     allowed_paths: vec!["/tmp".into()],
///     max_memory_mb: None,
///     max_cpu_seconds: None,
///     max_output_bytes: None,
/// };
///
/// let sandbox = MockSandbox::new(config);
/// assert!(sandbox.check_env_var("HOME").is_ok());
/// assert!(sandbox.check_env_var("SECRET").is_err());
/// assert!(sandbox.check_path("/tmp/data.txt").is_ok());
/// assert!(sandbox.check_path("/etc/passwd").is_err());
/// ```
pub struct MockSandbox {
    config: SandboxConfig,
    violations: Vec<MockSandboxViolation>,
}

impl MockSandbox {
    /// Create a new mock sandbox from the given configuration.
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config,
            violations: Vec::new(),
        }
    }

    /// Check whether accessing the given environment variable is allowed.
    ///
    /// Returns `Ok(())` if the variable is in the allowlist, or
    /// `Err(MockSandboxViolation::DisallowedEnvVar)` if not. The violation
    /// is also recorded internally.
    pub fn check_env_var(&mut self, var_name: &str) -> Result<(), MockSandboxViolation> {
        if self.config.allowed_env_vars.contains(&var_name.to_string()) {
            Ok(())
        } else {
            let violation = MockSandboxViolation::DisallowedEnvVar {
                var_name: var_name.to_string(),
            };
            self.violations.push(violation.clone());
            Err(violation)
        }
    }

    /// Check whether accessing the given filesystem path is allowed.
    ///
    /// Returns `Ok(())` if the path falls under one of the allowed path
    /// prefixes, or `Err(MockSandboxViolation::DisallowedPath)` if not.
    /// The violation is also recorded internally.
    pub fn check_path<P: Into<PathBuf>>(&mut self, path: P) -> Result<(), MockSandboxViolation> {
        let path = path.into();
        let allowed = self
            .config
            .allowed_paths
            .iter()
            .any(|allowed| path.starts_with(allowed));

        if allowed {
            Ok(())
        } else {
            let violation = MockSandboxViolation::DisallowedPath { path };
            self.violations.push(violation.clone());
            Err(violation)
        }
    }

    /// Return all violations recorded so far.
    pub fn violations(&self) -> &[MockSandboxViolation] {
        &self.violations
    }

    /// Return the number of violations recorded.
    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }

    /// Clear all recorded violations.
    pub fn clear_violations(&mut self) {
        self.violations.clear();
    }

    /// Return a reference to the underlying sandbox config.
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

/// Assert that an [`ErrorContext`] has the expected subsystem.
pub fn assert_error_context_subsystem(ctx: &ErrorContext, expected: Subsystem) {
    assert_eq!(
        ctx.subsystem, expected,
        "Expected subsystem {:?}, got {:?}",
        expected, ctx.subsystem,
    );
}

/// Assert that a [`HealthStatus`] is [`HealthStatus::Healthy`].
pub fn assert_healthy(status: &HealthStatus) {
    assert_eq!(
        *status,
        HealthStatus::Healthy,
        "Expected Healthy, got {status:?}",
    );
}

/// Assert that a [`HealthStatus`] is NOT [`HealthStatus::Healthy`].
pub fn assert_unhealthy(status: &HealthStatus) {
    assert_ne!(
        *status,
        HealthStatus::Healthy,
        "Expected unhealthy status, got Healthy",
    );
}

/// Assert that an [`ErrorContext`] has a non-empty code and message.
pub fn assert_error_context_non_empty(ctx: &ErrorContext) {
    assert!(!ctx.code.is_empty(), "ErrorContext code must not be empty");
    assert!(
        !ctx.message.is_empty(),
        "ErrorContext message must not be empty"
    );
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> SandboxConfig {
        SandboxConfig {
            enabled: true,
            allowed_env_vars: vec!["HOME".into(), "PATH".into()],
            allowed_paths: vec!["/tmp".into(), "/home/user".into()],
            max_memory_mb: None,
            max_cpu_seconds: None,
            max_output_bytes: None,
        }
    }

    #[test]
    fn new_sandbox_has_no_violations() {
        let sandbox = MockSandbox::new(test_config());
        assert_eq!(sandbox.violation_count(), 0);
        assert!(sandbox.violations().is_empty());
    }

    #[test]
    fn allowed_env_var_passes() {
        let mut sandbox = MockSandbox::new(test_config());
        assert!(sandbox.check_env_var("HOME").is_ok());
        assert!(sandbox.check_env_var("PATH").is_ok());
        assert_eq!(sandbox.violation_count(), 0);
    }

    #[test]
    fn disallowed_env_var_fails() {
        let mut sandbox = MockSandbox::new(test_config());
        let result = sandbox.check_env_var("SECRET_KEY");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            MockSandboxViolation::DisallowedEnvVar {
                var_name: "SECRET_KEY".into()
            }
        );
        assert_eq!(sandbox.violation_count(), 1);
    }

    #[test]
    fn allowed_path_passes() {
        let mut sandbox = MockSandbox::new(test_config());
        assert!(sandbox.check_path("/tmp/data.txt").is_ok());
        assert!(sandbox.check_path("/home/user/file.rs").is_ok());
        assert_eq!(sandbox.violation_count(), 0);
    }

    #[test]
    fn disallowed_path_fails() {
        let mut sandbox = MockSandbox::new(test_config());
        let result = sandbox.check_path("/etc/passwd");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            MockSandboxViolation::DisallowedPath {
                path: PathBuf::from("/etc/passwd")
            }
        );
        assert_eq!(sandbox.violation_count(), 1);
    }

    #[test]
    fn multiple_violations_accumulate() {
        let mut sandbox = MockSandbox::new(test_config());
        let _ = sandbox.check_env_var("SECRET");
        let _ = sandbox.check_path("/etc/shadow");
        assert_eq!(sandbox.violation_count(), 2);
    }

    #[test]
    fn clear_violations() {
        let mut sandbox = MockSandbox::new(test_config());
        let _ = sandbox.check_env_var("SECRET");
        assert_eq!(sandbox.violation_count(), 1);
        sandbox.clear_violations();
        assert_eq!(sandbox.violation_count(), 0);
    }

    #[test]
    fn config_accessor() {
        let config = test_config();
        let sandbox = MockSandbox::new(config.clone());
        assert_eq!(sandbox.config().allowed_env_vars, config.allowed_env_vars);
    }

    #[test]
    fn assert_healthy_passes() {
        assert_healthy(&HealthStatus::Healthy);
    }

    #[test]
    #[should_panic(expected = "Expected Healthy")]
    fn assert_healthy_panics_for_unhealthy() {
        assert_healthy(&HealthStatus::Unhealthy {
            details: "bad".into(),
        });
    }

    #[test]
    fn assert_unhealthy_passes() {
        assert_unhealthy(&HealthStatus::Unhealthy {
            details: "bad".into(),
        });
    }

    #[test]
    fn assert_error_context_subsystem_passes() {
        let ctx = ErrorContext::new(Subsystem::Provider, "TEST", "msg".to_string());
        assert_error_context_subsystem(&ctx, Subsystem::Provider);
    }
}
