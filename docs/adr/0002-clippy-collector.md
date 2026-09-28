# ADR-0002: Clippy collector metrics, findings and collector configuration

- Status: Accepted
- Date: 2026-09-27

## Context

The Clippy collector is the first real analyzer integration. It introduces
the first canonical metric names produced by a collector, the first real
findings, and a way to choose which collectors `code-gate collect` runs.

## Decision

### Execution

The collector runs, in the directory containing `quality.toml`:

```text
cargo clippy --workspace --all-targets --all-features --message-format=json
```

`--all-features` is the default feature selection; see
[ADR-0005](0005-cargo-feature-selection.md).

It consumes only Cargo's JSON messages on stdout, never the human-readable
output.

### Metrics

All metrics are `count`s with scope `project`:

- `code.clippy.total`, `code.clippy.warnings`, `code.clippy.errors` —
  diagnostics whose lint code starts with `clippy::` (source `clippy`);
- `code.compiler.warnings`, `code.compiler.errors` — all other rustc
  diagnostics seen during the run (source `rustc`).

All five are always emitted, including zeros, so that "no warnings" is
distinguishable from "not measured".

`project` is used instead of a new `workspace` scope because it is already the
aggregate scope in the metrics model and the example policies.

### Findings

One finding per unique diagnostic of level `warning` or `error`:

```text
rule_id   clippy::<lint> | rustc::<lint-or-code> | rustc::warning|error (uncoded)
severity  warning | error
source    clippy | rustc
scope     crate:<package-name>
location  primary span path/line/column, when present
```

`note`, `help` and `failure-note` diagnostics, rustc summary lines, rendered
output and code explanations are not retained.

### Error semantics

Warnings, denied lints and compile errors are evidence. The collector fails
only when Cargo did not finish a build (missing Cargo/Clippy, bad manifest),
when the build failed without any diagnostic (e.g. a failing build script),
or when output cannot be parsed.

### Configuration

`quality.toml` gains a `[collectors]` table with one sub-table per known
collector (`clippy`, `llvm_cov`, `audit`, `bca`), each with `enabled`
(default `false`). Unknown collector names are rejected. Enabling a
scaffolded collector is a hard error rather than a silent skip.

## Consequences

- Gates can now enforce `code.clippy.*` and `code.compiler.*` on real runs.
- When compilation fails, Clippy lints for the affected crates do not run, so
  `code.clippy.*` may undercount; policies should also gate
  `code.compiler.errors`.
- Per-crate metrics are not produced yet; crate identity is only preserved
  in findings.
