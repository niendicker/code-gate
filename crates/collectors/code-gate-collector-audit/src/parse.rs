//! Parsing of `cargo audit --json` output.
//!
//! The input is the RustSec `Report` document. This module is pure: it never
//! runs processes.

use std::collections::BTreeMap;

use code_gate_core::{CollectorOutput, Finding, Location, Metric, MetricUnit, Severity};
use serde::Deserialize;

/// Aggregate scope of the metrics produced by this collector.
pub const PROJECT_SCOPE: &str = "project";

const SOURCE: &str = "cargo-audit";
const LOCKFILE: &str = "Cargo.lock";

/// Severity of a vulnerability, from its advisory's CVSS vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VulnerabilitySeverity {
    Critical,
    High,
    Medium,
    /// CVSS 0.1–3.9, and also 0.0 ("none"): a published vulnerability is
    /// never counted as harmless.
    Low,
    /// The advisory has no CVSS vector, or one that cannot be scored.
    Unknown,
}

impl VulnerabilitySeverity {
    fn from_cvss(vector: Option<&str>) -> (Self, Option<f64>) {
        let Some(cvss) = vector.and_then(|vector| vector.parse::<cvss::Cvss>().ok()) else {
            return (Self::Unknown, None);
        };

        let severity = match cvss.severity() {
            cvss::Severity::Critical => Self::Critical,
            cvss::Severity::High => Self::High,
            cvss::Severity::Medium => Self::Medium,
            cvss::Severity::Low | cvss::Severity::None => Self::Low,
        };
        (severity, Some(cvss.score()))
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Unknown => "unknown",
        }
    }

    fn finding_severity(self) -> Severity {
        match self {
            Self::Critical => Severity::Critical,
            // Unscored advisories are treated as serious until triaged.
            Self::High | Self::Unknown => Severity::Error,
            Self::Medium | Self::Low => Severity::Warning,
        }
    }
}

/// A vulnerable package version in the lockfile.
#[derive(Debug, Clone, PartialEq)]
pub struct Vulnerability {
    pub advisory_id: String,
    pub package: String,
    pub version: String,
    pub title: String,
    pub severity: VulnerabilitySeverity,
    pub cvss_score: Option<f64>,
    pub patched: Vec<String>,
}

/// A non-vulnerability warning: unmaintained, unsound, yanked or notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    pub kind: String,
    pub package: String,
    pub version: String,
    /// `None` for yanked crates, which have no advisory.
    pub advisory_id: Option<String>,
    pub title: Option<String>,
}

/// Everything Code Gate keeps from a cargo-audit report.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AuditReport {
    pub vulnerabilities: Vec<Vulnerability>,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

/// Parses a `cargo audit --json` report.
pub fn parse_audit_output(input: &str) -> Result<AuditReport, ParseError> {
    let report: RawReport =
        serde_json::from_str(input).map_err(|error| ParseError(error.to_string()))?;

    let vulnerabilities = report
        .vulnerabilities
        .list
        .into_iter()
        .map(|raw| {
            let (severity, cvss_score) =
                VulnerabilitySeverity::from_cvss(raw.advisory.cvss.as_deref());
            Vulnerability {
                advisory_id: raw.advisory.id,
                package: raw.package.name,
                version: raw.package.version,
                title: raw.advisory.title,
                severity,
                cvss_score,
                patched: raw.versions.patched,
            }
        })
        .collect();

    let warnings = report
        .warnings
        .into_iter()
        .flat_map(|(kind, list)| {
            list.into_iter().map(move |raw| Warning {
                kind: kind.clone(),
                package: raw.package.name,
                version: raw.package.version,
                advisory_id: raw.advisory.as_ref().map(|advisory| advisory.id.clone()),
                title: raw.advisory.map(|advisory| advisory.title),
            })
        })
        .collect();

    Ok(AuditReport {
        vulnerabilities,
        warnings,
    })
}

impl AuditReport {
    pub fn vulnerability_count(&self, severity: VulnerabilitySeverity) -> usize {
        self.vulnerabilities
            .iter()
            .filter(|vulnerability| vulnerability.severity == severity)
            .count()
    }

    pub fn warning_count(&self, kind: &str) -> usize {
        self.warnings
            .iter()
            .filter(|warning| warning.kind == kind)
            .count()
    }

