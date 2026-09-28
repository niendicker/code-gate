//! cargo-llvm-cov collector.
//!
//! Runs `cargo llvm-cov --json --summary-only`, parses LLVM's coverage export
//! and normalizes the project totals into canonical `test.coverage.*`
//! percentages. Thresholds belong to the policy, not to this collector.

mod parse;

use std::path::Path;

use code_gate_core::{
    CargoFeatures, CollectionContext, Collector, CollectorError, CollectorOutput,
};
use code_gate_process::{Invocation, RunError, SystemRunner, ToolOutput, ToolRunner};

pub use parse::{Counter, CoverageTotals, PROJECT_SCOPE, ParseError, parse_coverage_output};

/// Maximum number of stderr lines quoted in error messages.
const STDERR_TAIL_LINES: usize = 20;

#[derive(Debug, Default)]
pub struct LlvmCovCollector<R = SystemRunner> {
    runner: R,
    features: CargoFeatures,
}

impl LlvmCovCollector {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<R: ToolRunner> LlvmCovCollector<R> {
    pub fn with_runner(runner: R) -> Self {
        Self {
            runner,
            features: CargoFeatures::default(),
        }
    }

    /// Builds with `features` instead of `--all-features`.
    pub fn with_features(mut self, features: CargoFeatures) -> Self {
        self.features = features;
        self
    }

    /// The exact command this collector runs in `workspace_root`.
    pub fn invocation(workspace_root: &Path, features: &CargoFeatures) -> Invocation {
        Invocation::new("cargo", workspace_root)
            .args(["llvm-cov", "--workspace"])
            .args(features.cargo_args())
            .args(["--json", "--summary-only"])
    }

    /// Runs the instrumented test suite and returns the raw output, without
    /// interpreting it.
    pub fn execute_llvm_cov(&self, workspace_root: &Path) -> Result<ToolOutput, CollectorError> {
        let invocation = Self::invocation(workspace_root, &self.features);
        self.runner.run(&invocation).map_err(|error| match error {
            RunError::NotFound { program } => CollectorError::ToolMissing(format!(
                "`{program}` was not found on PATH; install a Rust toolchain"
            )),
            other => CollectorError::Execution(format!("`{}`: {other}", invocation.display())),
        })
    }
}

impl<R: ToolRunner> Collector for LlvmCovCollector<R> {
    fn name(&self) -> &'static str {
        "llvm-cov"
    }

    fn collect(&self, context: &CollectionContext) -> Result<CollectorOutput, CollectorError> {
        let output = self.execute_llvm_cov(&context.workspace_root)?;
        interpret(&output).map(|totals| totals.to_collector_output())
    }
}

/// Classifies a finished `cargo llvm-cov` run.
///
/// Coverage is only meaningful for a test suite that passed, and
/// cargo-llvm-cov produces no report otherwise, so failing tests are a
/// collection failure rather than a coverage value.
pub fn interpret(output: &ToolOutput) -> Result<CoverageTotals, CollectorError> {
    if !output.success() {
        let full_stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = output.stderr_tail(STDERR_TAIL_LINES);

        return Err(if llvm_cov_missing(&full_stderr) {
            CollectorError::ToolMissing(format!(
                "cargo-llvm-cov is not installed (try `cargo install cargo-llvm-cov` \
                 or your distribution's package): {stderr}"
            ))
        } else if llvm_tools_missing(&full_stderr) {
            CollectorError::ToolMissing(format!(
                "LLVM tools not found; run `rustup component add llvm-tools-preview`, or \
                 without rustup set LLVM_COV and LLVM_PROFDATA to a matching LLVM \
                 (e.g. /usr/bin/llvm-cov and /usr/bin/llvm-profdata): {stderr}"
            ))
        } else if tests_failed(&full_stderr) {
            CollectorError::Execution(format!(
                "the test suite failed, so coverage was not measured: {stderr}"
            ))
        } else {
            CollectorError::Execution(format!(
                "cargo llvm-cov exited with {}: {stderr}",
                output.exit_description()
            ))
        });
    }

    let stdout = std::str::from_utf8(&output.stdout)
        .map_err(|error| CollectorError::Parse(format!("stdout is not valid UTF-8: {error}")))?;

    parse_coverage_output(stdout)
        .map_err(|error| CollectorError::Parse(format!("cargo llvm-cov JSON output: {error}")))
}

fn llvm_cov_missing(stderr: &str) -> bool {
    stderr.contains("no such command: `llvm-cov`")
}

fn llvm_tools_missing(stderr: &str) -> bool {
    stderr.contains("failed to find llvm-tools-preview")
}

fn tests_failed(stderr: &str) -> bool {
    stderr.contains("test result: FAILED") || stderr.contains("error: test failed")
}
