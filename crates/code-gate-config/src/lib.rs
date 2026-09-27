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
    pub gates: Vec<GateRule>,
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
