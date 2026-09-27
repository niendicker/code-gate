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
