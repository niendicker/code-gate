//! Collector behavior over simulated process results; no tests are executed.

use std::path::PathBuf;

use code_gate_collector_llvm_cov::LlvmCovCollector;
use code_gate_core::{
    CargoFeatures, CollectionContext, Collector, CollectorError, CollectorOutput,
};
use code_gate_process::{Invocation, RunError, ToolOutput, ToolRunner};

struct FakeRunner(Option<ToolOutput>);

impl ToolRunner for FakeRunner {
    fn run(&self, invocation: &Invocation) -> Result<ToolOutput, RunError> {
        assert_eq!(
            invocation.display(),
            "cargo llvm-cov --workspace --all-features --json --summary-only"
        );
        assert_eq!(invocation.current_dir, PathBuf::from("/work/cov"));

        self.0.clone().ok_or_else(|| RunError::NotFound {
            program: "cargo".into(),
        })
    }
}

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(path).unwrap()
}

fn run(
    exit_code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
) -> Result<CollectorOutput, CollectorError> {
    let runner = FakeRunner(Some(ToolOutput {
        exit_code: Some(exit_code),
        stdout,
        stderr,
    }));
    let context = CollectionContext {
        workspace_root: PathBuf::from("/work/cov"),
    };
    LlvmCovCollector::with_runner(runner).collect(&context)
}

#[test]
fn successful_run_produces_metrics() {
    let output = run(
        0,
        fixture("coverage-partial.json"),
        b"running 1 test".to_vec(),
    )
    .unwrap();

    assert_eq!(output.metrics.len(), 3);
}

#[test]
fn failing_tests_are_reported_as_such() {
    match run(101, Vec::new(), fixture("tests-failed.stderr")) {
        Err(CollectorError::Execution(message)) => {
            assert!(message.contains("test suite failed"), "{message}");
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn missing_cargo_llvm_cov_is_tool_missing() {
    let stderr = b"error: no such command: `llvm-cov`\n\nhelp: find a package".to_vec();

    assert!(matches!(
        run(101, Vec::new(), stderr),
        Err(CollectorError::ToolMissing(message)) if message.contains("cargo-llvm-cov")
    ));
}

#[test]
fn missing_llvm_tools_is_tool_missing_with_hint() {
    let stderr = b"error: failed to find llvm-tools-preview, please install llvm-tools-preview, \
                   or set LLVM_COV and LLVM_PROFDATA environment variables"
        .to_vec();

    assert!(matches!(
        run(1, Vec::new(), stderr),
        Err(CollectorError::ToolMissing(message)) if message.contains("LLVM_PROFDATA")
    ));
}

#[test]
fn missing_cargo_is_tool_missing() {
    let context = CollectionContext {
        workspace_root: PathBuf::from("/work/cov"),
    };

    assert!(matches!(
        LlvmCovCollector::with_runner(FakeRunner(None)).collect(&context),
        Err(CollectorError::ToolMissing(_))
    ));
}

#[test]
fn other_failures_are_execution_errors() {
    let stderr = b"error: failed to load coverage: no coverage data found".to_vec();

    match run(1, Vec::new(), stderr) {
        Err(CollectorError::Execution(message)) => {
            assert!(message.contains("exit code 1"), "{message}");
            assert!(message.contains("no coverage data found"), "{message}");
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn unparsable_output_is_parse_error() {
    assert!(matches!(
        run(0, b"not json".to_vec(), Vec::new()),
        Err(CollectorError::Parse(_))
    ));
}

/// Records the command line instead of running it.
struct RecordingRunner(std::cell::RefCell<Option<String>>);

impl ToolRunner for &RecordingRunner {
    fn run(&self, invocation: &Invocation) -> Result<ToolOutput, RunError> {
        *self.0.borrow_mut() = Some(invocation.display());
        Err(RunError::NotFound {
            program: "cargo".into(),
        })
    }
}

#[test]
fn feature_selection_reaches_the_command_line() {
    let runner = RecordingRunner(Default::default());
    let context = CollectionContext {
        workspace_root: PathBuf::from("/work/cov"),
    };
    let _ = LlvmCovCollector::with_runner(&runner)
        .with_features(CargoFeatures::Selected {
            default: false,
            features: vec!["std".into(), "serde".into()],
        })
        .collect(&context);

    assert_eq!(
        runner.0.borrow().as_deref(),
        Some(
            "cargo llvm-cov --workspace --no-default-features --features std,serde --json --summary-only"
        )
    );
}
