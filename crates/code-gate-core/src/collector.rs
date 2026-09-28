use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Finding, Metric};

#[derive(Debug, Clone)]
pub struct CollectionContext {
    pub workspace_root: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct CollectorOutput {
    pub metrics: Vec<Metric>,
    pub findings: Vec<Finding>,
}

/// Which Cargo features a collector builds the workspace with.
///
/// Serialized in reports as `"all"` or
/// `{"selected": {"default": true, "features": ["std"]}}`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CargoFeatures {
    /// `--all-features`.
    #[default]
    All,
    /// The default features (unless `default` is false) plus `features`.
    Selected {
        default: bool,
        features: Vec<String>,
    },
}

impl CargoFeatures {
    /// The Cargo command-line flags selecting these features.
    pub fn cargo_args(&self) -> Vec<String> {
        match self {
            Self::All => vec!["--all-features".to_owned()],
            Self::Selected { default, features } => {
                let mut args = Vec::new();
                if !default {
                    args.push("--no-default-features".to_owned());
                }
                if !features.is_empty() {
                    args.push("--features".to_owned());
                    args.push(features.join(","));
                }
                args
            }
        }
    }
}

pub trait Collector {
    fn name(&self) -> &'static str;

    /// The exact command line this collector runs in `context`, recorded in
    /// the report so the evidence can be reproduced. `None` for collectors
    /// that do not run anything yet.
    fn command(&self, context: &CollectionContext) -> Option<String>;

    /// The Cargo feature selection, for collectors that build the workspace.
    fn features(&self) -> Option<&CargoFeatures> {
        None
    }

    fn collect(&self, context: &CollectionContext) -> Result<CollectorOutput, CollectorError>;
}

#[derive(Debug, Error)]
pub enum CollectorError {
    #[error("collector tool is not installed: {0}")]
    ToolMissing(String),

    #[error("collector execution failed: {0}")]
    Execution(String),

    #[error("collector output could not be parsed: {0}")]
    Parse(String),

    #[error("collector is not implemented yet: {0}")]
    NotImplemented(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selected(default: bool, features: &[&str]) -> CargoFeatures {
        CargoFeatures::Selected {
            default,
            features: features.iter().map(|&feature| feature.to_owned()).collect(),
        }
    }

    #[test]
    fn maps_features_to_cargo_flags() {
        assert_eq!(CargoFeatures::All.cargo_args(), ["--all-features"]);
        assert!(selected(true, &[]).cargo_args().is_empty());
        assert_eq!(selected(false, &[]).cargo_args(), ["--no-default-features"]);
        assert_eq!(
            selected(true, &["serde", "std"]).cargo_args(),
            ["--features", "serde,std"]
        );
        assert_eq!(
            selected(false, &["std"]).cargo_args(),
            ["--no-default-features", "--features", "std"]
        );
    }

    #[test]
    fn serializes_features_for_reports() {
        assert_eq!(
            serde_json::to_string(&CargoFeatures::All).unwrap(),
            r#""all""#
        );
        let json = r#"{"selected":{"default":false,"features":["std"]}}"#;
        assert_eq!(
            serde_json::to_string(&selected(false, &["std"])).unwrap(),
            json
        );
        assert_eq!(
            serde_json::from_str::<CargoFeatures>(json).unwrap(),
            selected(false, &["std"])
        );
    }
}
