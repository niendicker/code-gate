# Architecture

## Architectural intent

Code Gate separates four responsibilities:

1. **Collection** — invoke or ingest external analysis tools.
2. **Normalization** — translate tool-specific output into canonical metrics/findings.
3. **Evaluation** — apply project policy without knowing which tool produced a metric.
4. **Presentation** — expose deterministic human- and machine-readable results.

The core must never require a specific analyzer.

## Dependency rule

```text
collectors ───────► core
config ───────────► core
report ───────────► core
collectors ───────► process        (external tool runner)
cli ──────────────► core + config + report + collectors

core ─X─► collectors
core ─X─► CLI
core ─X─► external tools
```

`code-gate-core` owns the domain.

## Canonical metric model

Every canonical metric is numeric.

```text
Metric
├── id
├── value: f64
├── unit
├── source
└── scope
```

Examples:

```text
code.clippy.warnings                 0 count
code.complexity.cognitive.max       12 score
test.coverage.lines                 91.4 percent
security.vulnerabilities.critical    0 count
unsafe.blocks                        2 count
runtime.loop.jitter.p99              4.8 ms
```

Textual evidence is represented as a `Finding`, not forced into a metric.

## Why numeric metrics?

It gives the quality engine a deliberately small algebra:

```text
== != > >= < <=
```

The same evaluator can therefore operate on static analysis, tests, security, architecture, and runtime evidence.

## Collectors are anti-corruption layers

An analyzer can change its output without changing Code Gate's domain contract.

```text
tool output
    ↓
collector
    ↓
canonical metric
```

Example:

```text
cargo llvm-cov JSON
    ↓
LlvmCovCollector
    ↓
test.coverage.lines = 93.2
```

## Error model

Collector failure and gate failure are different.

```text
collector failed to execute   -> runtime/tooling error
metric missing                -> policy decides fail/warn/ignore
metric violates threshold     -> quality gate failure
```

`CollectorError` distinguishes the tooling cases:

```text
ToolMissing      analyzer (or cargo) not installed
Execution        analyzer ran but failed without usable evidence
Parse            analyzer output could not be interpreted
NotImplemented   collector is only a scaffold
```

An analyzer that runs and *finds problems* is a success for the collector:
Clippy warnings or compile errors become metrics and findings, never a
`CollectorError`.

## Process execution

Collectors do not call `std::process::Command` directly. They describe an
`Invocation` (program, arguments, working directory — no shell) and run it
through the `ToolRunner` trait from `code-gate-process`. Tests substitute a
fake runner, and parsing is a pure function over captured output, so
collector behavior is tested against fixtures without running the tool.

This distinction must remain visible in CLI exit codes and machine reports.

## Exit codes

```text
0 = all enforced gates passed
1 = at least one enforced quality gate failed
2 = invalid policy/report/tooling/runtime error
```

## Scores

Scores are deliberately not part of the v0.1 decision model.

A project with one critical vulnerability must not be made "healthy" because coverage is excellent. Scores may be added later as descriptive summaries, never as replacements for hard gates.
