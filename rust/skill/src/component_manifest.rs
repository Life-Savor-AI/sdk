//! Component manifest builder for the Developer Portal.
//!
//! Provides [`ComponentManifestBuilder`] for programmatically constructing a
//! valid `component-manifest.toml` file with required metadata fields,
//! type validation, semver version checking, and badge support.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Builder for constructing a `component-manifest.toml` file.
///
/// The component manifest declares metadata about a component for the
/// Developer Portal: name, type, version, description, license,
/// compatibility, and status badges.
pub struct ComponentManifestBuilder {
    name: Option<String>,
    component_type: String,
    version: Option<String>,
    description: Option<String>,
    license: Option<String>,
    compatibility: Option<String>,
    badges: Vec<Badge>,
}

/// A status badge entry in the component manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Badge {
    /// Badge type, e.g. `"build"`, `"version"`, `"installs"`.
    pub badge_type: String,
    /// URL to the badge SVG image.
    pub url: String,
}

/// Serializable component manifest structure.
#[derive(Debug, Serialize, Deserialize)]
struct ComponentManifest {
    name: String,
    #[serde(rename = "type")]
    component_type: String,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    compatibility: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    badges: Vec<Badge>,
}

impl ComponentManifestBuilder {
    /// Create a new builder pre-configured for skill providers.
    ///
    /// Sets `component_type` to `"skill_provider"`.
    pub fn new_for_skill() -> Self {
        Self {
            name: None,
            component_type: "skill_provider".to_string(),
            version: None,
            description: None,
            license: None,
            compatibility: None,
            badges: Vec::new(),
        }
    }

    /// Set the component name.
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Set the component version (must be valid semver).
    pub fn version(mut self, version: &str) -> Self {
        self.version = Some(version.to_string());
        self
    }

    /// Set the component description.
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Set the license identifier (e.g. `"MIT"`, `"Apache-2.0"`).
    pub fn license(mut self, license: &str) -> Self {
        self.license = Some(license.to_string());
        self
    }

    /// Set the compatibility string (e.g. agent version range).
    pub fn compatibility(mut self, compat: &str) -> Self {
        self.compatibility = Some(compat.to_string());
        self
    }

    /// Add a status badge. The URL is generated from the component slug
    /// and badge type using the Developer Portal badge endpoint.
    pub fn badge(mut self, badge_type: &str, slug: &str) -> Self {
        let url = format!(
            "https://developer.stage.lifesavor.ai/badges/{}/{}.svg",
            slug, badge_type
        );
        self.badges.push(Badge {
            badge_type: badge_type.to_string(),
            url,
        });
        self
    }

    /// Validate and build the manifest, returning the TOML string.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `name` is not set
    /// - `version` is not set or is not valid semver
    /// - `component_type` does not match the SDK domain
    pub fn to_toml(&self) -> crate::error::Result<String> {
        let name = self.name.as_deref().ok_or_else(|| {
            crate::error::SkillSdkError::ManifestValidation(
                lifesavor_agent::registry::manifest::ManifestValidationError {
                    file_path: "component-manifest.toml".to_string(),
                    field_name: "name".to_string(),
                    description: "component name is required".to_string(),
                },
            )
        })?;

        let version = self.version.as_deref().ok_or_else(|| {
            crate::error::SkillSdkError::ManifestValidation(
                lifesavor_agent::registry::manifest::ManifestValidationError {
                    file_path: "component-manifest.toml".to_string(),
                    field_name: "version".to_string(),
                    description: "component version is required".to_string(),
                },
            )
        })?;

        if !is_valid_semver(version) {
            return Err(crate::error::SkillSdkError::ManifestValidation(
                lifesavor_agent::registry::manifest::ManifestValidationError {
                    file_path: "component-manifest.toml".to_string(),
                    field_name: "version".to_string(),
                    description: format!("'{}' is not a valid semver version", version),
                },
            ));
        }

        if self.component_type != "skill_provider" {
            return Err(crate::error::SkillSdkError::ManifestValidation(
                lifesavor_agent::registry::manifest::ManifestValidationError {
                    file_path: "component-manifest.toml".to_string(),
                    field_name: "type".to_string(),
                    description: format!(
                        "component type '{}' does not match Skill SDK domain (expected 'skill_provider')",
                        self.component_type
                    ),
                },
            ));
        }

        let manifest = ComponentManifest {
            name: name.to_string(),
            component_type: self.component_type.clone(),
            version: version.to_string(),
            description: self.description.clone(),
            license: self.license.clone(),
            compatibility: self.compatibility.clone(),
            badges: self.badges.clone(),
        };

        toml::to_string_pretty(&manifest).map_err(|e| {
            crate::error::SkillSdkError::Io(std::io::Error::other(
                format!("TOML serialization failed: {}", e),
            ))
        })
    }

    /// Validate and write the component manifest to a file.
    pub fn to_file(&self, path: &Path) -> crate::error::Result<()> {
        let toml_str = self.to_toml()?;
        std::fs::write(path, toml_str)?;
        Ok(())
    }
}

/// Check if a version string is valid semver (MAJOR.MINOR.PATCH with optional pre-release).
fn is_valid_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.splitn(2, '-').collect();
    let core = parts[0];
    let segments: Vec<&str> = core.split('.').collect();
    if segments.len() != 3 {
        return false;
    }
    for seg in &segments {
        if seg.is_empty() || !seg.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    if let Some(pre) = parts.get(1) {
        if pre.is_empty() {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_skill_manifest() {
        let toml_str = ComponentManifestBuilder::new_for_skill()
            .name("my-skill")
            .version("0.1.0")
            .description("A skill provider")
            .license("MIT")
            .to_toml()
            .unwrap();
        assert!(toml_str.contains("name = \"my-skill\""));
        assert!(toml_str.contains("type = \"skill_provider\""));
        assert!(toml_str.contains("version = \"0.1.0\""));
    }

    #[test]
    fn test_missing_name_errors() {
        let result = ComponentManifestBuilder::new_for_skill()
            .version("0.1.0")
            .to_toml();
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_version_errors() {
        let result = ComponentManifestBuilder::new_for_skill()
            .name("test")
            .version("not-semver")
            .to_toml();
        assert!(result.is_err());
    }

    #[test]
    fn test_badge_generation() {
        let toml_str = ComponentManifestBuilder::new_for_skill()
            .name("my-skill")
            .version("1.0.0")
            .badge("build", "my-skill")
            .to_toml()
            .unwrap();
        assert!(toml_str.contains("developer.stage.lifesavor.ai/badges/my-skill/build.svg"));
    }
}
