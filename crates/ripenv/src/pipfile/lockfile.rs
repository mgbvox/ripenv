//! `Pipfile.lock` generation from `uv.lock`.
//!
//! Converts the uv resolution output into a `Pipfile.lock` JSON file
//! for compatibility with pipenv-based workflows. After each successful
//! `uv lock`, we parse `uv.lock` from disk, walk the dependency graph
//! to categorize packages into `default` vs `develop`, collect hashes,
//! compute the Pipfile content hash, and write `Pipfile.lock` as
//! deterministic JSON.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uv_normalize::PackageName;
use uv_resolver::Lock;

use crate::pipfile::model::{Pipfile, PipfilePackage, PipfileRequires, PipfileSource};
use crate::printer::Printer;

// ---------------------------------------------------------------------------
// Pipfile.lock data model
// ---------------------------------------------------------------------------

/// Top-level `Pipfile.lock` structure.
#[derive(Debug, Serialize, Deserialize)]
pub struct PipfileLock {
    /// Metadata section.
    #[serde(rename = "_meta")]
    pub meta: PipfileLockMeta,
    /// Production dependencies (from `[packages]`).
    pub default: BTreeMap<String, PipfileLockPackage>,
    /// Development dependencies (from `[dev-packages]`).
    pub develop: BTreeMap<String, PipfileLockPackage>,
}

/// The `_meta` section of `Pipfile.lock`.
#[derive(Debug, Serialize, Deserialize)]
pub struct PipfileLockMeta {
    /// SHA256 hash of the Pipfile content.
    pub hash: PipfileLockHash,
    /// Pipfile.lock spec version (always 6).
    #[serde(rename = "pipfile-spec")]
    pub pipfile_spec: u32,
    /// Python requirements from the Pipfile.
    pub requires: serde_json::Value,
    /// Package index sources.
    pub sources: Vec<PipfileLockSource>,
}

/// Hash entry in `_meta`.
#[derive(Debug, Serialize, Deserialize)]
pub struct PipfileLockHash {
    /// SHA256 hex digest.
    pub sha256: String,
}

/// Source entry in `_meta.sources`.
#[derive(Debug, Serialize, Deserialize)]
pub struct PipfileLockSource {
    /// Source name (e.g., `"pypi"`).
    pub name: String,
    /// Index URL.
    pub url: String,
    /// Whether to verify SSL certificates.
    pub verify_ssl: bool,
}

