use serde::{Deserialize, Serialize};

use crate::CargoFeatures;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectMetadata {
    pub name: String,
    #[serde(default)]
    pub commit: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metric {
    pub id: String,
    pub value: f64,
    pub unit: MetricUnit,
    pub source: String,
    pub scope: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetricUnit {
    Count,
    Percent,
    Ratio,
    Score,
    Milliseconds,
    Bytes,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Finding {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub source: String,
    pub scope: String,
    #[serde(default)]
    pub location: Option<Location>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Location {
    pub path: String,
    #[serde(default)]
    pub line: Option<u32>,
    #[serde(default)]
    pub column: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateOperator {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MissingPolicy {
    Fail,
    Warn,
    Ignore,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GateRule {
    pub id: String,
    pub metric: String,
    pub scope: String,
    pub operator: GateOperator,
    pub threshold: f64,
    pub severity: Severity,
    pub missing: MissingPolicy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    Pass,
    Fail,
    Warning,
    Ignored,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GateEvaluation {
    pub rule_id: String,
    pub metric: String,
    pub scope: String,
    pub observed: Option<f64>,
    pub operator: GateOperator,
    pub threshold: f64,
    pub severity: Severity,
    pub status: GateStatus,
}

/// How one collector produced its evidence: the audit trail of a report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectorRun {
    /// The collector's `[collectors.<name>]` key.
    pub name: String,
    /// The exact command line that was run in the project directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// The Cargo feature selection, for collectors that build the workspace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<CargoFeatures>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QualityReport {
    pub project: ProjectMetadata,
    /// Absent from reports written before collectors were recorded.
    #[serde(default)]
    pub collectors: Vec<CollectorRun>,
    #[serde(default)]
    pub metrics: Vec<Metric>,
    #[serde(default)]
    pub findings: Vec<Finding>,
    #[serde(default)]
    pub gates: Vec<GateEvaluation>,
}
