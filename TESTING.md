# Testing Strategy

langchainx follows the seven-dimension testing model. Each dimension
covers a different lifecycle stage; skipping one leaves a gap that
surfaces later.

## Dimensions

| Dimension | When | Location |
|---|---|---|
| Unit | Every function | `#[cfg(test)]` in the same file |
| Property | Non-trivial input space | `tests/property_tests.rs` |
| Fuzz | Parsers, loaders, external input | `fuzz/fuzz_targets/` |
| Model Check | Arithmetic, bounds, invariants | `#[cfg(kani)]` inline |
| Conformance | Every `impl Trait` | `tests/conformance_*.rs` |
| Integration | Components wired together | `tests/e2e_*.rs` |
| Regression | After every bug fix | Alongside the fix |

## Test Tiers (CI)

- **Tier 1 — Offline**: FakeLLM/FakeEmbedder, no network. Always runs.
- **Tier 2 — Local LLM**: Ollama, skips if unavailable.
- **Tier 3 — Containers**: Postgres/Qdrant via smolvm, skips if unavailable.

## Running

```bash
# All offline tests (conformance + property + unit + e2e_offline)
cargo test --all-features

# Conformance only
cargo test --test 'conformance_*'

# Property tests only
cargo test --test property_tests

# Fuzz (requires nightly, 60s smoke)
cargo +nightly fuzz run fuzz_csv_loader -- -max_total_time=60

# Kani model check (requires cargo-kani)
cargo kani --harness token_usage_new_no_overflow -p langchainx-core
```

## Rules

- No `unwrap()` in tests without `expect("reason")`.
- Conformance test suites belong to the trait, not the impl.
- Fuzz corpus files (`fuzz/corpus/`) are permanent.
- `proptest-regressions/` directories are permanent.
- Every bug fix must include a regression test.
- Do not skip Unit to write Integration first.
