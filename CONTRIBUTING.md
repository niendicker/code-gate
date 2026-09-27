# Contributing

## Development principles

- keep the core independent from analyzer-specific formats;
- prefer explicit domain types over unstructured maps;
- add new canonical metric names only when an existing metric cannot express the concept;
- add tests with every gate/evaluator behavior change;
- avoid adding runtime dependencies to `code-gate-core` without a clear domain reason.

## Local checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Commit scope

Prefer small commits that isolate:

- domain changes;
- collector integration;
- policy/schema changes;
- documentation changes.

Changes to canonical metric names or the configuration schema should include an ADR.
