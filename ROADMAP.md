# Code Gate Roadmap

The roadmap is intentionally incremental. Code Gate should become useful before it becomes ambitious.

## v0.1 — Foundation

**Goal:** establish a stable domain model and a usable quality-gate engine.

- [x] Rust workspace
- [x] canonical numeric metric model
- [x] findings model
- [x] gate rules and evaluator
- [x] `quality.toml` policy
- [x] JSON report model
- [x] CLI foundation
- [x] `init` command
- [x] `check` command over canonical reports
- [x] collector contract
- [x] collector crate scaffolds
- [x] wire Clippy collector
- [ ] wire `bca` collector
- [ ] wire `cargo-llvm-cov` collector
- [ ] wire `cargo-audit` collector
- [ ] integration tests with real tool outputs
- [ ] lock canonical metric naming conventions

### v0.1 exit criteria

A Rust project can run open-source analyzers, normalize their output through Code Gate, and receive deterministic `PASS` / `FAIL` results from a versioned policy.

---

## v0.2 — Risk-aware quality

**Goal:** make policies reflect component risk instead of treating every crate equally.

- [ ] component inventory
- [ ] criticality classes A/B/C/D
- [ ] policy inheritance
- [ ] per-component gates
- [ ] `cargo-deny` collector
- [ ] `cargo-geiger` collector
- [ ] dependency graph metrics
- [ ] `unsafe` scope policy
- [ ] missing-metric policy improvements
- [ ] structured finding locations
- [ ] SARIF output

Suggested criticality model:

| Class | Intended use | Example |
|---|---|---|
| A | control/safety/core | control engine, supervisor |
| B | operational | protocol and field adapters |
| C | supporting | tools and services |
| D | development/test | simulators, lab utilities |

---

## v0.3 — Regression gates

**Goal:** evaluate change, not only absolute thresholds.

- [ ] baseline report support
- [ ] compare current branch against baseline
- [ ] percentage and absolute delta rules
- [ ] "must not regress" gate
- [ ] coverage regression
- [ ] complexity regression
- [ ] new `unsafe` detection
- [ ] new vulnerability detection
- [ ] code churn metrics
- [ ] changed-component scope

Example:

```text
coverage.main     = 91.4%
coverage.current  = 89.7%
delta             = -1.7%

policy max drop   = -1.0%
FAIL
```

---

## v0.4 — Test strength

**Goal:** measure whether tests actually detect behavioral changes.

- [ ] `cargo-mutants` collector
- [ ] mutation score
- [ ] surviving mutant findings
- [ ] critical-module mutation gates
- [ ] flaky-test metric
- [ ] ignored-test metric
- [ ] test duration trends

Mutation testing should normally be scheduled or release-oriented rather than required for every edit.

---

## v0.5 — Historical quality

**Goal:** make trends first-class.

- [ ] local history store
- [ ] commit metadata
- [ ] metric time series
- [ ] trend calculations
- [ ] moving averages
- [ ] regression alerts
- [ ] export format for Prometheus
- [ ] lightweight HTML report

The history layer must remain optional. Code Gate must stay useful as a stateless CLI.

---

## v0.6 — Runtime evidence

**Goal:** combine static quality with operational evidence.

Candidate metrics:

- deadline misses
- execution latency
- jitter
- watchdog resets
- queue depth
- reconnect duration
- message loss
- CPU saturation
- memory high-water mark

Runtime data should enter through the same canonical metric model instead of creating a parallel quality system.

---

## v0.7 — Architecture rules

**Goal:** make architectural constraints testable.

- [ ] crate dependency boundaries
- [ ] forbidden dependency directions
- [ ] dependency cycles
- [ ] public API size
- [ ] coupling indicators
- [ ] dependency depth
- [ ] architecture findings
- [ ] architecture baseline

---

## v1.0 — Stable quality platform

**Goal:** stable contracts suitable for broad adoption.

- [ ] stable `quality.toml` schema
- [ ] stable canonical metric namespace
- [ ] stable JSON report schema
- [ ] stable collector API
- [ ] documented exit-code contract
- [ ] reproducible releases
- [ ] signed release artifacts
- [ ] security policy
- [ ] compatibility policy
- [ ] migration documentation
- [ ] CI recipes for multiple open CI systems

## Explicit non-goals

Code Gate should not become:

- a compiler;
- a source-code parser for every language;
- a replacement for specialized analyzers;
- an IDE;
- a proprietary hosted quality platform;
- a single magical quality score that hides failed invariants.
