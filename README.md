# Code Gate

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
  --all-features --message-format=json` and produces:

  | Metric | Source | Scope |
  |---|---|---|
  | `code.clippy.total` | clippy | project |
  | `code.clippy.warnings` | clippy | project |
  | `code.clippy.errors` | clippy | project |
  | `code.compiler.warnings` | rustc | project |
  | `code.compiler.errors` | rustc | project |

  plus one finding per (deduplicated) diagnostic, with rule, severity,
  message, crate scope and source location when available.

Scaffolded only (enabling them makes `collect` fail with exit code `2`):

- `llvm_cov` (cargo-llvm-cov);
- `audit` (cargo-audit);
- `bca` (big-code-analysis).

Collector configuration:

```toml
[collectors.clippy]
enabled = true

[collectors.llvm_cov]
enabled = false
```

Collectors are disabled unless explicitly enabled; unknown collector names are
rejected.

## Example policy

```toml
[project]
name = "my-rust-project"
default_criticality = "B"

[collectors.clippy]
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

A tool that runs and reports problems (Clippy warnings, compile errors) is not
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