/// A locked package entry in the `default` or `develop` sections.
#[derive(Debug, Serialize, Deserialize)]
pub struct PipfileLockPackage {
    /// SHA256 hashes from all distributions (sdist + wheels).
    pub hashes: Vec<String>,
    /// Source index name (only for registry packages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<String>,
    /// PEP 508 environment markers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markers: Option<String>,
    /// Pinned version string (e.g., `"==1.2.3"`).
    pub version: String,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Generate a `Pipfile.lock` from the current `uv.lock` and `Pipfile`.
///
/// Reads `uv.lock` from `project_dir`, categorizes packages into
/// `default` / `develop` based on the Pipfile dependency graph, and
/// writes `Pipfile.lock` as deterministic JSON.
pub fn generate_pipfile_lock(
    project_dir: &Path,
    pipfile: &Pipfile,
    printer: &Printer,
) -> Result<()> {
    let uv_lock_path = project_dir.join("uv.lock");
    if !uv_lock_path.is_file() {
        return Ok(());
    }

    let uv_lock_content =
        fs_err::read_to_string(&uv_lock_path).context("failed to read uv.lock")?;
    let lock: Lock = toml::from_str(&uv_lock_content).context("failed to parse uv.lock")?;

    // Build name -> package lookup, skipping the virtual root project.
    let mut packages_by_name: BTreeMap<String, Vec<&uv_resolver::Package>> = BTreeMap::new();
    for package in lock.packages() {
        if package.is_virtual() {
            continue;
        }
        if package.version().is_none() {
            continue;
        }
        let name = package.name().to_string();
        packages_by_name.entry(name).or_default().push(package);
    }

    // Walk the dependency graph to split default vs develop.
    let (default_names, develop_names) = categorize_packages(pipfile, &lock);

    let build_entry = |package: &uv_resolver::Package| -> PipfileLockPackage {
        let hashes: Vec<String> = package.hashes().iter().map(ToString::to_string).collect();

        let version = package
            .version()
            .map(|v| format!("=={v}"))
            .unwrap_or_default();

        // Attempt to find the matching Pipfile source name for registry packages.
        let index = find_source_name(package, project_dir, pipfile);

        PipfileLockPackage {
            hashes,
            index,
            markers: None,
            version,
        }
    };

    let mut default = BTreeMap::new();
    let mut develop = BTreeMap::new();

    for name in &default_names {
        if let Some(packages) = packages_by_name.get(name.as_str()) {
            if let Some(package) = packages.first() {
                default.insert(name.clone(), build_entry(package));
            }
        }
    }

    for name in &develop_names {
        if default_names.contains(name) {
            continue;
        }
        if let Some(packages) = packages_by_name.get(name.as_str()) {
            if let Some(package) = packages.first() {
                develop.insert(name.clone(), build_entry(package));
            }
        }
    }

    let pipfile_lock = PipfileLock {
        meta: PipfileLockMeta {
            hash: PipfileLockHash {
                sha256: compute_pipfile_hash(pipfile),
            },
            pipfile_spec: 6,
            requires: pipfile_requires_to_json(pipfile.requires.as_ref()),
            sources: pipfile
                .source
                .iter()
                .map(|s| PipfileLockSource {
                    name: s.name.clone(),
                    url: s.url.clone(),
                    verify_ssl: s.verify_ssl,
                })
                .collect(),
        },
        default,
        develop,
    };

    let json =
        serde_json::to_string_pretty(&pipfile_lock).context("failed to serialize Pipfile.lock")?;
    let lockfile_path = project_dir.join("Pipfile.lock");
    fs_err::write(&lockfile_path, format!("{json}\n")).context("failed to write Pipfile.lock")?;

    printer.debug(&format!(
        "Wrote Pipfile.lock to {}",
        lockfile_path.display()
    ));
    Ok(())
}

// ---------------------------------------------------------------------------
// Pipfile hash computation
// ---------------------------------------------------------------------------

/// Compute the Pipfile content hash for the `_meta.hash` field.
///
/// Matches pipenv's algorithm: SHA256 of a JSON string built from the
/// Pipfile content with sorted keys and compact separators (`","`, `":"`).
fn compute_pipfile_hash(pipfile: &Pipfile) -> String {
    let mut root = serde_json::Map::new();

    // _meta: {requires, sources}
    let mut meta = serde_json::Map::new();
    meta.insert(
        "requires".to_owned(),
        pipfile_requires_to_json(pipfile.requires.as_ref()),
    );
    let sources: Vec<serde_json::Value> = pipfile.source.iter().map(source_to_json).collect();
    meta.insert("sources".to_owned(), serde_json::Value::Array(sources));
    root.insert("_meta".to_owned(), serde_json::Value::Object(meta));

    // default (packages)
    root.insert("default".to_owned(), packages_to_json(&pipfile.packages));

    // develop (dev-packages)
    root.insert(
        "develop".to_owned(),
        packages_to_json(&pipfile.dev_packages),
    );

    let json = serde_json::to_string(&serde_json::Value::Object(root))
        .expect("JSON serialization cannot fail");

    let hash = Sha256::digest(json.as_bytes());
    format!("{hash:x}")
}

/// Convert `PipfileRequires` to a JSON value.
fn pipfile_requires_to_json(requires: Option<&PipfileRequires>) -> serde_json::Value {
    match requires {
        Some(req) => {
            let mut map = serde_json::Map::new();
            if let Some(ref version) = req.python_version {
                map.insert(
                    "python_version".to_owned(),
                    serde_json::Value::String(version.clone()),
                );
            }
            if let Some(ref full_version) = req.python_full_version {
                map.insert(
                    "python_full_version".to_owned(),
                    serde_json::Value::String(full_version.clone()),
                );
            }
            serde_json::Value::Object(map)
        }
        None => serde_json::Value::Object(serde_json::Map::new()),
    }
}

/// Convert a `PipfileSource` to a JSON value.
fn source_to_json(source: &PipfileSource) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert(
        "name".to_owned(),
        serde_json::Value::String(source.name.clone()),
    );
    map.insert(
        "url".to_owned(),
        serde_json::Value::String(source.url.clone()),
    );
    map.insert(
        "verify_ssl".to_owned(),
        serde_json::Value::Bool(source.verify_ssl),
    );
    serde_json::Value::Object(map)
}

