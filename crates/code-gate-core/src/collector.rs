use std::path::PathBuf;

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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
}
