use std::{fs, path::Path};

use code_gate_core::QualityReport;
use thiserror::Error;

pub fn load_report(path: impl AsRef<Path>) -> Result<QualityReport, ReportError> {
    let content = fs::read_to_string(path).map_err(ReportError::Io)?;
    serde_json::from_str(&content).map_err(ReportError::Json)
}

pub fn write_report(path: impl AsRef<Path>, report: &QualityReport) -> Result<(), ReportError> {
    let content = serde_json::to_string_pretty(report).map_err(ReportError::Json)?;
    fs::write(path, content).map_err(ReportError::Io)
}

#[derive(Debug, Error)]
pub enum ReportError {
    #[error("report I/O failed: {0}")]
    Io(std::io::Error),

    #[error("invalid report JSON: {0}")]
    Json(serde_json::Error),
}
