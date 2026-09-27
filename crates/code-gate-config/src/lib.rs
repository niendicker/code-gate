use std::{fs, path::Path};

use code_gate_core::GateRule;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    #[serde(default = "default_criticality")]
    pub default_criticality: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityPolicy {
    pub project: ProjectConfig,
    #[serde(default)]
    pub collectors: CollectorsConfig,
    #[serde(default)]
    pub gates: Vec<GateRule>,
}

/// Which collectors `code-gate collect` runs. Every collector is disabled
/// unless its section explicitly sets `enabled = true`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectorsConfig {
    #[serde(default)]
    pub clippy: CollectorConfig,
    #[serde(default)]
    pub llvm_cov: CollectorConfig,
    #[serde(default)]
    pub audit: CollectorConfig,
    #[serde(default)]
    pub bca: CollectorConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectorConfig {
    #[serde(default)]
    pub enabled: bool,
}

pub fn load_policy(path: impl AsRef<Path>) -> Result<QualityPolicy, ConfigError> {
    let content = fs::read_to_string(path).map_err(ConfigError::Io)?;
    toml::from_str(&content).map_err(ConfigError::Toml)
}

pub fn default_policy_toml() -> &'static str {
    include_str!("../../../quality.example.toml")
}

fn default_criticality() -> String {
    "B".to_owned()
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read quality policy: {0}")]
    Io(std::io::Error),

    #[error("invalid quality policy: {0}")]
    Toml(toml::de::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(content: &str) -> Result<QualityPolicy, toml::de::Error> {
        toml::from_str(content)
    }

    #[test]
    fn default_policy_is_valid() {
        let policy = parse(default_policy_toml()).unwrap();

        assert!(policy.collectors.clippy.enabled);
        assert!(!policy.collectors.llvm_cov.enabled);
        assert!(!policy.gates.is_empty());
    }

    #[test]
    fn collectors_are_disabled_when_not_configured() {
        let policy = parse("[project]\nname = \"p\"\n").unwrap();

        assert!(!policy.collectors.clippy.enabled);
        assert!(!policy.collectors.llvm_cov.enabled);
        assert!(!policy.collectors.audit.enabled);
        assert!(!policy.collectors.bca.enabled);
    }

    #[test]
    fn enables_individual_collectors() {
        let policy = parse(
            "[project]\nname = \"p\"\n\n[collectors.clippy]\nenabled = true\n\n[collectors.bca]\nenabled = false\n",
        )
        .unwrap();

        assert!(policy.collectors.clippy.enabled);
        assert!(!policy.collectors.bca.enabled);
    }

    #[test]
    fn rejects_unknown_collectors() {
        let error =
            parse("[project]\nname = \"p\"\n\n[collectors.clipy]\nenabled = true\n").unwrap_err();

        assert!(error.to_string().contains("clipy"), "{error}");
    }
}
