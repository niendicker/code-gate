# ADR-0004: cargo-audit collector

- Status: Accepted
- Date: 2026-09-27

## Context

`security.vulnerabilities.critical` has been in the example policies since
the first commit, but no collector produced it, so the gate could only fail
as "missing". cargo-audit checks `Cargo.lock` against the RustSec advisory
database and is the de facto open-source tool for this in Rust.

RustSec advisories carry a CVSS vector (v3.0, v3.1 or v4.0) but no severity
label, and some advisories carry no vector at all.

## Decision

### Execution

In the directory containing `quality.toml`:

```text
cargo audit --json
```

stdout is the RustSec `Report` document. Only `vulnerabilities.list` and
`warnings` are read; other fields are ignored. Project configuration in
`.cargo/audit.toml` (ignored advisories, target filters) is honored by
cargo-audit itself, so ignored advisories never reach Code Gate.

### Metrics

Counts with scope `project` and source `cargo-audit`, always emitted (a clean
audit is `0`, not missing):

- `security.vulnerabilities.total`
- `security.vulnerabilities.critical` — CVSS 9.0–10.0
- `security.vulnerabilities.high` — CVSS 7.0–8.9
- `security.vulnerabilities.medium` — CVSS 4.0–6.9
- `security.vulnerabilities.low` — CVSS 0.0–3.9
- `security.vulnerabilities.unknown` — no CVSS vector, or one that cannot be
  scored
- `dependency.unmaintained`
- `dependency.unsound`
- `dependency.yanked`

Severity is the CVSS qualitative rating scale, computed from the vector with
RustSec's own `cvss` crate. CVSS 0.0 ("none") counts as `low`: a published
vulnerability is never reported as harmless.

The five severity buckets add up to `total`. A policy gating only on
`critical` lets unscored advisories through; gate on `total` or on `unknown`
as well to close that gap.

`dependency.*` uses the `dependency` domain rather than a tool-specific name:
unmaintained, unsound and yanked are properties of a dependency, whichever
tool reports them. `notice` warnings produce findings but no metric.

### Findings

One finding per vulnerable package version and per warning, located at
`Cargo.lock`, with the advisory id as `rule_id` (`yanked` for yanked crates,
which have no advisory). Vulnerability severities: critical → `critical`,
high and unknown → `error`, medium and low → `warning`. Warnings → `warning`.

### Error semantics

cargo-audit exits `1` when it *finds* vulnerabilities, so the exit status is
not used to decide success:

- A parsable report on stdout → metrics, whatever the exit code.
- `cargo audit` not installed → `ToolMissing`, with installation hints.
- Non-zero exit without a report (advisory database fetch failed, no
  lockfile, …) → `Execution`, quoting stderr.
- Exit `0` with unreadable output → `Parse`.

## Consequences

- `security.vulnerabilities.*` gates work on real runs.
- `collect` needs network access to fetch the advisory database, unless it
  is already cached and cargo-audit is configured not to fetch.
- `dependency.yanked` depends on cargo-audit's yanked check, which queries the
  crates.io index.
- Per-crate attribution (which workspace crate pulls in the vulnerable
  dependency) is not produced yet.
