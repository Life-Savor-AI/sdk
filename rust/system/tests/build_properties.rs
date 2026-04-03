//! Property-based tests for `BuildConfigBuilder` and `ComponentManifestBuilder`.
//!
//! **Property 18: BuildConfigBuilder produces valid YAML that round-trips**
//! **Validates: Requirements 28.1, 28.5**
//!
//! **Property 19: ComponentManifestBuilder validates type/version and produces valid TOML**
//! **Validates: Requirements 28.2, 28.4**

use lifesavor_system_sdk::build_config::BuildConfigBuilder;
use lifesavor_system_sdk::component_manifest::ComponentManifestBuilder;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Strategy for environment variable key-value pairs.
fn arb_env_vars() -> impl Strategy<Value = Vec<(String, String)>> {
    prop::collection::vec(
        (
            "[A-Z][A-Z0-9_]{0,15}",
            "[a-zA-Z0-9_/.-]{1,30}",
        ),
        0..=5,
    )
}

/// Strategy for additional build step commands.
fn arb_build_steps() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec("cargo [a-z]{3,10}( --[a-z-]{2,10}){0,2}", 0..=3)
}

/// Strategy for additional test step commands.
fn arb_test_steps() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec("cargo [a-z]{3,10}( --[a-z-]{2,10}){0,2}", 0..=3)
}

/// Strategy for output directory paths.
fn arb_output_dir() -> impl Strategy<Value = String> {
    "[a-z]{1,8}(/[a-z]{1,8}){0,3}"
}

/// Strategy for valid semver version strings.
fn arb_valid_semver() -> impl Strategy<Value = String> {
    (0u32..100, 0u32..100, 0u32..100, prop::option::of("[a-z]{1,8}"))
        .prop_map(|(major, minor, patch, pre)| {
            match pre {
                Some(p) => format!("{}.{}.{}-{}", major, minor, patch, p),
                None => format!("{}.{}.{}", major, minor, patch),
            }
        })
}

/// Strategy for component names (alphanumeric with hyphens).
fn arb_component_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{0,20}"
}

/// Strategy for invalid semver version strings.
fn arb_invalid_semver() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("".to_string()),
        Just("1.0".to_string()),
        Just("abc".to_string()),
        Just("1".to_string()),
        Just("1.0.0.0".to_string()),
        Just("1.0.0-".to_string()),
        "[a-z]{1,5}\\.[a-z]{1,5}\\.[a-z]{1,5}".prop_map(|s| s),
    ]
}

// ---------------------------------------------------------------------------
// Property 18: BuildConfigBuilder produces valid YAML that round-trips
// ---------------------------------------------------------------------------

