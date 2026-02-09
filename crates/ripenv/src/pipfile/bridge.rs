//! Bridge from Pipfile to uv's `PyProjectToml`.
//!
//! The central function [`pipfile_to_pyproject_toml`] converts a parsed
//! [`Pipfile`] into a TOML string that uv can parse as a `PyProjectToml`.
//! This lets ripenv delegate to uv's project machinery without forking it.

use std::fmt::Write;

use anyhow::{Context, Result};

use crate::pipfile::model::{Pipfile, PipfilePackage, PipfilePackageDetail, PipfileSource};

/// Convert a Pipfile into a virtual pyproject.toml TOML string.
///
/// The resulting string can be parsed by `PyProjectToml::from_string()`.
/// Git, path, and editable packages are represented via `[tool.uv.sources]`.
pub fn pipfile_to_pyproject_toml(pipfile: &Pipfile, project_name: &str) -> Result<String> {
    let mut toml = String::with_capacity(1024);

    // [project]
    writeln!(toml, "[project]")?;
    writeln!(toml, "name = \"{}\"", escape_toml_string(project_name))?;
    writeln!(toml, "version = \"0.0.0\"")?;

    // requires-python
    if let Some(requires) = &pipfile.requires {
        if let Some(ref version) = requires.python_version {
            writeln!(toml, "requires-python = \">={version}\"")?;
        } else if let Some(ref full_version) = requires.python_full_version {
            writeln!(toml, "requires-python = \"=={full_version}\"")?;
        }
    }

    // [project.dependencies]
    let deps = convert_packages(&pipfile.packages);
    writeln!(toml, "dependencies = [")?;
    for dep in &deps {
        writeln!(toml, "    \"{}\",", escape_toml_string(&dep.requirement))?;
    }
    writeln!(toml, "]")?;

    // [dependency-groups]
    let dev_deps = convert_packages(&pipfile.dev_packages);
    if !dev_deps.is_empty() {
        writeln!(toml)?;
        writeln!(toml, "[dependency-groups]")?;
        writeln!(toml, "dev = [")?;
        for dep in &dev_deps {
            writeln!(toml, "    \"{}\",", escape_toml_string(&dep.requirement))?;
        }
        writeln!(toml, "]")?;
    }

    // [[tool.uv.index]] for sources
    if !pipfile.source.is_empty() {
        writeln!(toml)?;
        for (i, source) in pipfile.source.iter().enumerate() {
            write_index_entry(&mut toml, source, i == 0)?;
        }
    }

    // [tool.uv.sources] for git/path/editable packages
    let all_sources: Vec<_> = deps
        .iter()
        .chain(dev_deps.iter())
        .filter(|d| d.source.is_some())
        .collect();

    if !all_sources.is_empty() {
        writeln!(toml)?;
        writeln!(toml, "[tool.uv.sources]")?;
        for dep in &all_sources {
            if let Some(ref source) = dep.source {
                writeln!(toml, "{} = {source}", dep.name)?;
            }
        }
    }

    Ok(toml)
}

/// A converted dependency with its PEP 508 requirement string and optional uv source.
struct ConvertedDep {
    /// The normalized package name (for use in `[tool.uv.sources]` keys).
    name: String,
    /// The PEP 508 requirement string (for `[project.dependencies]`).
    requirement: String,
    /// An optional inline TOML table for `[tool.uv.sources]`.
    source: Option<String>,
}

/// Convert a map of Pipfile packages to PEP 508 requirement strings.
fn convert_packages(
    packages: &std::collections::BTreeMap<String, PipfilePackage>,
) -> Vec<ConvertedDep> {
    packages
        .iter()
        .map(|(name, pkg)| convert_package(name, pkg))
        .collect()
}

/// Convert a single Pipfile package to a PEP 508 requirement string.
fn convert_package(name: &str, package: &PipfilePackage) -> ConvertedDep {
    match package {
        PipfilePackage::Simple(version) => ConvertedDep {
            name: name.to_owned(),
            requirement: format_simple_requirement(name, version),
            source: None,
        },
        PipfilePackage::Detailed(detail) => convert_detailed_package(name, detail),
    }
}

/// Format a simple version requirement like `requests>=1.0` or just `requests`.
fn format_simple_requirement(name: &str, version: &str) -> String {
    if version == "*" {
        name.to_owned()
    } else {
        format!("{name}{version}")
    }
}

/// Convert a detailed package spec to a requirement string + optional source.
fn convert_detailed_package(name: &str, detail: &PipfilePackageDetail) -> ConvertedDep {
    let mut req = String::new();

    // Package name
    req.push_str(name);

    // Extras: requests[security,tests]
    if !detail.extras.is_empty() {
        req.push('[');
        req.push_str(&detail.extras.join(","));
        req.push(']');
    }

    // Version specifier (only for non-VCS, non-path packages)
    if detail.git.is_none() && detail.path.is_none() {
        if let Some(ref version) = detail.version {
            if version != "*" {
                req.push_str(version);
            }
        }
    }

    // Environment markers
    let marker = build_marker(detail);
    if !marker.is_empty() {
        req.push_str("; ");
        req.push_str(&marker);
    }

    // Build uv source for git/path/editable packages
    let source = build_uv_source(detail);

    ConvertedDep {
        name: name.to_owned(),
        requirement: req,
        source,
    }
}

