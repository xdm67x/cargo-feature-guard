# cargo-feature-guard

A Cargo plugin that validates feature propagation across workspaces, detects forbidden features, and finds duplicate dependencies. Uses `cargo tree` as the source of truth.

## Agent Configuration

Sub-agents and skills live in `.agents/` (agent-agnostic, single source of truth).
See [`.agents/AGENTS.md`](.agents/AGENTS.md). After adding or removing an agent or
skill, run `.agents/scripts/link-skills.sh` to update the symlinks in `.claude/`.

## Quick Reference

```bash
mise install                 # Install pinned tools (rust, hk, taplo)
mise run build              # Build the binary
mise run test                # Run all tests
mise run lint                # Lint via hk (cargo fmt --check, clippy, taplo)
mise run check               # Full local CI suite (fmt, clippy, tests)
mise run ci                  # Everything CI runs
```

Git hooks are managed by [hk](https://hk.jdx.dev/) (`hk.pkl`). After cloning, run
`mise x -- hk install` to install the pre-commit hook, which formats and lints
staged files before each commit.

## Architecture

Single-binary Rust CLI (`src/main.rs`) — the entire codebase lives in one file. No library crate.

### Key Components

- **Config** — Deserialized from `feature-guard.toml` (see `feature-guard.toml` docs). Defines `[[entry-points]]` and `[[never-enables]]` rules.
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

GitHub Actions (`.github/workflows/ci.yml`): tools installed via mise (`mise.toml`), then `mise run lint` (hk) and `mise run check` (fmt, clippy, tests) on `ubuntu-latest`.

## Dependencies

Minimal: `regex`, `serde` (with `derive`), `toml`. No async, no heavy frameworks.
