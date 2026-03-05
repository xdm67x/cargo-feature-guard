//! Validate Cargo feature propagation across a workspace.
//!
//! Uses `cargo tree` as the source of truth (rather than static TOML analysis),
//! which accounts for feature unification by the Cargo resolver.
//!
//! Checks performed:
//!   1. propagate-feature — detects crates that define a feature F
//!      but don't receive it when building an entry point
//!   2. never-enables — verifies that a forbidden feature is never activated
//!      in a given context
//!   3. duplicate-deps — detects dependencies present in multiple versions

use regex::Regex;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

// ── Configuration (loaded from feature-guard.toml) ───────────────────────────

#[derive(Deserialize)]
struct Config {
    #[serde(rename = "entry-points")]
    entry_points: Vec<EntryPointConfig>,
    #[serde(rename = "never-enables")]
    never_enables: Vec<NeverEnablesConfig>,
}

#[derive(Deserialize)]
struct EntryPointConfig {
    package: String,
    features: Vec<String>,
}

fn string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct StringOrVec;

    impl<'de> de::Visitor<'de> for StringOrVec {
        type Value = Vec<String>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a string or array of strings")
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Vec<String>, E> {
            Ok(vec![value.to_owned()])
        }

        fn visit_seq<S: de::SeqAccess<'de>>(self, mut seq: S) -> Result<Vec<String>, S::Error> {
            let mut v = Vec::new();
            while let Some(s) = seq.next_element()? {
                v.push(s);
            }
            Ok(v)
        }
    }

    deserializer.deserialize_any(StringOrVec)
}

#[derive(Deserialize)]
struct NeverEnablesConfig {
    package: String,
    #[serde(deserialize_with = "string_or_vec")]
    forbidden: Vec<String>,
}

// ── Types ────────────────────────────────────────────────────────────────────

struct FeatureGap {
    entry_point: String,
    entry_features: Vec<String>,
    crate_name: String,
    feature: String,
    feature_content: Vec<String>,
}

struct NeverEnablesViolation {
    package: String,
    forbidden_feature: String,
    enabled_in: Vec<String>,
}

struct DuplicateDep {
    name: String,
    versions: Vec<String>,
}

struct CheckResult {
    feature_gaps: Vec<FeatureGap>,
    never_enables_violations: Vec<NeverEnablesViolation>,
    duplicate_deps: Vec<DuplicateDep>,
}

impl CheckResult {
    fn has_errors(&self) -> bool {
        !self.feature_gaps.is_empty() || !self.never_enables_violations.is_empty()
    }
}

// ── CLI argument parsing ────────────────────────────────────────────────────

struct CliArgs {
    config_path: Option<PathBuf>,
    init: bool,
}

