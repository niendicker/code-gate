//! Parsing of `cargo clippy --message-format=json` output.
//!
//! This module is pure: it receives the JSON lines Cargo wrote to stdout and
//! returns domain data. It never runs processes.

use std::collections::HashSet;

use code_gate_core::{CollectorOutput, Finding, Location, Metric, MetricUnit, Severity};
use serde::Deserialize;

/// Aggregate scope of the metrics produced by this collector.
pub const PROJECT_SCOPE: &str = "project";

const CLIPPY_SOURCE: &str = "clippy";
const COMPILER_SOURCE: &str = "rustc";
const CLIPPY_PREFIX: &str = "clippy::";

/// Structured result of one `cargo clippy` run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClippyRun {
    /// Relevant diagnostics, deduplicated, in emission order.
    pub diagnostics: Vec<Diagnostic>,
    /// `success` of Cargo's final `build-finished` message, if one was seen.
    pub build_success: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    pub origin: Origin,
    pub level: Level,
    /// Lint or error code, e.g. `clippy::len_zero`, `unused_variables`, `E0308`.
    pub code: Option<String>,
    pub message: String,
    /// Cargo package that produced the diagnostic.
    pub package: Option<String>,
    pub location: Option<SourceLocation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    Clippy,
    Compiler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Level {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    pub path: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// 1-based line number in the input.
    pub line: usize,
    pub reason: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.reason)
    }
}

impl std::error::Error for ParseError {}

/// Parses the JSON-lines stream Cargo writes with `--message-format=json`.
///
/// Every non-empty line must be a Cargo JSON message. Messages other than
/// compiler diagnostics and `build-finished` are ignored.
pub fn parse_clippy_output(input: &str) -> Result<ClippyRun, ParseError> {
    let mut run = ClippyRun::default();
    let mut seen = HashSet::new();

    for (index, line) in input.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let message: CargoMessage = serde_json::from_str(line).map_err(|error| ParseError {
            line: index + 1,
            reason: error.to_string(),
        })?;

        match message {
            CargoMessage::CompilerMessage {
                package_id,
                message,
            } => {
                let Some(diagnostic) = normalize(&package_id, message) else {
                    continue;
                };
                // Cargo reports the same diagnostic once per target that
                // compiles the file (e.g. `lib` and `lib test`).
                if seen.insert(diagnostic.clone()) {
                    run.diagnostics.push(diagnostic);
                }
            }
            CargoMessage::BuildFinished { success } => run.build_success = Some(success),
            CargoMessage::Other => {}
        }
    }

    Ok(run)
}

impl ClippyRun {
    pub fn count(&self, origin: Origin, level: Level) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.origin == origin && diagnostic.level == level)
            .count()
    }

    /// Converts the run into canonical metrics and findings.
    pub fn to_collector_output(&self) -> CollectorOutput {
        let clippy_warnings = self.count(Origin::Clippy, Level::Warning);
        let clippy_errors = self.count(Origin::Clippy, Level::Error);

        let metrics = vec![
            count_metric(
                "code.clippy.total",
                clippy_warnings + clippy_errors,
                CLIPPY_SOURCE,
            ),
            count_metric("code.clippy.warnings", clippy_warnings, CLIPPY_SOURCE),
            count_metric("code.clippy.errors", clippy_errors, CLIPPY_SOURCE),
            count_metric(
                "code.compiler.warnings",
                self.count(Origin::Compiler, Level::Warning),
                COMPILER_SOURCE,
            ),
            count_metric(
                "code.compiler.errors",
                self.count(Origin::Compiler, Level::Error),
                COMPILER_SOURCE,
            ),
        ];

        let findings = self
            .diagnostics
            .iter()
            .map(Diagnostic::to_finding)
            .collect();

        CollectorOutput { metrics, findings }
    }
}

impl Diagnostic {
    pub fn to_finding(&self) -> Finding {
        let (source, rule_id) = match (self.origin, &self.code) {
            (Origin::Clippy, Some(code)) => (CLIPPY_SOURCE, code.clone()),
            (Origin::Compiler, Some(code)) => (COMPILER_SOURCE, format!("rustc::{code}")),
            // Uncoded diagnostics (e.g. syntax errors) are always compiler ones.
            (_, None) => (
                COMPILER_SOURCE,
                match self.level {
                    Level::Warning => "rustc::warning".to_owned(),
                    Level::Error => "rustc::error".to_owned(),
                },
            ),
        };

        Finding {
            rule_id,
            severity: match self.level {
                Level::Warning => Severity::Warning,
                Level::Error => Severity::Error,
            },
            message: self.message.clone(),
            source: source.to_owned(),
            scope: self
                .package
                .as_ref()
                .map(|package| format!("crate:{package}"))
                .unwrap_or_else(|| PROJECT_SCOPE.to_owned()),
            location: self.location.as_ref().map(|location| Location {
                path: location.path.clone(),
                line: Some(location.line),
                column: Some(location.column),
            }),
        }
    }
}

