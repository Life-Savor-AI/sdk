//! Build configuration builder for the Developer Portal pipeline.
//!
//! Provides [`BuildConfigBuilder`] for programmatically constructing a valid
//! `lifesavor-build.yml` file with the correct runtime, build commands, test
//! commands, environment variables, and output directory for LLM providers.

use std::collections::HashMap;
use std::path::Path;

/// Builder for constructing a `lifesavor-build.yml` configuration file.
///
/// The build config declares how the Developer Portal's CodeBuild pipeline
/// should build and test an LLM provider component.
pub struct BuildConfigBuilder {
    runtime: String,
    build_commands: Vec<String>,
    test_commands: Vec<String>,
    env_vars: HashMap<String, String>,
    output_dir: String,
}

impl BuildConfigBuilder {
    /// Create a new builder pre-configured for model (LLM) providers.
    ///
    /// Defaults:
    /// - runtime: `"rust"`
    /// - build commands: `["cargo build --release"]`
    /// - test commands: `["cargo test"]`
    /// - output dir: `"target/release"`
    pub fn new_for_model() -> Self {
        Self {
            runtime: "rust".to_string(),
            build_commands: vec!["cargo build --release".to_string()],
            test_commands: vec!["cargo test".to_string()],
            env_vars: HashMap::new(),
            output_dir: "target/release".to_string(),
        }
    }

    /// Add an environment variable to the build configuration.
    pub fn env_var(mut self, key: &str, value: &str) -> Self {
        self.env_vars.insert(key.to_string(), value.to_string());
        self
    }

    /// Add an additional build step command.
    pub fn build_step(mut self, cmd: &str) -> Self {
        self.build_commands.push(cmd.to_string());
        self
    }

    /// Add an additional test command.
    pub fn test_step(mut self, cmd: &str) -> Self {
        self.test_commands.push(cmd.to_string());
        self
    }

    /// Set the output directory.
    pub fn output_dir(mut self, dir: &str) -> Self {
        self.output_dir = dir.to_string();
        self
    }

    /// Render the build config as a YAML string.
    pub fn to_yaml(&self) -> String {
        let mut yaml = String::new();
        yaml.push_str(&format!("runtime: {}\n", self.runtime));

        yaml.push_str("build_commands:\n");
        for cmd in &self.build_commands {
            yaml.push_str(&format!("  - \"{}\"\n", cmd));
        }

        yaml.push_str("test_commands:\n");
        for cmd in &self.test_commands {
            yaml.push_str(&format!("  - \"{}\"\n", cmd));
        }

        if !self.env_vars.is_empty() {
            yaml.push_str("env_vars:\n");
            let mut keys: Vec<&String> = self.env_vars.keys().collect();
            keys.sort();
            for key in keys {
                yaml.push_str(&format!("  {}: \"{}\"\n", key, self.env_vars[key]));
            }
        }

        yaml.push_str(&format!("output_dir: {}\n", self.output_dir));
        yaml
    }

    /// Write the build config to a file at the given path.
    pub fn to_file(&self, path: &Path) -> crate::error::Result<()> {
        std::fs::write(path, self.to_yaml())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model_build_config() {
        let builder = BuildConfigBuilder::new_for_model();
        let yaml = builder.to_yaml();
        assert!(yaml.contains("runtime: rust"));
        assert!(yaml.contains("cargo build --release"));
        assert!(yaml.contains("cargo test"));
        assert!(yaml.contains("output_dir: target/release"));
    }

    #[test]
    fn test_custom_env_vars_and_steps() {
        let yaml = BuildConfigBuilder::new_for_model()
            .env_var("RUST_LOG", "debug")
            .build_step("cargo clippy")
            .to_yaml();
        assert!(yaml.contains("RUST_LOG: \"debug\""));
        assert!(yaml.contains("cargo clippy"));
    }

    #[test]
    fn test_to_file_writes_yaml() {
        let dir = std::env::temp_dir().join("build_config_test_model");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("lifesavor-build.yml");
        BuildConfigBuilder::new_for_model()
            .to_file(&path)
            .unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("runtime: rust"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