    /// Canonical count metrics, always emitted (a clean audit is `0`, not
    /// missing), plus one finding per vulnerability and warning.
    pub fn to_collector_output(&self) -> CollectorOutput {
        use VulnerabilitySeverity::*;

        let mut counts = vec![("security.vulnerabilities.total", self.vulnerabilities.len())];
        counts.extend([Critical, High, Medium, Low, Unknown].map(|severity| {
            (
                match severity {
                    Critical => "security.vulnerabilities.critical",
                    High => "security.vulnerabilities.high",
                    Medium => "security.vulnerabilities.medium",
                    Low => "security.vulnerabilities.low",
                    Unknown => "security.vulnerabilities.unknown",
                },
                self.vulnerability_count(severity),
            )
        }));
        counts.extend([
            (
                "dependency.unmaintained",
                self.warning_count("unmaintained"),
            ),
            ("dependency.unsound", self.warning_count("unsound")),
            ("dependency.yanked", self.warning_count("yanked")),
        ]);

        let metrics = counts
            .into_iter()
            .map(|(id, count)| Metric {
                id: id.to_owned(),
                value: count as f64,
                unit: MetricUnit::Count,
                source: SOURCE.to_owned(),
                scope: PROJECT_SCOPE.to_owned(),
            })
            .collect();

        let findings = self
            .vulnerabilities
            .iter()
            .map(Vulnerability::to_finding)
            .chain(self.warnings.iter().map(Warning::to_finding))
            .collect();

        CollectorOutput { metrics, findings }
    }
}

impl Vulnerability {
    fn to_finding(&self) -> Finding {
        let score = match self.cvss_score {
            Some(score) => format!("CVSS {score:.1} {}", self.severity.as_str()),
            None => "no CVSS score".to_owned(),
        };
        let fix = if self.patched.is_empty() {
            "no patched version available".to_owned()
        } else {
            format!("upgrade to {}", self.patched.join(" or "))
        };

        finding(
            &self.advisory_id,
            self.severity.finding_severity(),
            format!(
                "{} {}: {} ({score}; {fix})",
                self.package, self.version, self.title
            ),
        )
    }
}

impl Warning {
    fn to_finding(&self) -> Finding {
        let rule_id = self.advisory_id.as_deref().unwrap_or(&self.kind);
        let message = match &self.title {
            Some(title) => format!(
                "{} {} is {}: {title}",
                self.package, self.version, self.kind
            ),
            None => format!("{} {} is {}", self.package, self.version, self.kind),
        };

        finding(rule_id, Severity::Warning, message)
    }
}

fn finding(rule_id: &str, severity: Severity, message: String) -> Finding {
    Finding {
        rule_id: rule_id.to_owned(),
        severity,
        message,
        source: SOURCE.to_owned(),
        scope: PROJECT_SCOPE.to_owned(),
        location: Some(Location {
            path: LOCKFILE.to_owned(),
            line: None,
            column: None,
        }),
    }
}

#[derive(Debug, Deserialize)]
struct RawReport {
    vulnerabilities: RawVulnerabilities,
    #[serde(default)]
    warnings: BTreeMap<String, Vec<RawWarning>>,
}

#[derive(Debug, Deserialize)]
struct RawVulnerabilities {
    list: Vec<RawVulnerability>,
}

#[derive(Debug, Deserialize)]
struct RawVulnerability {
    advisory: RawAdvisory,
    versions: RawVersions,
    package: RawPackage,
}

#[derive(Debug, Deserialize)]
struct RawWarning {
    package: RawPackage,
    #[serde(default)]
    advisory: Option<RawAdvisory>,
}

#[derive(Debug, Deserialize)]
struct RawAdvisory {
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    cvss: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawVersions {
    #[serde(default)]
    patched: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawPackage {
    name: String,
    version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_follows_cvss_rating_scale() {
        let rate = |vector| VulnerabilitySeverity::from_cvss(Some(vector)).0;

        assert_eq!(
            rate("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"),
            VulnerabilitySeverity::Critical
        );
        assert_eq!(
            rate("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:H"),
            VulnerabilitySeverity::High
        );
        assert_eq!(
            rate("CVSS:3.1/AV:N/AC:H/PR:N/UI:N/S:U/C:H/I:N/A:N"),
            VulnerabilitySeverity::Medium
        );
        assert_eq!(
            rate("CVSS:3.1/AV:L/AC:H/PR:H/UI:R/S:U/C:L/I:N/A:N"),
            VulnerabilitySeverity::Low
        );
        assert_eq!(
            rate("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:N"),
            VulnerabilitySeverity::Low
        );
    }

    #[test]
    fn missing_or_invalid_cvss_is_unknown() {
        assert_eq!(
            VulnerabilitySeverity::from_cvss(None),
            (VulnerabilitySeverity::Unknown, None)
        );
        assert_eq!(
            VulnerabilitySeverity::from_cvss(Some("CVSS:9.9/AV:N")),
            (VulnerabilitySeverity::Unknown, None)
        );
    }
}
