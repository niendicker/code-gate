//! Parser and normalization tests over real `cargo llvm-cov --json
//! --summary-only` output (cargo-llvm-cov 0.9.1, LLVM 22), paths sanitized.

use code_gate_collector_llvm_cov::{Counter, CoverageTotals, parse_coverage_output};
use code_gate_core::{CollectorOutput, MetricUnit};

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path).unwrap()
}

fn metric(output: &CollectorOutput, id: &str) -> Option<f64> {
    output
        .metrics
        .iter()
        .find(|metric| metric.id == id)
        .map(|metric| {
            assert_eq!(metric.unit, MetricUnit::Percent);
            assert_eq!(metric.scope, "project");
            assert_eq!(metric.source, "cargo-llvm-cov");
            metric.value
        })
}

fn approx(actual: Option<f64>, expected: f64) {
    let actual = actual.expect("metric missing");
    assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
}

#[test]
fn reads_project_totals() {
    let totals = parse_coverage_output(&fixture("coverage-partial.json")).unwrap();

    assert_eq!(
        totals,
        CoverageTotals {
            lines: Counter {
                count: 9,
                covered: 6
            },
            functions: Counter {
                count: 3,
                covered: 2
            },
            regions: Counter {
                count: 12,
                covered: 8
            },
            branches: Counter {
                count: 0,
                covered: 0
            },
        }
    );
}

#[test]
fn emits_percentage_metrics() {
    let output = parse_coverage_output(&fixture("coverage-multi-file.json"))
        .unwrap()
        .to_collector_output();

    approx(metric(&output, "test.coverage.lines"), 80.0);
    approx(metric(&output, "test.coverage.functions"), 80.0);
    approx(
        metric(&output, "test.coverage.regions"),
        15.0 * 100.0 / 19.0,
    );
    assert!(output.findings.is_empty());
}

#[test]
fn omits_kinds_without_instrumentation() {
    // Branch coverage needs nightly instrumentation; stable reports 0 of 0.
    let output = parse_coverage_output(&fixture("coverage-partial.json"))
        .unwrap()
        .to_collector_output();

    assert_eq!(metric(&output, "test.coverage.branches"), None);
    assert_eq!(output.metrics.len(), 3);
}

#[test]
fn rejects_invalid_json() {
    assert!(parse_coverage_output("running 1 test").is_err());
    assert!(parse_coverage_output("").is_err());
}

#[test]
fn rejects_other_export_types() {
    let input = fixture("coverage-partial.json")
        .replace("llvm.coverage.json.export", "llvm.something.else");
    let error = parse_coverage_output(&input).unwrap_err();

    assert!(error.0.contains("llvm.something.else"), "{error}");
}

#[test]
fn requires_exactly_one_export() {
    let error = parse_coverage_output(r#"{"data":[]}"#).unwrap_err();

    assert!(error.0.contains("found 0"), "{error}");
}
