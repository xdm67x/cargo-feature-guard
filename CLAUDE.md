# cargo-feature-guard

A Cargo plugin that validates feature propagation across workspaces, detects forbidden features, and finds duplicate dependencies. Uses `cargo tree` as the source of truth.

## Quick Reference

```bash
cargo build                  # Build the binary
cargo test                   # Run all tests
cargo fmt --check            # Check formatting
cargo clippy --all-targets -- -D warnings  # Lint
```

## Architecture

Single-binary Rust CLI (`src/main.rs`) — the entire codebase lives in one file. No library crate.

### Key Components

- **Config** — Deserialized from `feature-guard.toml` (or `check-features.toml`). Defines `[[entry-points]]` and `[[never-enables]]` rules.
- **Workspace parser** — Reads workspace `Cargo.toml`, resolves glob members, collects each crate's feature definitions.
- **Cargo tree parser** — Runs `cargo tree -e features` and parses the output with regex to extract resolved features per crate.
- **Three checks** run in sequence:
  1. Feature propagation — gaps where a crate defines feature F but doesn't receive it
  2. Never-enables — forbidden features that must stay disabled
  3. Duplicate deps — informational, does not fail the build

### Exit Codes

| Code | Meaning |
|------|---------|
| 0    | All checks passed |
| 1    | Feature gaps or never-enables violations |
| 2    | CLI usage error |

## Code Conventions

- **Rust edition 2024**
- No external CLI parsing crate — args are parsed manually
- Clippy lints enforced: `complexity = deny`, `perf = deny`, `single_char_pattern = deny`, `todo = deny`
- Formatting enforced via `cargo fmt`
- Tests are inline in `mod tests` at the bottom of `main.rs`
- Use `BTreeMap`/`BTreeSet` for deterministic output, `HashMap`/`HashSet` for internal lookups
- Structs are plain (no derives beyond `Deserialize` for config types)

## CI

GitHub Actions (`.github/workflows/ci.yml`): fmt check, clippy, tests on `ubuntu-latest` with stable Rust.

## Dependencies

Minimal: `regex`, `serde` (with `derive`), `toml`. No async, no heavy frameworks.
