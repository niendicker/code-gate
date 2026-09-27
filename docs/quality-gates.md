# Quality Gates

A quality gate is a deterministic predicate over one canonical metric.

## Rule

```text
metric + scope + operator + threshold
```

Example:

```text
test.coverage.lines @ core >= 95
```

## Operators

- `eq`
- `ne`
- `gt`
- `gte`
- `lt`
- `lte`

## Missing metrics

A rule also declares what happens when its metric is absent:

- `fail`
- `warn`
- `ignore`

Safety- or security-relevant gates should normally use `fail`.

## Severity

- `info`
- `warning`
- `error`
- `critical`

Severity describes the consequence/evidence. It does not change the mathematics of the gate.

## Hard invariants vs improvement metrics

Hard invariant:

```text
security.vulnerabilities.critical <= 0
```

Improvement metric:

```text
code.complexity.cognitive.p95 <= baseline + allowed_delta
```

Regression rules arrive in v0.3 and must remain distinct from absolute gates.

## Gate result

A gate evaluation records:

```text
rule id
metric
scope
observed value
operator
threshold
severity
status
```

The audit output must make a failed decision reproducible.
