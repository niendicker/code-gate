//! Parsing of `cargo llvm-cov --json --summary-only` output.
//!
//! The input is LLVM's `llvm.coverage.json.export` document. This module is
//! pure: it never runs processes.

use code_gate_core::{CollectorOutput, Metric, MetricUnit};
use serde::Deserialize;

/// Aggregate scope of the metrics produced by this collector.
pub const PROJECT_SCOPE: &str = "project";

const SOURCE: &str = "cargo-llvm-cov";
const EXPORT_TYPE: &str = "llvm.coverage.json.export";

/// Project-wide coverage totals.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CoverageTotals {
    pub lines: Counter,
    pub functions: Counter,
    pub regions: Counter,
    pub branches: Counter,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct Counter {
    pub count: u64,
    pub covered: u64,
}

impl Counter {
    /// Covered percentage, or `None` when nothing was instrumented: 0 of 0 is
    /// "not measured", not 0 % or 100 %.
    pub fn percent(&self) -> Option<f64> {
        (self.count > 0).then(|| self.covered as f64 * 100.0 / self.count as f64)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

/// Parses an LLVM coverage JSON export and returns its totals.
pub fn parse_coverage_output(input: &str) -> Result<CoverageTotals, ParseError> {
    let export: Export =
        serde_json::from_str(input).map_err(|error| ParseError(error.to_string()))?;

    match export.kind.as_deref() {
        None | Some(EXPORT_TYPE) => {}
        Some(kind) => {
            return Err(ParseError(format!(
                "unexpected export type `{kind}`, expected `{EXPORT_TYPE}`"
            )));
        }
    }

    // cargo-llvm-cov merges all test binaries into a single export.
    let [data] = <[Data; 1]>::try_from(export.data).map_err(|data| {
        ParseError(format!(
            "expected exactly one coverage export, found {}",
            data.len()
        ))
    })?;

    let totals = data.totals;
    Ok(CoverageTotals {
        lines: totals.lines,
        functions: totals.functions,
        regions: totals.regions,
        branches: totals.branches.unwrap_or_default(),
    })
}

impl CoverageTotals {
    /// Canonical percentage metrics. A kind with nothing instrumented (e.g.
    /// branches without `-Z coverage-options=branch`) is omitted so that the
    /// policy's `missing` rule applies instead of a fabricated value.
    pub fn to_collector_output(&self) -> CollectorOutput {
        let metrics = [
            ("test.coverage.lines", self.lines),
            ("test.coverage.functions", self.functions),
            ("test.coverage.regions", self.regions),
            ("test.coverage.branches", self.branches),
        ]
        .into_iter()
        .filter_map(|(id, counter)| {
            counter.percent().map(|value| Metric {
                id: id.to_owned(),
                value,
                unit: MetricUnit::Percent,
                source: SOURCE.to_owned(),
                scope: PROJECT_SCOPE.to_owned(),
            })
        })
        .collect();

        CollectorOutput {
            metrics,
            findings: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Export {
    #[serde(rename = "type")]
    kind: Option<String>,
    data: Vec<Data>,
}

#[derive(Debug, Deserialize)]
struct Data {
    totals: Totals,
}

#[derive(Debug, Deserialize)]
struct Totals {
    lines: Counter,
    functions: Counter,
    regions: Counter,
    #[serde(default)]
    branches: Option<Counter>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_is_undefined_without_instrumented_items() {
        assert_eq!(Counter::default().percent(), None);
        assert_eq!(
            Counter {
                count: 4,
                covered: 3
            }
            .percent(),
            Some(75.0)
        );
    }
}
