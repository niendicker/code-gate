# Code Gate

[![CI](https://github.com/niendicker/code-gate/actions/workflows/ci.yml/badge.svg)](https://github.com/niendicker/code-gate/actions/workflows/ci.yml)

**Code Gate** is an open-source, tool-agnostic quality gate for Rust projects.

It does **not** try to replace Clippy, LLVM coverage, `cargo-audit`, `bca`, mutation testing, or other analyzers. Instead, Code Gate provides the missing layer between them:

> **collect → normalize → evaluate → report**

The long-term goal is to make software quality policies explicit, versioned, reproducible, and independent of any single analysis vendor.

## Principles

1. **Open tooling first** — integrations target open-source analyzers.
2. **Canonical metrics** — external tools are adapters, not the domain model.
3. **Policy as code** — quality rules live in `quality.toml`.
4. **Gates before scores** — correctness and safety invariants cannot be averaged away.
5. **Regression-aware** — a project that is not perfect must at least stop getting worse.
6. **Component criticality** — stricter rules for higher-risk components.
7. **Portable CI** — the engine must work independently of a specific CI vendor.
8. **Auditable output** — machine-readable reports must explain every failed gate.

## Architecture

```text
 External OSS analyzers
 ┌─────────┬───────────┬──────────┬────────────┐
 │ Clippy  │    bca    │ llvm-cov │ cargo-audit│ ...
 └────┬────┴─────┬─────┴────┬─────┴─────┬──────┘
      │          │          │           │
      ▼          ▼          ▼           ▼
                  Collectors
                      │
                      ▼
              Canonical Metrics
                      │
         ┌────────────┴────────────┐
         ▼                         ▼
     Findings                  quality.toml
         │                         │
         └────────────┬────────────┘
                      ▼
                 Gate Engine
                      │
             ┌────────┴────────┐
             ▼                 ▼
          Console            JSON
          PASS/FAIL        Audit trail
```

## Workspace

```text
crates/
├── code-gate-core/        # domain model + gate evaluator
├── code-gate-config/      # quality.toml parsing
├── code-gate-report/      # canonical JSON reports
├── code-gate-cli/         # CLI
├── code-gate-process/     # small, testable external-tool runner
└── collectors/
    ├── code-gate-collector-clippy/
    ├── code-gate-collector-bca/
    ├── code-gate-collector-llvm-cov/
    └── code-gate-collector-audit/
```

## Quick start

```bash
cargo build --release

./target/release/code-gate collect
./target/release/code-gate check \
  --config quality.toml \
  --report quality.json
```

`collect` reads `quality.toml` from the current directory (or `--config`),
analyzes the Cargo project in that file's directory with every enabled
collector, and writes the canonical report to `quality.json` (or `--output`).
It does not judge the result: warnings found by a tool are evidence, and
`collect` still exits `0`. `check` then evaluates the report against the gates.
The report also records, under `collectors`, the exact command each collector
ran and the Cargo features it built with, so the evidence can be reproduced
from the report alone.

This repository dogfoods itself with the root [`quality.toml`](quality.toml).

To start a policy in another Rust project:

```bash
code-gate init      # writes quality.toml from quality.example.toml
```

The static example policy still works without any collector:

```bash
cargo run -p code-gate-cli -- check \
  --config examples/safety-critical/quality.toml \
  --report examples/safety-critical/quality-report.json
```

## Status (v0.1)

Implemented:

- canonical metric and finding model;
- quality-gate evaluator;
- `quality.toml` parser, including `[collectors.*]` toggles;
- JSON report loader/writer;
- CLI: `init`, `collect`, `check`, `version`;
- **Clippy collector** — runs `cargo clippy --workspace --all-targets
  --all-features --message-format=json` (feature selection configurable)
  and produces:

  | Metric | Source | Scope |
  |---|---|---|
  | `code.clippy.total` | clippy | project |
  | `code.clippy.warnings` | clippy | project |
  | `code.clippy.errors` | clippy | project |
  | `code.compiler.warnings` | rustc | project |
  | `code.compiler.errors` | rustc | project |

  plus one finding per (deduplicated) diagnostic, with rule, severity,
  message, crate scope and source location when available.
- **cargo-llvm-cov collector** — runs `cargo llvm-cov --workspace
  --all-features --json --summary-only` (feature selection configurable)
  and produces project-wide percentages:

  | Metric | Source | Scope |
  |---|---|---|
  | `test.coverage.lines` | cargo-llvm-cov | project |
  | `test.coverage.functions` | cargo-llvm-cov | project |
  | `test.coverage.regions` | cargo-llvm-cov | project |
  | `test.coverage.branches` | cargo-llvm-cov | project (only when branch instrumentation is enabled) |

  Requires [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov) and
  the LLVM tools: `rustup component add llvm-tools-preview`, or, on a
  toolchain without rustup (e.g. distribution packages), point
  `LLVM_COV` and `LLVM_PROFDATA` at an LLVM matching `rustc -vV`:

  ```bash
  export LLVM_COV=/usr/bin/llvm-cov LLVM_PROFDATA=/usr/bin/llvm-profdata
  ```

  A failing test suite makes `collect` exit `2`: coverage of a failing suite
  is not measured.

- **cargo-audit collector** — runs `cargo audit --json` against `Cargo.lock`
  and produces counts from the RustSec advisory database:

  | Metric | Source | Scope |
  |---|---|---|
  | `security.vulnerabilities.total` | cargo-audit | project |
  | `security.vulnerabilities.{critical,high,medium,low}` | cargo-audit | project |
  | `security.vulnerabilities.unknown` | cargo-audit | project |
  | `dependency.unmaintained` | cargo-audit | project |
  | `dependency.unsound` | cargo-audit | project |
  | `dependency.yanked` | cargo-audit | project |

  plus one finding per advisory hit. Severity comes from the advisory's CVSS
  vector; advisories without one are counted as `unknown`, so gate on `total`
  (or `unknown`) as well as `critical`. Finding vulnerabilities is evidence:
  `collect` still exits `0`. Requires
  [`cargo-audit`](https://github.com/rustsec/rustsec/tree/main/cargo-audit)
  (`cargo install cargo-audit --locked` or your distribution's package) and
  network access to fetch the advisory database.

Scaffolded only (enabling it makes `collect` fail with exit code `2`):

- `bca` (big-code-analysis).

Collector configuration:

```toml
[collectors.clippy]
enabled = true

[collectors.llvm_cov]
enabled = false

[collectors.audit]
enabled = false
```

Collectors are disabled unless explicitly enabled; unknown collector names are
rejected.

`clippy` and `llvm_cov` build the workspace with `--all-features` by default.
For projects whose features cannot all be enabled together, choose the
features per collector:

```toml
[collectors.llvm_cov]
enabled = true
features = "default"          # or "all", "none", or ["std", "serde"]
# default_features = false    # only together with a list
```

See [ADR-0005](docs/adr/0005-cargo-feature-selection.md).

## Example policy

```toml
[project]
name = "my-rust-project"
default_criticality = "B"

[collectors.clippy]
enabled = true

[collectors.audit]
enabled = true

[[gates]]
id = "no-clippy-warnings"
metric = "code.clippy.warnings"
scope = "project"
operator = "lte"
threshold = 0
severity = "error"
missing = "fail"

[[gates]]
id = "no-critical-vulnerabilities"
metric = "security.vulnerabilities.critical"
scope = "project"
operator = "lte"
threshold = 0
severity = "critical"
missing = "fail"

[[gates]]
id = "core-line-coverage"
metric = "test.coverage.lines"
scope = "core"
operator = "gte"
threshold = 95
severity = "error"
missing = "fail"
```

## CLI contract

```text
code-gate init
code-gate collect [--config quality.toml] [--output quality.json]
code-gate check   [--config quality.toml] [--report quality.json]
code-gate version
```

Exit codes:

```text
0  collect: report written / check: all enforced gates passed
1  check: at least one quality gate failed
2  configuration, report, tooling or runtime error
```

A tool that runs and reports problems (Clippy warnings, compile errors,
vulnerable dependencies) is not
an error: `collect` records the evidence and exits `0`. Exit code `2` from
`collect` means the evidence could not be gathered — tool missing, tool failed
without diagnostics, unparsable output, or a scaffolded collector enabled — and
no report is written.

## Planned analyzer integrations

- Clippy
- big-code-analysis (`bca`)
- cargo-llvm-cov
- cargo-audit / RustSec
- cargo-deny
- cargo-geiger
- cargo-mutants

See [ROADMAP.md](ROADMAP.md) for sequencing.

## License

Dual licensed under **MIT OR Apache-2.0**.
