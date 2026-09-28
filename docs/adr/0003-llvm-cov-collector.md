# ADR-0003: cargo-llvm-cov collector

- Status: Accepted
- Date: 2026-09-27

## Context

`test.coverage.lines` is the most common quality gate and is already used by
the example policies. It is the second real collector, and the first whose
tool emits a single JSON document instead of a message stream.

## Decision

### Execution

In the directory containing `quality.toml`:

```text
cargo llvm-cov --workspace --all-features --json --summary-only
```

`--all-features` is the default feature selection; see
[ADR-0005](0005-cargo-feature-selection.md).

stdout is LLVM's `llvm.coverage.json.export` document; test output goes to
stderr. Only `data[0].totals` is read. Exactly one export is required, and
the `type` field, when present, must be `llvm.coverage.json.export`.

### Metrics

Percentages with scope `project` and source `cargo-llvm-cov`:

- `test.coverage.lines`
- `test.coverage.functions`
- `test.coverage.regions`
- `test.coverage.branches`

Values are computed as `covered / count * 100` rather than copied from LLVM's
`percent` field. A kind with `count = 0` is **omitted**: nothing was
instrumented, so the value is undefined. The gate's `missing` policy then
decides, instead of the collector fabricating 0 % (always fails) or 100 %
(always passes). On stable Rust this applies to branches.

No findings are produced yet.

### Error semantics

- `cargo llvm-cov` not installed, or LLVM tools not found → `ToolMissing`,
  with installation hints (including `LLVM_COV`/`LLVM_PROFDATA` for
  toolchains without rustup).
- Test suite failed → `Execution` ("coverage was not measured").
  cargo-llvm-cov produces no report in that case, and coverage of a failing
  suite is not meaningful evidence. `--ignore-run-fail` is deliberately not
  used, because it would hide test failures from the report entirely.
- Any other non-zero exit → `Execution`; unreadable output → `Parse`.

## Consequences

- Coverage gates now work on real runs.
- `collect` runs the test suite a second time when tests are also run
  elsewhere in CI.
- Test failures stop collection (exit 2) until a test-result collector exists
  that can report them as metrics (`test.*`).
- Per-crate coverage is not produced yet.
