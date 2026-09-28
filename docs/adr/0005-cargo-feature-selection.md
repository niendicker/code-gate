# ADR-0005: Cargo feature selection for building collectors

- Status: Accepted
- Date: 2026-09-27

## Context

The Clippy and cargo-llvm-cov collectors always passed `--all-features`.
That fails on real projects whose features cannot be enabled together, or
only work in some builds. Running Code Gate on `itoa`, for example, failed
because its `no-panic` feature is a link-time check that cannot link in a
coverage build; without `--all-features` the same suite measured 92.35 %.

## Decision

`[collectors.clippy]` and `[collectors.llvm_cov]`, the collectors that build
the workspace with Cargo, accept a feature selection:

```toml
[collectors.llvm_cov]
enabled = true
features = "all"             # default: --all-features
# features = "default"       # the crates' default features
# features = "none"          # --no-default-features
# features = ["std", "serde"]  # --features std,serde
# default_features = false   # with a list: adds --no-default-features
```

- The default stays `"all"`, so existing policies behave exactly as before.
- `default_features` is only accepted together with a list; with a keyword it
  would be redundant or contradictory, and is rejected rather than guessed.
- Feature names must be listed one per string; names containing commas or
  whitespace are rejected, because Cargo would split them.
- Collectors that do not build (`audit`, `bca`) do not accept these keys:
  their sections keep rejecting unknown fields.

The domain type `CargoFeatures` lives in `code-gate-core`, shared by the
configuration and the collectors, and maps a selection to Cargo flags.

## Consequences

- Projects with incompatible features can be analyzed.
- Clippy and coverage can use different selections, for example linting with
  all features while measuring coverage with the defaults.
- The selection is not recorded in `quality.json` yet, so a report alone does
  not say which feature set it measured; the policy file does.
