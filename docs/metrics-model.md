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

## Stability

Metric names become API once adopted.

New collectors should prefer existing canonical identifiers before adding analyzer-specific ones.
