# ADR-0001: Use a canonical numeric metric model

- Status: Accepted
- Date: 2026-09-27

## Context

Code Gate integrates analyzers with different output formats and vocabularies. Coupling policy rules directly to each tool would make the quality model unstable and make tool replacement expensive.

## Decision

Code Gate will normalize analyzer output into a tool-independent domain model.

A metric consists of:

- stable identifier;
- numeric value (`f64`);
- canonical unit;
- source;
- scope.

Discrete textual evidence is represented as a finding.

## Consequences

### Positive

- analyzers are replaceable;
- policy remains tool-independent;
- comparison and history are straightforward;
- gate evaluation stays small and deterministic;
- runtime and static evidence can share one model.

### Negative

- collectors must normalize analyzer-specific concepts;
- some rich analyzer data cannot be expressed as a metric and must remain in findings;
- metric naming requires governance.

## Alternatives rejected

### Tool-specific policy rules

Rejected because it creates direct coupling between policy and analyzer output.

### Arbitrary JSON values as metric values

Rejected because it dramatically complicates comparison, trends, and gate semantics for little practical benefit.