fn parse_args() -> CliArgs {
    let mut args: Vec<String> = std::env::args().skip(1).collect();

    // When invoked as `cargo feature-guard`, Cargo passes "feature-guard" as the
    // first argument — skip it.
    if args.first().is_some_and(|a| a == "feature-guard") {
        args.remove(0);
    }

    let mut config_path = None;
    let mut init = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--init" => {
                init = true;
            }
            "--config" => {
                i += 1;
                if i < args.len() {
                    config_path = Some(PathBuf::from(&args[i]));
                } else {
                    eprintln!("❌ --config requires a path argument");
                    std::process::exit(2);
                }
            }
            "--help" | "-h" => {
                println!("Usage: cargo feature-guard [OPTIONS]");
                println!();
                println!(
                    "Validates Cargo feature propagation, forbidden features, and duplicate deps."
                );
                println!();
                println!("Options:");
                println!("  --init           Generate a starter feature-guard.toml");
                println!("  --config <path>  Path to config file (default: feature-guard.toml)");
                println!("  -h, --help       Print this help message");
                println!("  -V, --version    Print version");
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("cargo-feature-guard {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            other => {
                eprintln!("❌ unexpected argument '{other}'");
                eprintln!("Usage: cargo feature-guard [--init] [--config <path>]");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    CliArgs { config_path, init }
}

// ── Config file resolution ──────────────────────────────────────────────────

fn resolve_config_path(root: &Path, explicit: Option<PathBuf>) -> PathBuf {
    if let Some(p) = explicit {
        return p;
    }

    let primary = root.join("feature-guard.toml");
    if primary.exists() {
        return primary;
    }

    eprintln!("❌ no config file found (looked for feature-guard.toml)");
    std::process::exit(2);
}

// ── Init command ─────────────────────────────────────────────────────────────

const INIT_TEMPLATE: &str = r#"# Feature Guard Configuration
# See: https://github.com/xdm67x/cargo-feature-guard
#
# [[entry-points]]    — packages + features to validate propagation for
# [[never-enables]]   — features that must never be activated

[[entry-points]]
package = "my-app"
features = ["default"]

# [[entry-points]]
# package = "my-lib"
# features = ["std", "serde"]

[[never-enables]]
package = "my-app"
forbidden = "mock"

# [[never-enables]]
# package = "my-lib"
# forbidden = ["test-only", "unstable"]
"#;

fn handle_init(root: &Path) -> Result<(), String> {
    if !root.join("Cargo.toml").exists() {
        return Err("not a Cargo project (no Cargo.toml found)".to_string());
    }

    let config_path = root.join("feature-guard.toml");
    if config_path.exists() {
        return Err("feature-guard.toml already exists".to_string());
    }

    std::fs::write(&config_path, INIT_TEMPLATE)
        .map_err(|e| format!("failed to write feature-guard.toml: {e}"))?;

    println!("✅ Created feature-guard.toml — edit it to match your workspace.");
    Ok(())
}

// ── Workspace parsing ────────────────────────────────────────────────────────

struct CrateInfo {
    features: HashMap<String, Vec<String>>,
}

fn parse_workspace(root: &Path) -> HashMap<String, CrateInfo> {
    let cargo_path = root.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {e}", cargo_path.display()));
    let data: toml::Value = content
        .parse()
        .unwrap_or_else(|e| panic!("Cannot parse {}: {e}", cargo_path.display()));

    let members = data["workspace"]["members"]
        .as_array()
        .expect("workspace.members should be an array");

    let mut crates = HashMap::new();
    for member in members {
        let member_str = member.as_str().unwrap();
        // Handle glob patterns in workspace members
        let member_dirs = resolve_workspace_member(root, member_str);
        for crate_dir in member_dirs {
            let ct = crate_dir.join("Cargo.toml");
            let Ok(ct_content) = std::fs::read_to_string(&ct) else {
                continue;
            };
            let d: toml::Value = ct_content
                .parse()
                .unwrap_or_else(|e| panic!("Cannot parse {}: {e}", ct.display()));

            let Some(name) = d
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
            else {
                continue;
            };
            let name = name.to_string();

            let features = match d.get("features").and_then(|f| f.as_table()) {
                Some(table) => table
                    .iter()
                    .map(|(k, v)| {
                        let values = v
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|x| x.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default();
                        (k.clone(), values)
                    })
                    .collect(),
                None => HashMap::new(),
            };

            crates.insert(name, CrateInfo { features });
        }
    }
    crates
}

fn resolve_workspace_member(root: &Path, pattern: &str) -> Vec<PathBuf> {
    if pattern.contains('*') {
        // Simple glob: "foo/*" → list subdirectories of "foo/"
        let prefix = pattern.trim_end_matches('*').trim_end_matches('/');
        let base = root.join(prefix);
        let Ok(entries) = std::fs::read_dir(&base) else {
            return Vec::new();
        };
        entries
            .filter_map(|e| {
                let e = e.ok()?;
                if e.file_type().ok()?.is_dir() && e.path().join("Cargo.toml").exists() {
                    Some(e.path())
                } else {
                    None
                }
            })
            .collect()
    } else {
        vec![root.join(pattern)]
    }
}

// ── Cargo tree parsing ───────────────────────────────────────────────────────

fn parse_cargo_tree_output(
    stdout: &str,
    re: &Regex,
    prefix_re: &Regex,
) -> HashMap<String, HashSet<String>> {
    let mut resolved: HashMap<String, HashSet<String>> = HashMap::new();
    for line in stdout.lines() {
        let clean = prefix_re.replace(line, "");
        if let Some(caps) = re.captures(&clean) {
            let crate_name = caps[1].to_string();
            let feats: HashSet<String> = caps[2]
                .split(',')
                .map(|f| f.trim().to_string())
                .filter(|f| !f.is_empty())
                .collect();
            resolved.entry(crate_name).or_default().extend(feats);
        }
    }
    resolved
}

// ── Check 1: Feature propagation ─────────────────────────────────────────────

fn cargo_tree_resolved_features(
    pkg: &str,
    features: &[&str],
    re: &Regex,
    prefix_re: &Regex,
) -> HashMap<String, HashSet<String>> {
    let features_str = features.join(",");
    let output = Command::new("cargo")
        .args([
            "tree",
            "-e",
            "features",
            "-p",
            pkg,
            "--features",
            &features_str,
            "-f",
            "{p} [{f}]",
        ])
        .output()
        .expect("Failed to run cargo tree");

    if !output.status.success() {
        eprintln!("  ⚠️ cargo tree failed for {pkg} --features {features_str}:");
        eprintln!("    {}", String::from_utf8_lossy(&output.stderr).trim());
        return HashMap::new();
    }

    parse_cargo_tree_output(&String::from_utf8_lossy(&output.stdout), re, prefix_re)
}

fn check_feature_propagation(
    config: &Config,
    crates: &HashMap<String, CrateInfo>,
    re: &Regex,
    prefix_re: &Regex,
) -> Vec<FeatureGap> {
    let mut all_gaps = Vec::new();
    let mut seen = HashSet::new();

    for ep in &config.entry_points {
        let features_ref: Vec<&str> = ep.features.iter().map(|s| s.as_str()).collect();
        let label = format!("{} --features {}", ep.package, ep.features.join(","));
        println!("  {label}");

        let resolved = cargo_tree_resolved_features(&ep.package, &features_ref, re, prefix_re);
        if resolved.is_empty() {
            continue;
        }

        let mut gaps = Vec::new();
        for (crate_name, crate_info) in crates {
            let Some(active) = resolved.get(crate_name) else {
                continue;
            };

            for (feature, content) in &crate_info.features {
                if active.contains(feature) {
                    continue;
                }
                if features_ref.contains(&feature.as_str()) {
                    gaps.push(FeatureGap {
                        entry_point: ep.package.clone(),
                        entry_features: ep.features.clone(),
                        crate_name: crate_name.clone(),
                        feature: feature.clone(),
                        feature_content: content.clone(),
                    });
                }
            }
        }

        let status = if gaps.is_empty() {
            "✅ ok".to_string()
        } else {
            format!("⚠️ {} gap(s)", gaps.len())
        };
        println!("    {status}");

        for gap in gaps {
            let key = format!("{}::{}::{}", gap.crate_name, gap.feature, gap.entry_point);
            if seen.insert(key) {
                all_gaps.push(gap);
            }
        }
    }

    all_gaps
}

// ── Check 2: Never-enables ───────────────────────────────────────────────────

fn check_never_enables(
    config: &Config,
    re: &Regex,
    prefix_re: &Regex,
) -> Vec<NeverEnablesViolation> {
    let mut violations = Vec::new();

    for rule in &config.never_enables {
        let forbidden_list = rule.forbidden.join(", ");
        println!(
            "  {}: [{}] must stay disabled",
            rule.package, forbidden_list
        );

        let output = Command::new("cargo")
            .args([
                "tree",
                "-e",
                "features",
                "-p",
                &rule.package,
                "-f",
                "{p} [{f}]",
            ])
            .output()
            .expect("Failed to run cargo tree");

        if !output.status.success() {
            eprintln!(
                "    ⚠️ cargo tree failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
            continue;
        }

        let resolved =
            parse_cargo_tree_output(&String::from_utf8_lossy(&output.stdout), re, prefix_re);

        for forbidden in &rule.forbidden {
            let enabled_in: Vec<String> = resolved
                .iter()
                .filter(|(_, feats)| feats.contains(forbidden))
                .map(|(name, _)| name.clone())
                .collect();

            if !enabled_in.is_empty() {
                println!(
                    "    ⚠️ '{}' enabled in: {}",
                    forbidden,
                    enabled_in.join(", ")
                );
                violations.push(NeverEnablesViolation {
                    package: rule.package.clone(),
                    forbidden_feature: forbidden.clone(),
                    enabled_in,
                });
            } else {
                println!("    ✅ '{}': ok", forbidden);
            }
        }
    }

    violations
}

// ── Check 3: Duplicate dependencies ──────────────────────────────────────────

fn check_duplicate_deps() -> Vec<DuplicateDep> {
    let output = Command::new("cargo")
        .args(["tree", "-d", "--depth=0"])
        .output()
        .expect("Failed to run cargo tree");

    if !output.status.success() && output.stdout.is_empty() {
        eprintln!(
            "  ⚠️ cargo tree -d failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        return Vec::new();
    }

    let re = Regex::new(r"^(\S+) v([\d.]+\S*)").unwrap();
    let mut deps: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(caps) = re.captures(line) {
            let name = caps[1].to_string();
            let version = caps[2].to_string();
            deps.entry(name).or_default().insert(version);
        }
    }

    let duplicates: Vec<DuplicateDep> = deps
        .into_iter()
        .filter(|(_, versions)| versions.len() > 1)
        .map(|(name, versions)| DuplicateDep {
            name,
            versions: versions.into_iter().collect(),
        })
        .collect();

    if !duplicates.is_empty() {
        println!("  ⚠️ {} duplicate dep(s)", duplicates.len());
        for dup in &duplicates {
            println!("    {}: {}", dup.name, dup.versions.join(", "));
        }
    } else {
        println!("  ✅ No duplicate dependencies");
    }

    duplicates
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() -> ExitCode {
    let cli = parse_args();
    let root = std::env::current_dir().expect("Cannot get current directory");

    if cli.init {
        if let Err(msg) = handle_init(&root) {
            eprintln!("❌ {msg}");
            return ExitCode::from(2);
        }
        return ExitCode::SUCCESS;
    }

    let config_path = resolve_config_path(&root, cli.config_path);

    let config_content = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {e}", config_path.display()));
    let config: Config = toml::from_str(&config_content)
        .unwrap_or_else(|e| panic!("Cannot parse {}: {e}", config_path.display()));

    let crates = parse_workspace(&root);

    let re = Regex::new(r"(\S+) v[\d.]+ \([^)]+\) \[([^\]]*)\]").unwrap();
    let prefix_re = Regex::new(r"^[│├└─\s]+").unwrap();

    println!("📦 Workspace: {} crates\n", crates.len());

    println!("🔗 [1/3] Feature propagation");
    let feature_gaps = check_feature_propagation(&config, &crates, &re, &prefix_re);
    println!();

    println!("🚫 [2/3] Never-enables");
    let never_enables_violations = check_never_enables(&config, &re, &prefix_re);
    println!();

    println!("📋 [3/3] Duplicate dependencies");
    let duplicate_deps = check_duplicate_deps();
    println!();

    let result = CheckResult {
        feature_gaps,
        never_enables_violations,
        duplicate_deps,
    };

    // ── Summary ──────────────────────────────────────────────────────────────

    println!("{}", "━".repeat(60));

    if !result.has_errors() {
        println!("✅ All checks passed!");
        return ExitCode::SUCCESS;
    }

    if !result.feature_gaps.is_empty() {
        println!("\n❌ {} feature gap(s):\n", result.feature_gaps.len());
        for gap in &result.feature_gaps {
            let features_str = gap.entry_features.join(",");
            println!(
                "  {}: feature '{}' not enabled",
                gap.crate_name, gap.feature
            );
            println!(
                "    📍 Entry point: {} --features {features_str}",
                gap.entry_point
            );
            if !gap.feature_content.is_empty() {
                println!("    📄 Defined content: {:?}", gap.feature_content);
            }
            println!(
                "    💡 Fix: forward '{}' from a parent crate in the dependency graph",
                gap.feature
            );
            println!();
        }
    }

    if !result.never_enables_violations.is_empty() {
        println!(
            "\n❌ {} never-enables violation(s):\n",
            result.never_enables_violations.len()
        );
        for v in &result.never_enables_violations {
            println!("  '{}' is enabled in {}:", v.forbidden_feature, v.package);
            for c in &v.enabled_in {
                println!("    - {c}");
            }
            println!();
        }
    }

    if !result.duplicate_deps.is_empty() {
        println!(
            "\n⚠️ {} duplicate dep(s) (informational):\n",
            result.duplicate_deps.len()
        );
        for dup in &result.duplicate_deps {
            println!("  {}: {}", dup.name, dup.versions.join(", "));
        }
        println!();
    }

    ExitCode::FAILURE
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_cargo_tree_output_basic() {
        let re = Regex::new(r"(\S+) v[\d.]+ \([^)]+\) \[([^\]]*)\]").unwrap();
        let prefix_re = Regex::new(r"^[│├└─\s]+").unwrap();

        let input = "\
engine v0.1.0 (/path/to/engine) [mock,nfc]
├── fsv_api v0.1.0 (/path/to/fsv_api) [mock]
│   └── smart_card_utils v0.1.0 (/path/to/scu) [mock,record_apdu]
└── storage v0.1.0 (/path/to/storage) []";

        let result = parse_cargo_tree_output(input, &re, &prefix_re);

        assert_eq!(
            result.get("engine").unwrap(),
            &HashSet::from(["mock".to_string(), "nfc".to_string()])
        );
        assert_eq!(
            result.get("fsv_api").unwrap(),
            &HashSet::from(["mock".to_string()])
        );
        assert_eq!(
            result.get("smart_card_utils").unwrap(),
            &HashSet::from(["mock".to_string(), "record_apdu".to_string()])
        );
        assert!(result.get("storage").unwrap().is_empty());
    }

    #[test]
    fn test_parse_cargo_tree_output_empty() {
        let re = Regex::new(r"(\S+) v[\d.]+ \([^)]+\) \[([^\]]*)\]").unwrap();
        let prefix_re = Regex::new(r"^[│├└─\s]+").unwrap();

        let result = parse_cargo_tree_output("", &re, &prefix_re);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_cargo_tree_output_merges_features() {
        let re = Regex::new(r"(\S+) v[\d.]+ \([^)]+\) \[([^\]]*)\]").unwrap();
        let prefix_re = Regex::new(r"^[│├└─\s]+").unwrap();

        let input = "\
root v0.1.0 (/path) [a]
├── dep v0.1.0 (/path/dep) [feat1]
└── dep v0.1.0 (/path/dep) [feat2]";

        let result = parse_cargo_tree_output(input, &re, &prefix_re);
        let dep_feats = result.get("dep").unwrap();
        assert!(dep_feats.contains("feat1"));
        assert!(dep_feats.contains("feat2"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"
[[entry-points]]
package = "daemon"
features = ["mock", "nfc"]

[[entry-points]]
package = "c_api"
features = ["nfc"]

[[never-enables]]
package = "c_api"
forbidden = "mock"
"#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.entry_points.len(), 2);
        assert_eq!(config.entry_points[0].package, "daemon");
        assert_eq!(config.entry_points[0].features, vec!["mock", "nfc"]);
        assert_eq!(config.never_enables.len(), 1);
        assert_eq!(config.never_enables[0].forbidden, vec!["mock"]);
    }

    #[test]
    fn test_config_deserialization_forbidden_array() {
        let toml_str = r#"
[[entry-points]]
package = "daemon"
features = ["mock"]

[[never-enables]]
package = "daemon"
forbidden = ["mock", "test-only"]
"#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.never_enables.len(), 1);
        assert_eq!(config.never_enables[0].forbidden, vec!["mock", "test-only"]);
    }

    #[test]
    fn test_config_deserialization_empty_sections() {
        let toml_str = r#"
entry-points = []
never-enables = []
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.entry_points.is_empty());
        assert!(config.never_enables.is_empty());
    }

    #[test]
    fn test_resolve_config_explicit_path() {
        let tmp = std::env::temp_dir().join("cfg-test-explicit");
        std::fs::create_dir_all(&tmp).unwrap();
        let custom = tmp.join("custom.toml");
        std::fs::File::create(&custom)
            .unwrap()
            .write_all(b"")
            .unwrap();

        let result = resolve_config_path(&tmp, Some(custom.clone()));
        assert_eq!(result, custom);

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_resolve_config_primary_fallback() {
        let tmp = std::env::temp_dir().join("cfg-test-primary");
        std::fs::create_dir_all(&tmp).unwrap();

        // Only feature-guard.toml exists → should pick it
        let primary = tmp.join("feature-guard.toml");
        std::fs::File::create(&primary)
            .unwrap()
            .write_all(b"")
            .unwrap();

        let result = resolve_config_path(&tmp, None);
        assert_eq!(result, primary);

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_check_result_has_errors() {
        let empty = CheckResult {
            feature_gaps: vec![],
            never_enables_violations: vec![],
            duplicate_deps: vec![],
        };
        assert!(!empty.has_errors());

        let with_gap = CheckResult {
            feature_gaps: vec![FeatureGap {
                entry_point: "pkg".to_string(),
                entry_features: vec!["f".to_string()],
                crate_name: "dep".to_string(),
                feature: "f".to_string(),
                feature_content: vec![],
            }],
            never_enables_violations: vec![],
            duplicate_deps: vec![],
        };
        assert!(with_gap.has_errors());

        // Duplicate deps alone don't count as errors (informational)
        let with_dups = CheckResult {
            feature_gaps: vec![],
            never_enables_violations: vec![],
            duplicate_deps: vec![DuplicateDep {
                name: "serde".to_string(),
                versions: vec!["1.0".to_string(), "2.0".to_string()],
            }],
        };
        assert!(!with_dups.has_errors());
    }

    #[test]
    fn init_creates_config_file() {
        let tmp = std::env::temp_dir().join("init-test-creates");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::File::create(tmp.join("Cargo.toml")).unwrap();

        // Remove any leftover config from a previous run
        let _ = std::fs::remove_file(tmp.join("feature-guard.toml"));

        handle_init(&tmp).unwrap();

        let content = std::fs::read_to_string(tmp.join("feature-guard.toml")).unwrap();
        assert!(content.contains("[[entry-points]]"));
        assert!(content.contains("[[never-enables]]"));

        // Verify the generated file is valid TOML that deserializes into Config
        let config: Config = toml::from_str(&content).unwrap();
        assert_eq!(config.entry_points.len(), 1);
        assert_eq!(config.never_enables.len(), 1);

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn init_fails_without_cargo_toml() {
        let tmp = std::env::temp_dir().join("init-test-no-cargo");
        std::fs::create_dir_all(&tmp).unwrap();
        let _ = std::fs::remove_file(tmp.join("Cargo.toml"));

        let result = handle_init(&tmp);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no Cargo.toml"));

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn init_fails_if_config_exists() {
        let tmp = std::env::temp_dir().join("init-test-exists");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::File::create(tmp.join("Cargo.toml")).unwrap();
        std::fs::File::create(tmp.join("feature-guard.toml"))
            .unwrap()
            .write_all(b"existing")
            .unwrap();

        let result = handle_init(&tmp);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));

        std::fs::remove_dir_all(&tmp).ok();
    }
}
