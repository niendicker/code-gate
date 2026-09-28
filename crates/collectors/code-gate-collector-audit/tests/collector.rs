//! Collector behavior over simulated process results; cargo-audit is not run.

use std::path::PathBuf;

use code_gate_collector_audit::AuditCollector;
use code_gate_core::{CollectionContext, Collector, CollectorError, CollectorOutput};
use code_gate_process::{Invocation, RunError, ToolOutput, ToolRunner};

struct FakeRunner(Option<ToolOutput>);

impl ToolRunner for FakeRunner {
    fn run(&self, invocation: &Invocation) -> Result<ToolOutput, RunError> {
        assert_eq!(invocation.display(), "cargo audit --json");
        assert_eq!(invocation.current_dir, PathBuf::from("/work/audit"));

        self.0.clone().ok_or_else(|| RunError::NotFound {
            program: "cargo".into(),
        })
    }
}

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(path).unwrap()
}

fn context() -> CollectionContext {
    CollectionContext {
        workspace_root: PathBuf::from("/work/audit"),
    }
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
    AuditCollector::with_runner(runner).collect(&context())
}

#[test]
fn clean_audit_produces_metrics() {
    let output = run(0, fixture("audit-clean.json"), Vec::new()).unwrap();

    assert_eq!(output.metrics.len(), 9);
}

#[test]
fn vulnerabilities_found_is_evidence_not_failure() {
    // cargo-audit exits 1 when it finds vulnerabilities.
    let output = run(1, fixture("audit-vulnerable.json"), Vec::new()).unwrap();

    assert_eq!(output.findings.len(), 3);
}

#[test]
fn missing_cargo_audit_is_tool_missing() {
    let stderr =
        b"error: no such command: `audit`\n\nhelp: a command with a similar name exists: `add`"
            .to_vec();

    assert!(matches!(
        run(101, Vec::new(), stderr),
        Err(CollectorError::ToolMissing(message)) if message.contains("cargo install cargo-audit")
    ));
}

#[test]
fn missing_cargo_is_tool_missing() {
    assert!(matches!(
        AuditCollector::with_runner(FakeRunner(None)).collect(&context()),
        Err(CollectorError::ToolMissing(_))
    ));
}

#[test]
fn failure_without_report_is_execution_error() {
    match run(2, Vec::new(), fixture("no-lockfile.stderr")) {
        Err(CollectorError::Execution(message)) => {
            assert!(message.contains("exit code 2"), "{message}");
            assert!(message.contains("Couldn't load Cargo.lock"), "{message}");
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn unparsable_output_on_success_is_parse_error() {
    assert!(matches!(
        run(0, b"not json".to_vec(), Vec::new()),
        Err(CollectorError::Parse(_))
    ));
}