proptest! {
    /// **Property 18 (YAML content): BuildConfigBuilder produces YAML containing
    /// all specified parameters (env vars, commands, output dir).**
    ///
    /// **Validates: Requirements 28.1, 28.5**
    #[test]
    fn build_config_yaml_contains_all_params(
        env_vars in arb_env_vars(),
        extra_build_steps in arb_build_steps(),
        extra_test_steps in arb_test_steps(),
        output_dir in arb_output_dir(),
    ) {
        let mut builder = BuildConfigBuilder::new_for_system();

        for (key, value) in &env_vars {
            builder = builder.env_var(key, value);
        }
        for step in &extra_build_steps {
            builder = builder.build_step(step);
        }
        for step in &extra_test_steps {
            builder = builder.test_step(step);
        }
        builder = builder.output_dir(&output_dir);

        let yaml = builder.to_yaml();

        // Must contain runtime
        prop_assert!(yaml.contains("runtime: rust"),
            "YAML missing runtime: {}", yaml);

        // Must contain default build command
        prop_assert!(yaml.contains("cargo build --release"),
            "YAML missing default build command: {}", yaml);

        // Must contain default test command
        prop_assert!(yaml.contains("cargo test"),
            "YAML missing default test command: {}", yaml);

        // Must contain all extra build steps
        for step in &extra_build_steps {
            prop_assert!(yaml.contains(step.as_str()),
                "YAML missing build step '{}': {}", step, yaml);
        }

        // Must contain all extra test steps
        for step in &extra_test_steps {
            prop_assert!(yaml.contains(step.as_str()),
                "YAML missing test step '{}': {}", step, yaml);
        }

        // Must contain all env vars (last-write-wins for duplicate keys,
        // matching HashMap::insert semantics in BuildConfigBuilder).
        let mut deduped: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        for (key, value) in &env_vars {
            deduped.insert(key.as_str(), value.as_str());
        }
        for (key, value) in &deduped {
            prop_assert!(yaml.contains(key),
                "YAML missing env var key '{}': {}", key, yaml);
            prop_assert!(yaml.contains(value),
                "YAML missing env var value '{}': {}", value, yaml);
        }

        // Must contain output dir
        prop_assert!(yaml.contains(&output_dir),
            "YAML missing output_dir '{}': {}", output_dir, yaml);
    }

    /// **Property 18 (file round-trip): Writing via `to_file` and reading back
    /// produces equivalent YAML content.**
    ///
    /// **Validates: Requirements 28.5**
    #[test]
    fn build_config_file_round_trip(
        env_vars in arb_env_vars(),
        extra_build_steps in arb_build_steps(),
        output_dir in arb_output_dir(),
    ) {
        let mut builder = BuildConfigBuilder::new_for_system();

        for (key, value) in &env_vars {
            builder = builder.env_var(key, value);
        }
        for step in &extra_build_steps {
            builder = builder.build_step(step);
        }
        builder = builder.output_dir(&output_dir);

        let yaml_in_memory = builder.to_yaml();

        // Write to temp file and read back
        let dir = std::env::temp_dir().join(format!(
            "build_config_prop18_{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("lifesavor-build.yml");

        builder.to_file(&path).expect("to_file should succeed");
        let yaml_from_file = std::fs::read_to_string(&path)
            .expect("should read back written file");

        prop_assert_eq!(yaml_in_memory, yaml_from_file,
            "YAML from to_yaml() and to_file() must be identical");

        let _ = std::fs::remove_dir_all(&dir);
    }
}

// ---------------------------------------------------------------------------
// Property 19: ComponentManifestBuilder validates type/version and produces
//              valid TOML
// ---------------------------------------------------------------------------

proptest! {
    /// **Property 19 (valid inputs): For any valid semver version and component
    /// name, `to_toml()` succeeds and produces valid TOML containing the
    /// specified fields.**
    ///
    /// **Validates: Requirements 28.2, 28.4**
    #[test]
    fn component_manifest_valid_inputs_produce_toml(
        name in arb_component_name(),
        version in arb_valid_semver(),
    ) {
        let result = ComponentManifestBuilder::new_for_system()
            .name(&name)
            .version(&version)
            .to_toml();

        prop_assert!(result.is_ok(),
            "to_toml() should succeed for valid name='{}' version='{}': {:?}",
            name, version, result.err());

        let toml_str = result.unwrap();

        // Must be parseable as valid TOML
        let parsed: toml::Value = toml::from_str(&toml_str)
            .map_err(|e| TestCaseError::fail(
                format!("to_toml() produced invalid TOML: {}", e)
            ))?;

        // Verify fields in parsed TOML
        prop_assert_eq!(
            parsed.get("name").and_then(|v| v.as_str()),
            Some(name.as_str()),
            "TOML name mismatch"
        );
        prop_assert_eq!(
            parsed.get("type").and_then(|v| v.as_str()),
            Some("system_component"),
            "TOML type mismatch"
        );
        prop_assert_eq!(
            parsed.get("version").and_then(|v| v.as_str()),
            Some(version.as_str()),
            "TOML version mismatch"
        );
    }

    /// **Property 19 (invalid version rejection): For any invalid semver
    /// version string, `to_toml()` returns an error.**
    ///
    /// **Validates: Requirements 28.4**
    #[test]
    fn component_manifest_rejects_invalid_semver(
        version in arb_invalid_semver(),
    ) {
        let result = ComponentManifestBuilder::new_for_system()
            .name("test-component")
            .version(&version)
            .to_toml();

        prop_assert!(result.is_err(),
            "to_toml() should reject invalid semver '{}' but succeeded", version);
    }

    /// **Property 19 (file round-trip): Writing via `to_file` and reading back
    /// produces valid TOML with matching fields.**
    ///
    /// **Validates: Requirements 28.2, 28.5**
    #[test]
    fn component_manifest_file_round_trip(
        name in arb_component_name(),
        version in arb_valid_semver(),
    ) {
        let builder = ComponentManifestBuilder::new_for_system()
            .name(&name)
            .version(&version);

        let toml_in_memory = builder.to_toml().expect("to_toml should succeed");

        let dir = std::env::temp_dir().join(format!(
            "component_manifest_prop19_{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("component-manifest.toml");

        builder.to_file(&path).expect("to_file should succeed");
        let toml_from_file = std::fs::read_to_string(&path)
            .expect("should read back written file");

        prop_assert_eq!(toml_in_memory, toml_from_file,
            "TOML from to_toml() and to_file() must be identical");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
