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
└── collectors/
    ├── code-gate-collector-clippy/
    ├── code-gate-collector-bca/
    ├── code-gate-collector-llvm-cov/
    └── code-gate-collector-audit/
```

## Current foundation

The repository currently contains:

- canonical metric and finding model;
- quality-gate evaluator;
- `quality.toml` parser;
- JSON report loader/writer;
- CLI with `init`, `check`, and `version`;
- collector interface;
- initial collector crates as extension points;
- example safety-critical policy;
- JSON schema draft;
- architecture and roadmap documentation.

The collectors are intentionally scaffolded first. The core does not parse tool-specific output.

## Try the gate engine

Using the included example:

```bash
cargo run -p code-gate-cli -- check \
  --config examples/safety-critical/quality.toml \
  --report examples/safety-critical/quality-report.json
```

Initialize a policy in another Rust project:

```bash
cargo run -p code-gate-cli -- init
```

## Example policy

```toml
[project]
name = "my-rust-project"
default_criticality = "B"

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
code-gate check --config quality.toml --report quality-report.json
code-gate version
```

Exit codes:

```text
0  gate passed
1  gate failed
2  configuration/report/runtime error
```

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
