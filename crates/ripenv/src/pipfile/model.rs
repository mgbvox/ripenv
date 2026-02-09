//! Data model types for Pipfile deserialization.
//!
//! These structs mirror the Pipfile TOML schema. Packages can be specified
//! as either a simple version string (`"*"`, `">=1.0"`) or a table with
//! extended fields (`{version = ">=1.0", extras = ["security"]}`).

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

/// Top-level Pipfile structure.
#[derive(Debug, Deserialize)]
pub struct Pipfile {
    /// Package index sources.
    #[serde(default)]
    pub source: Vec<PipfileSource>,

    /// Production dependencies.
    #[serde(default)]
    pub packages: BTreeMap<String, PipfilePackage>,

    /// Development dependencies.
    #[serde(rename = "dev-packages", default)]
    pub dev_packages: BTreeMap<String, PipfilePackage>,

    /// Python version requirements.
    pub requires: Option<PipfileRequires>,

    /// Script definitions.
    #[serde(default)]
    pub scripts: BTreeMap<String, String>,

    /// Pipenv-specific settings.
    #[serde(default)]
    pub pipenv: Option<PipfileSettings>,
}

impl Pipfile {
    /// Parse a Pipfile from the given path.
    pub fn from_path(path: &Path) -> Result<Self> {
        let content = fs_err::read_to_string(path)?;
        let pipfile: Self = toml::from_str(&content)?;
        Ok(pipfile)
    }
}

/// A `[[source]]` entry in the Pipfile.
#[derive(Debug, Deserialize)]
pub struct PipfileSource {
    /// Source name (e.g., `"pypi"`).
    pub name: String,

    /// Index URL.
    pub url: String,

    /// Whether to verify SSL certificates.
    #[serde(default = "default_true")]
    pub verify_ssl: bool,
}

/// A package dependency in the Pipfile.
///
/// Pipfile packages can be either a simple version string like `"*"` or
/// `">=1.0"`, or a table with extended fields like
/// `{version = ">=1.0", extras = ["security"], markers = "..."}`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PipfilePackage {
    /// Simple version string: `requests = "*"` or `requests = ">=1.0"`.
    Simple(String),

    /// Table with extended fields: `requests = {version = ">=1.0", extras = ["security"]}`.
    Detailed(PipfilePackageDetail),
}

/// Extended package specification fields.
#[derive(Debug, Default, Deserialize)]
pub struct PipfilePackageDetail {
    /// Version specifier (e.g., `">=1.0"`, `"*"`).
    pub version: Option<String>,

    /// Extra features to install.
    #[serde(default)]
    pub extras: Vec<String>,

    /// PEP 508 environment markers.
    pub markers: Option<String>,

    /// Platform-specific marker shorthand (e.g., `"== 'linux'"`).
    pub sys_platform: Option<String>,

    /// Git repository URL.
    pub git: Option<String>,

    /// Git ref (branch, tag, or commit).
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,

    /// Local path to a package.
    pub path: Option<String>,

    /// Whether the package is installed as editable.
    #[serde(default)]
    pub editable: bool,

    /// Specific index to install from.
    pub index: Option<String>,
}

/// The `[requires]` section of a Pipfile.
#[derive(Debug, Deserialize)]
pub struct PipfileRequires {
    /// Python version (e.g., `"3.12"`).
    pub python_version: Option<String>,

    /// Full Python version (e.g., `"3.12.1"`).
    pub python_full_version: Option<String>,
}

/// The `[pipenv]` section for pipenv-specific settings.
#[derive(Debug, Deserialize)]
pub struct PipfileSettings {
    /// Whether to allow pre-release versions.
    #[serde(default)]
    pub allow_prereleases: bool,
}

/// Helper for serde default values.
fn default_true() -> bool {
    true
}
