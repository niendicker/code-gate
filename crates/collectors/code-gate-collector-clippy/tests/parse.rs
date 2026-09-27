//! Parser and normalization tests over real `cargo clippy --message-format=json`
//! output (clippy 0.1.98), captured from a sample crate with paths sanitized.

use code_gate_collector_clippy::{Level, Origin, parse_clippy_output};
use code_gate_core::{CollectorOutput, Finding, Location, MetricUnit, Severity};

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap()
}

fn collect(name: &str) -> CollectorOutput {
    parse_clippy_output(&fixture(name))
        .unwrap()
        .to_collector_output()
}

fn metric(output: &CollectorOutput, id: &str) -> f64 {
    let metric = output
        .metrics
        .iter()
        .find(|metric| metric.id == id)
        .unwrap_or_else(|| panic!("metric {id} missing"));
    assert_eq!(metric.unit, MetricUnit::Count);
    assert_eq!(metric.scope, "project");
    metric.value
}

/// (clippy.total, clippy.warnings, clippy.errors, compiler.warnings, compiler.errors)
fn counts(output: &CollectorOutput) -> (f64, f64, f64, f64, f64) {
    (
        metric(output, "code.clippy.total"),
        metric(output, "code.clippy.warnings"),
        metric(output, "code.clippy.errors"),
        metric(output, "code.compiler.warnings"),
        metric(output, "code.compiler.errors"),
    )
}

#[test]
fn clean_run_emits_zeroed_metrics_and_no_findings() {
    let output = collect("clippy-clean.jsonl");

    assert_eq!(counts(&output), (0.0, 0.0, 0.0, 0.0, 0.0));
    assert_eq!(output.metrics.len(), 5);
    assert!(output.findings.is_empty());
}

#[test]
fn metrics_declare_their_source() {
    let output = collect("clippy-clean.jsonl");
    let source = |id: &str| {
        output
            .metrics
            .iter()
            .find(|metric| metric.id == id)
            .map(|metric| metric.source.as_str())
    };

    assert_eq!(source("code.clippy.warnings"), Some("clippy"));
    assert_eq!(source("code.compiler.warnings"), Some("rustc"));
}

#[test]
fn single_warning_becomes_one_located_finding() {
    let output = collect("clippy-warning.jsonl");

    // Cargo emits the diagnostic twice (lib and lib test); it is counted once.
    assert_eq!(counts(&output), (1.0, 1.0, 0.0, 0.0, 0.0));
    assert_eq!(
        output.findings,
        vec![Finding {
            rule_id: "clippy::len_zero".into(),
            severity: Severity::Warning,
            message: "length comparison to zero".into(),
            source: "clippy".into(),
            scope: "crate:sample".into(),
            location: Some(Location {
                path: "src/lib.rs".into(),
                line: Some(2),
                column: Some(5),
            }),
        }]
    );
}

#[test]
fn multiple_warnings_are_split_between_clippy_and_compiler() {
    let output = collect("clippy-warnings.jsonl");

    assert_eq!(counts(&output), (2.0, 2.0, 0.0, 1.0, 0.0));

    let rules: Vec<(&str, &str)> = output
        .findings
        .iter()
        .map(|finding| (finding.rule_id.as_str(), finding.source.as_str()))
        .collect();
    assert_eq!(
        rules,
        vec![
            ("rustc::unused_variables", "rustc"),
            ("clippy::ptr_arg", "clippy"),
            ("clippy::len_zero", "clippy"),
        ]
    );
}

#[test]
fn denied_clippy_lint_counts_as_clippy_error() {
    let run = parse_clippy_output(&fixture("clippy-error.jsonl")).unwrap();
    let output = run.to_collector_output();

    assert_eq!(run.build_success, Some(false));
    assert_eq!(counts(&output), (1.0, 0.0, 1.0, 0.0, 0.0));
    assert_eq!(output.findings[0].severity, Severity::Error);
}

#[test]
fn compiler_error_is_not_a_clippy_metric() {
    let run = parse_clippy_output(&fixture("compiler-error.jsonl")).unwrap();
    let output = run.to_collector_output();

    assert_eq!(run.build_success, Some(false));
    assert_eq!(counts(&output), (0.0, 0.0, 0.0, 0.0, 1.0));
    // The trailing `failure-note` is context, not a finding.
    assert_eq!(output.findings.len(), 1);
    assert_eq!(output.findings[0].rule_id, "rustc::E0308");
    assert_eq!(
        output.findings[0].location,
        Some(Location {
            path: "src/lib.rs".into(),
            line: Some(1),
            column: Some(34),
        })
    );
}

#[test]
fn diagnostic_without_location_is_kept_without_location() {
    let output = collect("no-location.jsonl");

    assert_eq!(counts(&output), (0.0, 0.0, 0.0, 1.0, 0.0));
    assert_eq!(output.findings.len(), 1);
    assert_eq!(output.findings[0].rule_id, "rustc::E0602");
    assert_eq!(output.findings[0].scope, "crate:sample");
    assert_eq!(output.findings[0].location, None);
}

#[test]
fn mixed_cargo_messages_only_yield_diagnostics() {
    // Contains build-script, compiler-artifact and build-finished messages.
    let run = parse_clippy_output(&fixture("mixed-cargo-messages.jsonl")).unwrap();
    let output = run.to_collector_output();

    assert_eq!(run.build_success, Some(true));
    assert_eq!(run.count(Origin::Clippy, Level::Warning), 2);
    assert_eq!(counts(&output), (2.0, 2.0, 0.0, 0.0, 0.0));
    assert!(
        output
            .findings
            .iter()
            .all(|finding| finding.location.as_ref().unwrap().path == "src/bin/tool.rs")
    );
}

#[test]
fn invalid_json_reports_the_offending_line() {
    let input = format!("{}\nnot json\n", fixture("clippy-clean.jsonl").trim_end());
    let error = parse_clippy_output(&input).unwrap_err();

    assert_eq!(error.line, 4);
}

#[test]
fn json_without_cargo_reason_is_rejected() {
    let error = parse_clippy_output("{\"hello\":\"world\"}").unwrap_err();

    assert_eq!(error.line, 1);
    assert!(error.reason.contains("reason"), "{}", error.reason);
}

#[test]
fn empty_output_has_no_build_result() {
    let run = parse_clippy_output("").unwrap();

    assert_eq!(run.build_success, None);
    assert!(run.diagnostics.is_empty());
}
