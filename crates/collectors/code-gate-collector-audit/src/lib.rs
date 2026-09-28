//! cargo-audit collector.
//!
//! Runs `cargo audit --json` against the project's `Cargo.lock`, parses the
//! RustSec report and normalizes it into canonical `security.vulnerabilities.*`
//! and `dependency.*` counts, with one finding per advisory hit. Thresholds
//! belong to the policy, not to this collector.

mod parse;

use std::path::Path;

use code_gate_core::{CollectionContext, Collector, CollectorError, CollectorOutput};
use code_gate_process::{Invocation, RunError, SystemRunner, ToolOutput, ToolRunner};

pub use parse::{
    AuditReport, PROJECT_SCOPE, ParseError, Vulnerability, VulnerabilitySeverity, Warning,
    parse_audit_output,
};

/// Maximum number of stderr lines quoted in error messages.
const STDERR_TAIL_LINES: usize = 20;

#[derive(Debug, Default)]
pub struct AuditCollector<R = SystemRunner> {
    runner: R,
}

impl AuditCollector {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<R: ToolRunner> AuditCollector<R> {
    pub fn with_runner(runner: R) -> Self {
        Self { runner }
    }

    /// The exact command this collector runs in `workspace_root`.
    pub fn invocation(workspace_root: &Path) -> Invocation {
        Invocation::new("cargo", workspace_root).args(["audit", "--json"])
    }

    /// Runs the audit and returns the raw output, without interpreting it.
    pub fn execute_audit(&self, workspace_root: &Path) -> Result<ToolOutput, CollectorError> {
        let invocation = Self::invocation(workspace_root);
        self.runner.run(&invocation).map_err(|error| match error {
            RunError::NotFound { program } => CollectorError::ToolMissing(format!(
                "`{program}` was not found on PATH; install a Rust toolchain"
            )),
            other => CollectorError::Execution(format!("`{}`: {other}", invocation.display())),
        })
    }
}

impl<R: ToolRunner> Collector for AuditCollector<R> {
    fn name(&self) -> &'static str {
        "audit"
    }

    fn collect(&self, context: &CollectionContext) -> Result<CollectorOutput, CollectorError> {
        let output = self.execute_audit(&context.workspace_root)?;
        interpret(&output).map(|report| report.to_collector_output())
    }
}

/// Classifies a finished `cargo audit --json` run.
///
/// cargo-audit exits non-zero when it *finds* vulnerabilities (or denied
/// warnings), so the exit code alone does not mean failure: a parsable report
/// on stdout is evidence whatever the exit status. Only a run without a
/// report is a collection failure.
pub fn interpret(output: &ToolOutput) -> Result<AuditReport, CollectorError> {
    let stdout = std::str::from_utf8(&output.stdout)
        .map_err(|error| CollectorError::Parse(format!("stdout is not valid UTF-8: {error}")))?;

    match parse_audit_output(stdout) {
        Ok(report) => return Ok(report),
        Err(error) if output.success() => {
            return Err(CollectorError::Parse(format!(
                "cargo audit JSON output: {error}"
            )));
        }
        Err(_) => {}
    }

    let full_stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = output.stderr_tail(STDERR_TAIL_LINES);

    Err(if audit_missing(&full_stderr) {
        CollectorError::ToolMissing(format!(
            "cargo-audit is not installed (try `cargo install cargo-audit --locked` \
             or your distribution's package): {stderr}"
        ))
    } else {
        CollectorError::Execution(format!(
            "cargo audit exited with {} without a report: {stderr}",
            output.exit_description()
        ))
    })
}

fn audit_missing(stderr: &str) -> bool {
    stderr.contains("no such command: `audit`")
}
