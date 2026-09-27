//! Clippy collector.
//!
//! Runs `cargo clippy --message-format=json`, parses Cargo's structured
//! diagnostics and normalizes them into canonical metrics and findings. It
//! never decides whether the project passes: that is the gate engine's job.

mod parse;

use std::path::Path;

use code_gate_core::{CollectionContext, Collector, CollectorError, CollectorOutput};
use code_gate_process::{Invocation, RunError, SystemRunner, ToolOutput, ToolRunner};

pub use parse::{
    ClippyRun, Diagnostic, Level, Origin, PROJECT_SCOPE, ParseError, SourceLocation,
    parse_clippy_output,
};

/// Maximum number of stderr lines quoted in error messages.
const STDERR_TAIL_LINES: usize = 20;

#[derive(Debug, Default)]
pub struct ClippyCollector<R = SystemRunner> {
    runner: R,
}

impl ClippyCollector {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<R: ToolRunner> ClippyCollector<R> {
    pub fn with_runner(runner: R) -> Self {
        Self { runner }
    }

    /// The exact command this collector runs in `workspace_root`.
    pub fn invocation(workspace_root: &Path) -> Invocation {
        Invocation::new("cargo", workspace_root).args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--message-format=json",
        ])
    }

    /// Runs Clippy and returns its raw output, without interpreting it.
    pub fn execute_clippy(&self, workspace_root: &Path) -> Result<ToolOutput, CollectorError> {
        let invocation = Self::invocation(workspace_root);
        self.runner.run(&invocation).map_err(|error| match error {
            RunError::NotFound { program } => CollectorError::ToolMissing(format!(
                "`{program}` was not found on PATH; install a Rust toolchain"
            )),
            other => CollectorError::Execution(format!("`{}`: {other}", invocation.display())),
        })
    }
}

impl<R: ToolRunner> Collector for ClippyCollector<R> {
    fn name(&self) -> &'static str {
        "clippy"
    }

    fn collect(&self, context: &CollectionContext) -> Result<CollectorOutput, CollectorError> {
        let output = self.execute_clippy(&context.workspace_root)?;
        interpret(&output).map(|run| run.to_collector_output())
    }
}

/// Separates "Clippy ran and found problems" from "Clippy could not run".
///
/// Warnings and errors reported as diagnostics are evidence, not failures.
/// A run is only a collector failure when Cargo produced no build result, or
/// when the build failed without any diagnostic explaining why (for example a
/// failing build script), because the metrics would then be incomplete.
pub fn interpret(output: &ToolOutput) -> Result<ClippyRun, CollectorError> {
    let stdout = std::str::from_utf8(&output.stdout)
        .map_err(|error| CollectorError::Parse(format!("stdout is not valid UTF-8: {error}")))?;
    let run = parse_clippy_output(stdout)
        .map_err(|error| CollectorError::Parse(format!("cargo clippy JSON output, {error}")))?;

    let stderr = output.stderr_tail(STDERR_TAIL_LINES);

    match run.build_success {
        None if clippy_missing(&stderr) => Err(CollectorError::ToolMissing(format!(
            "cargo-clippy is not installed (try `rustup component add clippy`): {stderr}"
        ))),
        None => Err(CollectorError::Execution(format!(
            "cargo clippy exited with {} without finishing the build: {stderr}",
            output.exit_description()
        ))),
        Some(false)
            if run.count(Origin::Clippy, Level::Error) == 0
                && run.count(Origin::Compiler, Level::Error) == 0 =>
        {
            Err(CollectorError::Execution(format!(
                "cargo clippy build failed without reporting diagnostics: {stderr}"
            )))
        }
        Some(_) => Ok(run),
    }
}

fn clippy_missing(stderr: &str) -> bool {
    stderr.contains("no such command: `clippy`")
        || (stderr.contains("cargo-clippy") && stderr.contains("not installed"))
}