fn count_metric(id: &str, value: usize, source: &str) -> Metric {
    Metric {
        id: id.to_owned(),
        value: value as f64,
        unit: MetricUnit::Count,
        source: source.to_owned(),
        scope: PROJECT_SCOPE.to_owned(),
    }
}

fn normalize(package_id: &str, message: RustcDiagnostic) -> Option<Diagnostic> {
    let level = match message.level.as_str() {
        "warning" => Level::Warning,
        "error" | "error: internal compiler error" => Level::Error,
        // note, help, failure-note: context for other diagnostics.
        _ => return None,
    };

    let code = message.code.map(|code| code.code);
    let location = message
        .spans
        .iter()
        .find(|span| span.is_primary)
        .map(|span| SourceLocation {
            path: span.file_name.clone(),
            line: span.line_start,
            column: span.column_start,
        });

    if code.is_none() && location.is_none() && is_summary(&message.message) {
        return None;
    }

    let origin = match &code {
        Some(code) if code.starts_with(CLIPPY_PREFIX) => Origin::Clippy,
        _ => Origin::Compiler,
    };

    Some(Diagnostic {
        origin,
        level,
        code,
        message: message.message,
        package: package_name(package_id),
        location,
    })
}

/// Rustc's trailing summaries ("aborting due to 2 previous errors",
/// "3 warnings emitted") restate other diagnostics and must not be counted.
fn is_summary(message: &str) -> bool {
    message.starts_with("aborting due to")
        || message.ends_with("warning emitted")
        || message.ends_with("warnings emitted")
}

/// Extracts the package name from a Cargo package ID.
///
/// Supports the package ID spec format (`path+file:///ws/foo#0.1.0`,
/// `registry+https://…#foo@1.2.3`) and the legacy `foo 0.1.0 (source)` format.
fn package_name(package_id: &str) -> Option<String> {
    let name = match package_id.split_once('#') {
        Some((url, fragment)) => match fragment.split_once('@') {
            Some((name, _version)) => name,
            // Without `@`, the fragment is just a version and the name is the
            // last path segment of the URL.
            None => url.trim_end_matches('/').rsplit('/').next()?,
        },
        None => package_id.split_whitespace().next()?,
    };

    (!name.is_empty()).then(|| name.to_owned())
}

#[derive(Debug, Deserialize)]
#[serde(tag = "reason", rename_all = "kebab-case")]
enum CargoMessage {
    CompilerMessage {
        package_id: String,
        message: RustcDiagnostic,
    },
    BuildFinished {
        success: bool,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct RustcDiagnostic {
    message: String,
    level: String,
    code: Option<RustcCode>,
    #[serde(default)]
    spans: Vec<RustcSpan>,
}

#[derive(Debug, Deserialize)]
struct RustcCode {
    code: String,
}

#[derive(Debug, Deserialize)]
struct RustcSpan {
    file_name: String,
    line_start: u32,
    column_start: u32,
    is_primary: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_package_names() {
        assert_eq!(
            package_name("path+file:///work/sample#0.1.0").as_deref(),
            Some("sample")
        );
        assert_eq!(
            package_name("path+file:///work/crates/cli#code-gate-cli@0.1.0").as_deref(),
            Some("code-gate-cli")
        );
        assert_eq!(
            package_name("registry+https://github.com/rust-lang/crates.io-index#serde@1.0.228")
                .as_deref(),
            Some("serde")
        );
        assert_eq!(
            package_name("sample 0.1.0 (path+file:///work/sample)").as_deref(),
            Some("sample")
        );
        assert_eq!(package_name(""), None);
    }

    #[test]
    fn ignores_rustc_summaries() {
        assert!(is_summary("aborting due to 2 previous errors"));
        assert!(is_summary("1 warning emitted"));
        assert!(is_summary("3 warnings emitted"));
        assert!(!is_summary("unused variable: `x`"));
    }
}