/// Build a PEP 508 marker string from detail fields.
fn build_marker(detail: &PipfilePackageDetail) -> String {
    let mut parts = Vec::new();

    if let Some(ref markers) = detail.markers {
        parts.push(markers.clone());
    }

    if let Some(ref sys_platform) = detail.sys_platform {
        parts.push(format!("sys_platform {sys_platform}"));
    }

    parts.join(" and ")
}

/// Build an inline TOML source table for `[tool.uv.sources]`.
fn build_uv_source(detail: &PipfilePackageDetail) -> Option<String> {
    if let Some(ref git) = detail.git {
        let mut parts = vec![format!("git = \"{git}\"")];
        if let Some(ref git_ref) = detail.git_ref {
            // Map pipenv's generic `ref` to uv's `rev`
            parts.push(format!("rev = \"{git_ref}\""));
        }
        return Some(format!("{{ {} }}", parts.join(", ")));
    }

    if let Some(ref path) = detail.path {
        let mut parts = vec![format!("path = \"{path}\"")];
        if detail.editable {
            parts.push("editable = true".to_owned());
        }
        return Some(format!("{{ {} }}", parts.join(", ")));
    }

    // Index-pinned packages: source is the index name
    if let Some(ref index) = detail.index {
        return Some(format!("{{ index = \"{index}\" }}"));
    }

    None
}

/// Write a `[[tool.uv.index]]` entry for a Pipfile source.
fn write_index_entry(toml: &mut String, source: &PipfileSource, is_first: bool) -> Result<()> {
    writeln!(toml, "[[tool.uv.index]]").context("failed to write index entry")?;
    writeln!(toml, "name = \"{}\"", escape_toml_string(&source.name))?;
    writeln!(toml, "url = \"{}\"", escape_toml_string(&source.url))?;
    // The first source in pipenv is typically the default index.
    if is_first {
        writeln!(toml, "default = true")?;
    }
    writeln!(toml)?;
    Ok(())
}

/// Escape a string for use in a TOML quoted string value.
fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_pipfile() -> Pipfile {
        Pipfile::from_path(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/minimal/Pipfile"),
        )
        .unwrap()
    }

    #[test]
    fn bridge_minimal_pipfile() {
        let pipfile = minimal_pipfile();
        let toml = pipfile_to_pyproject_toml(&pipfile, "test-project").unwrap();

        assert!(toml.contains("[project]"));
        assert!(toml.contains("name = \"test-project\""));
        assert!(toml.contains("requires-python = \">=3.12\""));
        assert!(toml.contains("\"requests\""));
    }

    #[test]
    fn bridge_complex_specs() {
        let pipfile = Pipfile::from_path(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/complex-specs/Pipfile"),
        )
        .unwrap();
        let toml = pipfile_to_pyproject_toml(&pipfile, "complex").unwrap();

        // Extras should be included
        assert!(toml.contains("requests[security]>=2.32.0"));
        // Markers should be present
        assert!(toml.contains("sys_platform"));
        // Editable source should be present
        assert!(toml.contains("[tool.uv.sources]"));
        assert!(toml.contains("editable = true"));
    }

    #[test]
    fn bridge_vcs_packages() {
        let pipfile = Pipfile::from_path(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/vcs-and-editable/Pipfile"),
        )
        .unwrap();
        let toml = pipfile_to_pyproject_toml(&pipfile, "vcs-test").unwrap();

        // Git source
        assert!(toml.contains("git = \"https://github.com/example/my-git-pkg.git\""));
        assert!(toml.contains("rev = \"main\""));
        // Path source
        assert!(toml.contains("path = \"./local-pkg\""));
    }

    #[test]
    fn bridge_multiple_sources() {
        let pipfile = Pipfile::from_path(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/multiple-sources/Pipfile"),
        )
        .unwrap();
        let toml = pipfile_to_pyproject_toml(&pipfile, "multi-src").unwrap();

        // Both index entries should be present
        assert!(toml.contains("[[tool.uv.index]]"));
        assert!(toml.contains("name = \"pypi\""));
        assert!(toml.contains("name = \"private\""));
        // First source should be default
        assert!(toml.contains("default = true"));
        // Index-pinned package should have source
        assert!(toml.contains("index = \"private\""));
    }

    #[test]
    fn bridge_dev_packages() {
        let pipfile = Pipfile::from_path(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/with-dev-deps/Pipfile"),
        )
        .unwrap();
        let toml = pipfile_to_pyproject_toml(&pipfile, "dev-test").unwrap();

        assert!(toml.contains("[dependency-groups]"));
        assert!(toml.contains("dev = ["));
        assert!(toml.contains("pytest>=7.4.0"));
        assert!(toml.contains("pytest-cov==4.*"));
    }

    #[test]
    fn bridge_wildcard_version() {
        // "*" should produce just the package name, no version specifier
        let toml = format_simple_requirement("flask", "*");
        assert_eq!(toml, "flask");
    }

    #[test]
    fn bridge_specific_version() {
        let toml = format_simple_requirement("requests", ">=2.32.0");
        assert_eq!(toml, "requests>=2.32.0");
    }
}
