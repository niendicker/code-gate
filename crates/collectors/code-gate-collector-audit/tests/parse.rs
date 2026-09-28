//! Parser and normalization tests over `cargo audit --json` reports.
//!
//! `audit-clean.json` and `audit-vulnerable.json` are real cargo-audit 0.22.2
//! output (advisory database of 2026-09-25). `audit-unscored.json` is
//! synthetic, in the same format, for cases that cannot be reproduced on
//! demand: an advisory without CVSS, an unsound and a yanked crate.

use code_gate_collector_audit::{AuditReport, VulnerabilitySeverity, parse_audit_output};
use code_gate_core::{CollectorOutput, MetricUnit, Severity};

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path).unwrap()
}

fn parse(name: &str) -> AuditReport {
    parse_audit_output(&fixture(name)).unwrap()
}

fn metric(output: &CollectorOutput, id: &str) -> f64 {
    let metric = output
        .metrics
        .iter()
        .find(|metric| metric.id == id)
        .unwrap_or_else(|| panic!("metric {id} missing"));
    assert_eq!(metric.unit, MetricUnit::Count);
    assert_eq!(metric.scope, "project");
    assert_eq!(metric.source, "cargo-audit");
    metric.value
}

#[test]
fn clean_audit_reports_zero_counts() {
    let output = parse("audit-clean.json").to_collector_output();

    assert_eq!(output.metrics.len(), 9);
    assert!(output.metrics.iter().all(|metric| metric.value == 0.0));
    assert!(output.findings.is_empty());
}

#[test]
fn reads_vulnerabilities_with_cvss_severity() {
    let report = parse("audit-vulnerable.json");

    let summary: Vec<_> = report
        .vulnerabilities
        .iter()
        .map(|v| (v.advisory_id.as_str(), v.package.as_str(), v.severity))
        .collect();
    assert_eq!(
        summary,
        [
            ("RUSTSEC-2023-0071", "rsa", VulnerabilitySeverity::Medium),
            ("RUSTSEC-2020-0071", "time", VulnerabilitySeverity::Medium),
        ]
    );
    assert_eq!(report.vulnerabilities[0].cvss_score, Some(5.9));
    assert_eq!(report.vulnerabilities[1].patched, [">=0.2.23"]);
}

#[test]
fn emits_vulnerability_and_dependency_counts() {
    let output = parse("audit-vulnerable.json").to_collector_output();

    assert_eq!(metric(&output, "security.vulnerabilities.total"), 2.0);
    assert_eq!(metric(&output, "security.vulnerabilities.critical"), 0.0);
    assert_eq!(metric(&output, "security.vulnerabilities.high"), 0.0);
    assert_eq!(metric(&output, "security.vulnerabilities.medium"), 2.0);
    assert_eq!(metric(&output, "security.vulnerabilities.low"), 0.0);
    assert_eq!(metric(&output, "security.vulnerabilities.unknown"), 0.0);
    assert_eq!(metric(&output, "dependency.unmaintained"), 1.0);
    assert_eq!(metric(&output, "dependency.unsound"), 0.0);
    assert_eq!(metric(&output, "dependency.yanked"), 0.0);
}

#[test]
fn findings_explain_each_hit() {
    let output = parse("audit-vulnerable.json").to_collector_output();

    let findings: Vec<_> = output
        .findings
        .iter()
        .map(|f| (f.rule_id.as_str(), f.severity, f.message.as_str()))
        .collect();
    assert_eq!(
        findings,
        [
            (
                "RUSTSEC-2023-0071",
                Severity::Warning,
                "rsa 0.9.6: Marvin Attack: potential key recovery through timing sidechannels \
                 (CVSS 5.9 medium; no patched version available)"
            ),
            (
                "RUSTSEC-2020-0071",
                Severity::Warning,
                "time 0.1.45: Potential segfault in the time crate \
                 (CVSS 6.2 medium; upgrade to >=0.2.23)"
            ),
            (
                "RUSTSEC-2021-0139",
                Severity::Warning,
                "ansi_term 0.12.1 is unmaintained: ansi_term is Unmaintained"
            ),
        ]
    );

    for finding in &output.findings {
        assert_eq!(finding.source, "cargo-audit");
        assert_eq!(finding.location.as_ref().unwrap().path, "Cargo.lock");
    }
}

#[test]
fn unscored_vulnerability_is_unknown_and_an_error() {
    let output = parse("audit-unscored.json").to_collector_output();

    assert_eq!(metric(&output, "security.vulnerabilities.total"), 1.0);
    assert_eq!(metric(&output, "security.vulnerabilities.unknown"), 1.0);
    assert_eq!(metric(&output, "dependency.unsound"), 1.0);
    assert_eq!(metric(&output, "dependency.yanked"), 1.0);

    let finding = &output.findings[0];
    assert_eq!(finding.severity, Severity::Error);
    assert_eq!(
        finding.message,
        "example-crate 1.3.1: Out-of-bounds read in parser \
         (no CVSS score; upgrade to >=1.4.2 or ^1.3.9)"
    );
}

#[test]
fn rejects_documents_that_are_not_audit_reports() {
    assert!(parse_audit_output("{}").is_err());
    assert!(parse_audit_output(r#"{"vulnerabilities": {}}"#).is_err());
    assert!(parse_audit_output("not json").is_err());
}

#[test]
fn yanked_crate_has_no_advisory() {
    let output = parse("audit-unscored.json").to_collector_output();

    let yanked = output
        .findings
        .iter()
        .find(|finding| finding.rule_id == "yanked")
        .expect("yanked finding");
    assert_eq!(yanked.severity, Severity::Warning);
    assert_eq!(yanked.message, "futures-util 0.3.29 is yanked");
}
