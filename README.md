# cargo-feature-guard

Validate Cargo feature propagation across a workspace. Catches common mistakes like forgetting to forward a feature flag through intermediate crates.

## What it checks

1. **Feature propagation** — detects crates that define a feature `F` but don't receive it when building an entry point with `--features F`
2. **Never-enables** — verifies that a forbidden feature is never activated in a given build context (e.g. `mock` must never reach `c_api`)
3. **Duplicate dependencies** — reports dependencies present in multiple versions (informational, does not fail the build)

Uses `cargo tree` as the source of truth, which accounts for feature unification by the Cargo resolver.

## Installation

```bash
# From crates.io
cargo install cargo-feature-guard

# Or with cargo-binstall (downloads pre-built binary)
cargo binstall cargo-feature-guard
```

## Usage

```bash
# Run in your workspace root (looks for feature-guard.toml or check-features.toml)
cargo feature-guard

# Specify a custom config file
cargo feature-guard --config path/to/config.toml
```

## Configuration

Create a `feature-guard.toml` in your workspace root:

```toml
# Define entry points and which features they should be built with.
# The tool checks that every crate defining one of these features actually
# receives it through the dependency graph.
[[entry-points]]
package = "my-app"
features = ["mock", "nfc"]

[[entry-points]]
package = "my-lib"
features = ["nfc"]

# Verify that a feature is NEVER enabled for a given package.
# Useful to ensure test-only features don't leak into production builds.
[[never-enables]]
package = "my-lib"
manifest-path = "my-lib/Cargo.toml"
forbidden = "mock"
```

### `[[entry-points]]`

| Field | Description |
|-------|-------------|
| `package` | The `-p` package name passed to `cargo tree` |
| `features` | Features to enable. Each feature is checked: if a workspace crate defines it but doesn't receive it, that's a gap. |

### `[[never-enables]]`

| Field | Description |
|-------|-------------|
| `package` | The `-p` package name |
| `manifest-path` | Path to the package's `Cargo.toml` |
| `forbidden` | Feature that must never be activated in this package's dependency tree |

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | All checks passed |
| `1` | Feature gaps or never-enables violations found |
| `2` | CLI usage error (bad arguments, missing config) |

## License

MIT
