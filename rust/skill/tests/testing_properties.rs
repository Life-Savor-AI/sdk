//! Property-based tests for the Skill SDK `testing` module.
//!
//! **Property 15: MockSandbox enforces restrictions on env vars and paths**
//!
//! **Validates: Requirements 15.4**

use std::path::PathBuf;

use lifesavor_skill_sdk::testing::{MockSandbox, MockSandboxViolation};
use lifesavor_skill_sdk::SandboxConfig;
use proptest::prelude::*;

/// Strategy for a `SandboxConfig` with arbitrary allowed env vars and paths.
fn arb_sandbox_config() -> impl Strategy<Value = SandboxConfig> {
    (
        prop::collection::vec("[A-Z][A-Z0-9_]{0,15}", 0..=5),
        prop::collection::vec("/[a-z]{1,8}(/[a-z]{1,8}){0,2}", 0..=5),
    )
        .prop_map(|(env_vars, paths)| SandboxConfig {
            enabled: true,
            allowed_env_vars: env_vars,
            allowed_paths: paths,
            max_memory_mb: None,
            max_cpu_seconds: None,
            max_output_bytes: None,
        })
}

proptest! {
    /// **Property 15 (allowed env vars pass): For any SandboxConfig, checking
    /// an env var that IS in the allowlist returns Ok and records no violation.**
    ///
    /// **Validates: Requirements 15.4**
    #[test]
    fn mock_sandbox_allows_declared_env_vars(config in arb_sandbox_config()) {
        prop_assume!(!config.allowed_env_vars.is_empty());

        let mut sandbox = MockSandbox::new(config.clone());
        for var in &config.allowed_env_vars {
            let result = sandbox.check_env_var(var);
            prop_assert!(
                result.is_ok(),
                "check_env_var({:?}) should pass for allowed var",
                var
            );
        }
        prop_assert_eq!(sandbox.violation_count(), 0);
    }

    /// **Property 15 (disallowed env vars flagged): For any SandboxConfig and
    /// any env var NOT in the allowlist, check_env_var returns an error and
    /// records a DisallowedEnvVar violation.**
    ///
    /// **Validates: Requirements 15.4**
    #[test]
    fn mock_sandbox_flags_undeclared_env_vars(
        config in arb_sandbox_config(),
        extra_vars in prop::collection::vec("[A-Z]{1,4}_XTRA_[0-9]{1,3}", 1..=3),
    ) {
        let disallowed: Vec<String> = extra_vars
            .into_iter()
            .filter(|v| !config.allowed_env_vars.contains(v))
            .collect();
        prop_assume!(!disallowed.is_empty());

        let mut sandbox = MockSandbox::new(config);
        for var in &disallowed {
            let result = sandbox.check_env_var(var);
            prop_assert!(result.is_err());
            prop_assert_eq!(
                result.unwrap_err(),
                MockSandboxViolation::DisallowedEnvVar {
                    var_name: var.clone(),
                }
            );
        }
        prop_assert_eq!(sandbox.violation_count(), disallowed.len());
    }

    /// **Property 15 (allowed paths pass): For any SandboxConfig, checking a
    /// path that is under an allowed prefix returns Ok and records no violation.**
    ///
    /// **Validates: Requirements 15.4**
    #[test]
    fn mock_sandbox_allows_paths_under_prefix(
        config in arb_sandbox_config(),
        suffixes in prop::collection::vec("[a-z]{1,6}\\.dat", 1..=3),
    ) {
        prop_assume!(!config.allowed_paths.is_empty());

        let mut sandbox = MockSandbox::new(config.clone());
        for (i, suffix) in suffixes.iter().enumerate() {
            let base = &config.allowed_paths[i % config.allowed_paths.len()];
            let path = format!("{}/{}", base, suffix);
            let result = sandbox.check_path(&path);
            prop_assert!(
                result.is_ok(),
                "check_path({:?}) should pass for path under allowed prefix {:?}",
                path,
                base
            );
        }
        prop_assert_eq!(sandbox.violation_count(), 0);
    }

    /// **Property 15 (disallowed paths flagged): For any SandboxConfig and any
    /// path NOT under any allowed prefix, check_path returns an error and
    /// records a DisallowedPath violation.**
    ///
    /// **Validates: Requirements 15.4**
    #[test]
    fn mock_sandbox_flags_disallowed_paths(
        config in arb_sandbox_config(),
        bad_paths in prop::collection::vec(
            "/forbidden_[a-z]{1,5}/[a-z]{1,6}", 1..=3
        ),
    ) {
        let truly_disallowed: Vec<String> = bad_paths
            .into_iter()
            .filter(|p| {
                !config
                    .allowed_paths
                    .iter()
                    .any(|allowed| PathBuf::from(p).starts_with(allowed))
            })
            .collect();
        prop_assume!(!truly_disallowed.is_empty());

        let mut sandbox = MockSandbox::new(config);
        for path_str in &truly_disallowed {
            let result = sandbox.check_path(path_str.as_str());
            prop_assert!(result.is_err());
            prop_assert_eq!(
                result.unwrap_err(),
                MockSandboxViolation::DisallowedPath {
                    path: PathBuf::from(path_str),
                }
            );
        }
        prop_assert_eq!(sandbox.violation_count(), truly_disallowed.len());
    }

    /// **Property 15 (violations accumulate and clear): For any sequence of
    /// disallowed accesses, violations accumulate; after clear_violations,
    /// the count resets to zero.**
    ///
    /// **Validates: Requirements 15.4**
    #[test]
    fn mock_sandbox_violations_accumulate_and_clear(
        extra_vars in prop::collection::vec("[A-Z]{2,5}_NOPE", 1..=5),
    ) {
        let config = SandboxConfig {
            enabled: true,
            allowed_env_vars: vec![],
            allowed_paths: vec![],
            max_memory_mb: None,
            max_cpu_seconds: None,
            max_output_bytes: None,
        };
        let mut sandbox = MockSandbox::new(config);

        for var in &extra_vars {
            let _ = sandbox.check_env_var(var);
        }
        prop_assert_eq!(sandbox.violation_count(), extra_vars.len());
        prop_assert_eq!(sandbox.violations().len(), extra_vars.len());

        sandbox.clear_violations();
        prop_assert_eq!(sandbox.violation_count(), 0);
        prop_assert!(sandbox.violations().is_empty());
    }

    /// **Property 15 (config accessor): The config returned by MockSandbox::config()
    /// matches the config it was constructed with.**
    ///
    /// **Validates: Requirements 15.4**
    #[test]
    fn mock_sandbox_config_accessor_matches(config in arb_sandbox_config()) {
        let sandbox = MockSandbox::new(config.clone());
        prop_assert_eq!(&sandbox.config().allowed_env_vars, &config.allowed_env_vars);
        prop_assert_eq!(&sandbox.config().allowed_paths, &config.allowed_paths);
        prop_assert_eq!(sandbox.config().enabled, config.enabled);
    }
}
