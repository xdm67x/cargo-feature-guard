# Contributing

Thanks for contributing to `cargo-feature-guard`! This document explains how to set up your environment, structure your commits, and open pull requests.

## Development Setup

```bash
git clone https://github.com/xdm67x/cargo-feature-guard
cd cargo-feature-guard
cargo build
cargo test
```

Before opening a PR, make sure the following checks pass:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Commit Messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/). Every commit message MUST follow this format:

```
<type>(<scope>): <short summary>
```

- `type` — the kind of change: `feat`, `fix`, `docs`, `chore`, `refactor`, `test`, or `ci`.
- `scope` — (optional but encouraged) the area of the codebase affected, e.g. `cli`, `parser`, `config`, `checks`, `ci`.
- `summary` — imperative mood, lowercase, no trailing period, max 72 characters.

### Examples

```
feat(cli): add --json output flag
fix(parser): handle quoted feature names in cargo tree output
docs: expand exit code table in README
ci: cache cargo registry in test workflow
```

### Rules

- Keep the subject line under 72 characters.
- Use the body (separated by a blank line) to explain *what* and *why*, not *how*.
- Prefer small, focused commits — one logical change per commit.

## Pull Requests

1. **Fork and branch** — create a branch from `main` named after your change:

   ```bash
   git checkout -b feat/json-output
   ```

2. **Make your changes** and commit them following the commit message conventions above.

3. **Verify locally** — run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` before pushing.

4. **Push and open the PR**:

   ```bash
   git push -u origin feat/json-output
   ```

   Then open a pull request against `main` on GitHub (via the web UI or `gh pr create`).

### PR Guidelines

- **Title**: short, imperative mood, under 70 characters.
- **Description**: two short sections —
  - **Why** — the context and motivation. What problem does this change solve?
  - **How** — the overall approach. Focus on the strategy, not a file-by-file listing.
- Keep the description concise — aim for under 15 lines.
- Reference any related issues (e.g. `Closes #12`).
- PRs must pass CI (fmt, clippy, tests) before merging.

## Code Style

- Rust edition 2024; args are parsed manually (no CLI parsing crate).
- Tests are inline in `mod tests` at the bottom of `src/main.rs`.
- Use `BTreeMap`/`BTreeSet` for deterministic output, `HashMap`/`HashSet` for internal lookups.

## Reporting Issues

Open a GitHub issue with a minimal reproduction, your `cargo-feature-guard.toml` config (if relevant), and the output of `cargo tree -e features`.
