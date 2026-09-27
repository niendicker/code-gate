//! Collector behavior over simulated process results; no Cargo is executed.

use std::{cell::RefCell, path::PathBuf};

use code_gate_collector_clippy::ClippyCollector;
use code_gate_core::{CollectionContext, Collector, CollectorError};
use code_gate_process::{Invocation, RunError, ToolOutput, ToolRunner};

enum Scripted {
    Output(ToolOutput),
    NotFound,
}

struct FakeRunner {
    result: Scripted,
    invocations: RefCell<Vec<Invocation>>,
}

impl FakeRunner {
    fn new(result: Scripted) -> Self {
        Self {
            result,
            invocations: RefCell::new(Vec::new()),
        }
    }
}

impl ToolRunner for &FakeRunner {
    fn run(&self, invocation: &Invocation) -> Result<ToolOutput, RunError> {
        self.invocations.borrow_mut().push(invocation.clone());
        match &self.result {
            Scripted::Output(output) => Ok(output.clone()),
            Scripted::NotFound => Err(RunError::NotFound {
                program: invocation.program.to_string_lossy().into_owned(),
            }),
        }
    }
}

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(path).unwrap()
}

fn output(exit_code: i32, stdout: Vec<u8>, stderr: &str) -> Scripted {
    Scripted::Output(ToolOutput {
        exit_code: Some(exit_code),
        stdout,
        stderr: stderr.as_bytes().to_vec(),
    })
}

fn collect(runner: &FakeRunner) -> Result<code_gate_core::CollectorOutput, CollectorError> {
    let context = CollectionContext {
        workspace_root: PathBuf::from("/work/sample"),
    };
    ClippyCollector::with_runner(runner).collect(&context)
}

#[test]
fn runs_cargo_clippy_with_json_diagnostics_in_workspace_root() {
    let runner = FakeRunner::new(output(0, fixture("clippy-clean.jsonl"), ""));
    collect(&runner).unwrap();

    let invocations = runner.invocations.borrow();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].current_dir, PathBuf::from("/work/sample"));
    assert_eq!(
        invocations[0].display(),
        "cargo clippy --workspace --all-targets --all-features --message-format=json"
    );
}

#[test]
fn warnings_are_evidence_not_failure() {
    let runner = FakeRunner::new(output(0, fixture("clippy-warnings.jsonl"), ""));
    let collected = collect(&runner).unwrap();

    assert_eq!(collected.findings.len(), 3);
}

#[test]
fn failed_build_with_diagnostics_is_evidence_not_failure() {
    // Denied lints and compile errors make cargo exit with 101.
    for name in ["clippy-error.jsonl", "compiler-error.jsonl"] {
        let runner = FakeRunner::new(output(101, fixture(name), "error: could not compile"));
        let collected = collect(&runner).unwrap();

        assert_eq!(collected.findings.len(), 1, "{name}");
    }
}

#[test]
fn missing_cargo_is_tool_missing() {
    let runner = FakeRunner::new(Scripted::NotFound);

    assert!(matches!(
        collect(&runner),
        Err(CollectorError::ToolMissing(_))
    ));
}

#[test]
fn missing_clippy_component_is_tool_missing() {
    for stderr in [
        "error: no such command: `clippy`",
        "error: 'cargo-clippy' is not installed for the toolchain 'stable-x86_64-unknown-linux-gnu'",
    ] {
        let runner = FakeRunner::new(output(1, Vec::new(), stderr));

        assert!(
            matches!(collect(&runner), Err(CollectorError::ToolMissing(_))),
            "{stderr}"
        );
    }
}

#[test]
fn cargo_failure_before_build_is_execution_error() {
    let stderr = "error: could not find `Cargo.toml` in `/work/sample` or any parent directory";
    let runner = FakeRunner::new(output(101, Vec::new(), stderr));

    match collect(&runner) {
        Err(CollectorError::Execution(message)) => {
            assert!(message.contains("exit code 101"), "{message}");
            assert!(message.contains("could not find `Cargo.toml`"), "{message}");
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn failed_build_without_diagnostics_is_execution_error() {
    // e.g. a build script panicked: cargo reports failure but no diagnostics.
    let stdout = br#"{"reason":"build-finished","success":false}"#.to_vec();
    let runner = FakeRunner::new(output(
        101,
        stdout,
        "error: failed to run custom build command",
    ));

    assert!(matches!(
        collect(&runner),
        Err(CollectorError::Execution(_))
    ));
}

#[test]
fn unparsable_output_is_parse_error() {
    let runner = FakeRunner::new(output(0, b"warning: this is human output\n".to_vec(), ""));

    match collect(&runner) {
        Err(CollectorError::Parse(message)) => assert!(message.contains("line 1"), "{message}"),
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn non_utf8_output_is_parse_error() {
    let runner = FakeRunner::new(output(0, vec![0xff, 0xfe], ""));

    assert!(matches!(collect(&runner), Err(CollectorError::Parse(_))));
}