/// Convert a Pipfile packages map to a JSON value.
///
/// Each package is serialized as its version string (e.g., `"*"` or
/// `{version = ">=1.0", extras = [...]}`) using the same structure
/// as the Pipfile TOML, but in JSON form.
fn packages_to_json(packages: &BTreeMap<String, PipfilePackage>) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (name, package) in packages {
        let value = match package {
            PipfilePackage::Simple(version) => serde_json::Value::String(version.clone()),
            PipfilePackage::Detailed(detail) => {
                serde_json::to_value(detail).unwrap_or(serde_json::Value::Null)
            }
        };
        map.insert(name.clone(), value);
    }
    serde_json::Value::Object(map)
}

// ---------------------------------------------------------------------------
// Dependency graph walk (default vs develop categorization)
// ---------------------------------------------------------------------------

/// Categorize resolved packages into `default` and `develop` sets.
///
/// Walks the dependency graph starting from Pipfile `[packages]` and
/// `[dev-packages]` roots. A package reachable from any default root
/// goes in `default`. A package reachable *only* from develop roots
/// goes in `develop`.
fn categorize_packages(pipfile: &Pipfile, lock: &Lock) -> (BTreeSet<String>, BTreeSet<String>) {
    // Build adjacency map: normalized package name -> set of dependency names.
    let mut adjacency: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for package in lock.packages() {
        let name = package.name().to_string();
        let deps: Vec<String> = package
            .dependencies()
            .iter()
            .map(|d| d.package_name().to_string())
            .collect();
        adjacency.insert(name, deps);
    }

    // BFS from default roots.
    let default_roots: Vec<String> = pipfile
        .packages
        .keys()
        .map(|k| normalize_package_name(k))
        .collect();
    let default_names = bfs_reachable(&adjacency, &default_roots);

    // BFS from develop roots.
    let develop_roots: Vec<String> = pipfile
        .dev_packages
        .keys()
        .map(|k| normalize_package_name(k))
        .collect();
    let develop_names = bfs_reachable(&adjacency, &develop_roots);

    // Develop-only = develop reachable minus default reachable.
    let develop_only: BTreeSet<String> =
        develop_names.difference(&default_names).cloned().collect();

    (default_names, develop_only)
}

