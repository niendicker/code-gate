# Canonical Metrics Model

## Naming

Metric identifiers use dotted namespaces:

```text
<domain>.<subject>.<measurement>
```

Examples:

```text
code.clippy.warnings
code.complexity.cyclomatic.max
code.complexity.cognitive.p95
code.duplication.percent
test.coverage.lines
test.coverage.branches
test.mutation.score
security.vulnerabilities.critical
security.vulnerabilities.high
dependency.count.direct
dependency.cycles
unsafe.blocks
unsafe.functions
build.duration
runtime.loop.latency.p99
runtime.loop.jitter.p99
```

## Domains

Initial domains:

```text
code
test
security
dependency
unsafe
build
architecture
runtime
```

## Scope

Every metric has a scope.

Recommended scope forms:

```text
project
crate:<crate-name>
module:<module-path>
component:<component-name>
```

The v0.1 examples use simple names such as `project` and `core`; a future schema revision can enforce normalized scope URIs.

## Units

Initial canonical units:

- `count`
- `percent`
- `ratio`
- `score`
- `milliseconds`
- `bytes`
- `none`

A collector is responsible for normalizing units before producing a metric.

## Findings

A finding describes discrete evidence:

```text
rule_id
severity
message
source
scope
location?
```

Examples:

- an unsafe block in a forbidden module;
- a surviving mutant;
- a known vulnerability;
- a forbidden dependency edge;
- a Clippy diagnostic retained for audit.

## Implemented metrics

| Metric | Unit | Source | Scope | Collector |
|---|---|---|---|---|
| `code.clippy.total` | count | clippy | project | clippy |
| `code.clippy.warnings` | count | clippy | project | clippy |
| `code.clippy.errors` | count | clippy | project | clippy |
| `code.compiler.warnings` | count | rustc | project | clippy |
| `code.compiler.errors` | count | rustc | project | clippy |
| `test.coverage.lines` | percent | cargo-llvm-cov | project | llvm_cov |
| `test.coverage.functions` | percent | cargo-llvm-cov | project | llvm_cov |
| `test.coverage.regions` | percent | cargo-llvm-cov | project | llvm_cov |
| `test.coverage.branches` | percent | cargo-llvm-cov | project | llvm_cov |
| `security.vulnerabilities.total` | count | cargo-audit | project | audit |
| `security.vulnerabilities.critical` | count | cargo-audit | project | audit |
| `security.vulnerabilities.high` | count | cargo-audit | project | audit |
| `security.vulnerabilities.medium` | count | cargo-audit | project | audit |
| `security.vulnerabilities.low` | count | cargo-audit | project | audit |
| `security.vulnerabilities.unknown` | count | cargo-audit | project | audit |
| `dependency.unmaintained` | count | cargo-audit | project | audit |
| `dependency.unsound` | count | cargo-audit | project | audit |
| `dependency.yanked` | count | cargo-audit | project | audit |

A diagnostic is a Clippy one when its lint code starts with `clippy::`;
everything else rustc reports during the run is a compiler diagnostic.
Diagnostics are deduplicated because Cargo repeats them once per target that
compiles the same file. See [ADR-0002](adr/0002-clippy-collector.md).

Coverage percentages are `covered / count * 100` over LLVM's project totals.
A coverage kind with nothing instrumented (typically branches on stable Rust)
is omitted rather than reported as 0 % or 100 %. See
[ADR-0003](adr/0003-llvm-cov-collector.md).

Vulnerability severity is the CVSS qualitative rating of the advisory's
vector; CVSS 0.0 counts as `low`, and an advisory without a scorable vector
counts as `unknown`. The five severity buckets add up to `total`. See
[ADR-0004](adr/0004-cargo-audit-collector.md).

## Stability

Metric names become API once adopted.

New collectors should prefer existing canonical identifiers before adding analyzer-specific ones.
