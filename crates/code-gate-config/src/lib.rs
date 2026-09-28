use std::{fs, path::Path};

use code_gate_core::{CargoFeatures, GateRule};
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
    pub clippy: CargoCollectorConfig,
    #[serde(default)]
    pub llvm_cov: CargoCollectorConfig,
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

/// A collector that builds the workspace with Cargo, and therefore needs to
/// know which features to enable.
///
/// ```toml
/// [collectors.llvm_cov]
/// enabled = true
/// features = "all"           # default: --all-features
/// # features = "default"     # the crates' default features
/// # features = "none"        # --no-default-features
/// # features = ["std", "serde"]
/// # default_features = false # only together with a list
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(try_from = "RawCargoCollectorConfig", into = "RawCargoCollectorConfig")]
pub struct CargoCollectorConfig {
    pub enabled: bool,
    pub features: CargoFeatures,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCargoCollectorConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    features: Option<FeaturesSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_features: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    untagged,
    expecting = "\"all\", \"default\", \"none\" or a list of feature names"
)]
enum FeaturesSpec {
    Keyword(FeaturesKeyword),
    List(Vec<String>),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum FeaturesKeyword {
    All,
    Default,
    None,
}

impl TryFrom<RawCargoCollectorConfig> for CargoCollectorConfig {
    type Error = String;

    fn try_from(raw: RawCargoCollectorConfig) -> Result<Self, Self::Error> {
        let features = match (raw.features, raw.default_features) {
            (None | Some(FeaturesSpec::Keyword(FeaturesKeyword::All)), None) => CargoFeatures::All,
            (Some(FeaturesSpec::Keyword(FeaturesKeyword::Default)), None) => {
                CargoFeatures::Selected {
                    default: true,
                    features: Vec::new(),
                }
            }
            (Some(FeaturesSpec::Keyword(FeaturesKeyword::None)), None) => CargoFeatures::Selected {
                default: false,
                features: Vec::new(),
            },
            (Some(FeaturesSpec::List(features)), default) => {
                if let Some(bad) = features.iter().find(|feature| {
                    feature.is_empty() || feature.contains(|c: char| c == ',' || c.is_whitespace())
                }) {
                    return Err(format!(
                        "invalid feature name {bad:?}: list each feature as its own string"
                    ));
                }
                CargoFeatures::Selected {
                    default: default.unwrap_or(true),
                    features,
                }
            }
            (_, Some(_)) => {
                return Err("`default_features` can only be combined with a list of \
                            `features`; use `features = \"none\"` to disable default features"
                    .to_owned());
            }
        };

        Ok(Self {
            enabled: raw.enabled,
            features,
        })
    }
}

impl From<CargoCollectorConfig> for RawCargoCollectorConfig {
    fn from(config: CargoCollectorConfig) -> Self {
        let (features, default_features) = match config.features {
            CargoFeatures::All => (None, None),
            CargoFeatures::Selected { default, features } if features.is_empty() => (
                Some(FeaturesSpec::Keyword(if default {
                    FeaturesKeyword::Default
                } else {
                    FeaturesKeyword::None
                })),
                None,
            ),
            CargoFeatures::Selected { default, features } => (
                Some(FeaturesSpec::List(features)),
                (!default).then_some(false),
            ),
        };

        Self {
            enabled: config.enabled,
            features,
            default_features,
        }
    }
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
        assert!(policy.collectors.llvm_cov.enabled);
        assert!(policy.collectors.audit.enabled);
        assert!(!policy.collectors.bca.enabled);
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

    fn llvm_cov_features(section: &str) -> Result<CargoFeatures, String> {
        parse(&format!(
            "[project]\nname = \"p\"\n\n[collectors.llvm_cov]\n{section}"
        ))
        .map(|policy| policy.collectors.llvm_cov.features)
        .map_err(|error| error.to_string())
    }

    fn selected(default: bool, features: &[&str]) -> CargoFeatures {
        CargoFeatures::Selected {
            default,
            features: features.iter().map(|&feature| feature.to_owned()).collect(),
        }
    }

    #[test]
    fn cargo_collectors_use_all_features_by_default() {
        assert_eq!(
            llvm_cov_features("enabled = true\n"),
            Ok(CargoFeatures::All)
        );
        assert_eq!(
            parse("[project]\nname = \"p\"\n")
                .unwrap()
                .collectors
                .clippy
                .features,
            CargoFeatures::All
        );
    }

    #[test]
    fn parses_feature_selections() {
        assert_eq!(
            llvm_cov_features("features = \"all\"\n"),
            Ok(CargoFeatures::All)
        );
        assert_eq!(
            llvm_cov_features("features = \"default\"\n"),
            Ok(selected(true, &[]))
        );
        assert_eq!(
            llvm_cov_features("features = \"none\"\n"),
            Ok(selected(false, &[]))
        );
        assert_eq!(
            llvm_cov_features("features = [\"std\", \"serde\"]\n"),
            Ok(selected(true, &["std", "serde"]))
        );
        assert_eq!(
            llvm_cov_features("features = [\"std\"]\ndefault_features = false\n"),
            Ok(selected(false, &["std"]))
        );
    }

    #[test]
    fn rejects_invalid_feature_selections() {
        let unknown_keyword = llvm_cov_features("features = \"al\"\n").unwrap_err();
        assert!(
            unknown_keyword.contains("a list of feature names"),
            "{unknown_keyword}"
        );

        let orphan_default = llvm_cov_features("default_features = false\n").unwrap_err();
        assert!(
            orphan_default.contains("features = \"none\""),
            "{orphan_default}"
        );

        let joined = llvm_cov_features("features = [\"std,serde\"]\n").unwrap_err();
        assert!(joined.contains("invalid feature name"), "{joined}");

        let typo = llvm_cov_features("feature = \"none\"\n").unwrap_err();
        assert!(typo.contains("feature"), "{typo}");
    }

    #[test]
    fn features_are_not_accepted_by_non_cargo_collectors() {
        assert!(
            parse("[project]\nname = \"p\"\n\n[collectors.audit]\nfeatures = \"none\"\n").is_err()
        );
    }

    #[test]
    fn rejects_unknown_collectors() {
        let error =
            parse("[project]\nname = \"p\"\n\n[collectors.clipy]\nenabled = true\n").unwrap_err();

        assert!(error.to_string().contains("clipy"), "{error}");
    }
}