/// BFS reachability from a set of root package names.
fn bfs_reachable(adjacency: &BTreeMap<String, Vec<String>>, roots: &[String]) -> BTreeSet<String> {
    let mut visited = BTreeSet::new();
    let mut queue: VecDeque<String> = roots.iter().cloned().collect();

    while let Some(name) = queue.pop_front() {
        if !visited.insert(name.clone()) {
            continue;
        }
        if let Some(deps) = adjacency.get(&name) {
            for dep in deps {
                if !visited.contains(dep) {
                    queue.push_back(dep.clone());
                }
            }
        }
    }
    visited
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Normalize a package name to match uv's convention (lowercase, hyphens).
fn normalize_package_name(name: &str) -> String {
    PackageName::from_str(name)
        .map(|n| n.to_string())
        .unwrap_or_else(|_| name.to_lowercase().replace('_', "-"))
}

/// Attempt to find the Pipfile source name for a registry package.
///
/// Matches the package's index URL against the Pipfile sources. Returns
/// `None` for non-registry packages or if no match is found.
fn find_source_name(
    package: &uv_resolver::Package,
    project_dir: &Path,
    pipfile: &Pipfile,
) -> Option<String> {
    let index_url = package.index(project_dir).ok()??;
    let index_str = index_url.to_string();

    // Match against Pipfile sources by URL prefix.
    for source in &pipfile.source {
        if index_str.starts_with(&source.url) || source.url.starts_with(index_str.as_str()) {
            return Some(source.name.clone());
        }
    }
    None
}

use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_package_name() {
        assert_eq!(normalize_package_name("Flask"), "flask");
        assert_eq!(normalize_package_name("my_package"), "my-package");
        assert_eq!(normalize_package_name("requests"), "requests");
        assert_eq!(normalize_package_name("Jinja2"), "jinja2");
    }

    #[test]
    fn test_pipfile_requires_to_json_empty() {
        let result = pipfile_requires_to_json(None);
        assert_eq!(result, serde_json::json!({}));
    }

    #[test]
    fn test_pipfile_requires_to_json_with_version() {
        let requires = PipfileRequires {
            python_version: Some("3.12".to_owned()),
            python_full_version: None,
        };
        let result = pipfile_requires_to_json(Some(&requires));
        assert_eq!(result, serde_json::json!({"python_version": "3.12"}));
    }

    #[test]
    fn test_packages_to_json_simple() {
        let mut packages = BTreeMap::new();
        packages.insert(
            "requests".to_owned(),
            PipfilePackage::Simple("*".to_owned()),
        );
        let result = packages_to_json(&packages);
        assert_eq!(result, serde_json::json!({"requests": "*"}));
    }

    #[test]
    fn test_compute_pipfile_hash_deterministic() {
        let pipfile = Pipfile::default_new();
        let hash1 = compute_pipfile_hash(&pipfile);
        let hash2 = compute_pipfile_hash(&pipfile);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 hex = 64 chars
    }

    #[test]
    fn test_bfs_reachable_simple() {
        let mut adjacency = BTreeMap::new();
        adjacency.insert(
            "flask".to_owned(),
            vec!["werkzeug".to_owned(), "jinja2".to_owned()],
        );
        adjacency.insert("werkzeug".to_owned(), vec!["markupsafe".to_owned()]);
        adjacency.insert("jinja2".to_owned(), vec!["markupsafe".to_owned()]);
        adjacency.insert("markupsafe".to_owned(), vec![]);

        let reachable = bfs_reachable(&adjacency, &["flask".to_owned()]);
        assert!(reachable.contains("flask"));
        assert!(reachable.contains("werkzeug"));
        assert!(reachable.contains("jinja2"));
        assert!(reachable.contains("markupsafe"));
        assert_eq!(reachable.len(), 4);
    }

    #[test]
    fn test_bfs_reachable_disjoint() {
        let mut adjacency = BTreeMap::new();
        adjacency.insert("flask".to_owned(), vec!["werkzeug".to_owned()]);
        adjacency.insert("werkzeug".to_owned(), vec![]);
        adjacency.insert("pytest".to_owned(), vec!["pluggy".to_owned()]);
        adjacency.insert("pluggy".to_owned(), vec![]);

        let default = bfs_reachable(&adjacency, &["flask".to_owned()]);
        let develop = bfs_reachable(&adjacency, &["pytest".to_owned()]);

        assert!(default.contains("flask"));
        assert!(default.contains("werkzeug"));
        assert!(!default.contains("pytest"));

        assert!(develop.contains("pytest"));
        assert!(develop.contains("pluggy"));
        assert!(!develop.contains("flask"));
    }
}
