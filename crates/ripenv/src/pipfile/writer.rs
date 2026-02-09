//! Pipfile writer: serialize a [`Pipfile`] back to TOML.
//!
//! Used by `ripenv install <pkg>` to update the Pipfile after adding packages.
//! Preserves standard section ordering:
//! `[[source]]`, `[packages]`, `[dev-packages]`, `[requires]`, `[scripts]`, `[pipenv]`.

use std::fmt::Write;
use std::path::Path;

use anyhow::Result;

use crate::pipfile::model::{Pipfile, PipfilePackage, PipfilePackageDetail};

impl Pipfile {
    /// Write the Pipfile to the given path.
    pub fn write_to(&self, path: &Path) -> Result<()> {
        let content = self.to_toml_string()?;
        fs_err::write(path, content)?;
        Ok(())
    }

    /// Serialize the Pipfile to a TOML string.
    pub fn to_toml_string(&self) -> Result<String> {
        let mut out = String::with_capacity(512);

        // [[source]] entries
        for source in &self.source {
            writeln!(out, "[[source]]")?;
            writeln!(out, "url = \"{}\"", source.url)?;
            writeln!(out, "verify_ssl = {}", source.verify_ssl)?;
            writeln!(out, "name = \"{}\"", source.name)?;
            writeln!(out)?;
        }

        // [packages]
        writeln!(out, "[packages]")?;
        write_packages(&mut out, &self.packages)?;
        writeln!(out)?;

        // [dev-packages]
        writeln!(out, "[dev-packages]")?;
        write_packages(&mut out, &self.dev_packages)?;
        writeln!(out)?;

        // [requires]
        if let Some(ref requires) = self.requires {
            writeln!(out, "[requires]")?;
            if let Some(ref version) = requires.python_version {
                writeln!(out, "python_version = \"{version}\"")?;
            }
            if let Some(ref full_version) = requires.python_full_version {
                writeln!(out, "python_full_version = \"{full_version}\"")?;
            }
            writeln!(out)?;
        }

        // [scripts]
        if !self.scripts.is_empty() {
            writeln!(out, "[scripts]")?;
            for (name, command) in &self.scripts {
                writeln!(out, "{name} = \"{}\"", escape_toml_value(command))?;
            }
            writeln!(out)?;
        }

        // [pipenv]
        if let Some(ref settings) = self.pipenv {
            writeln!(out, "[pipenv]")?;
            if settings.allow_prereleases {
                writeln!(out, "allow_prereleases = true")?;
            }
            writeln!(out)?;
        }

        Ok(out)
    }
}

/// Write a map of packages to TOML.
fn write_packages(
    out: &mut String,
    packages: &std::collections::BTreeMap<String, PipfilePackage>,
) -> Result<()> {
    for (name, pkg) in packages {
        match pkg {
            PipfilePackage::Simple(version) => {
                writeln!(out, "{name} = \"{version}\"")?;
            }
            PipfilePackage::Detailed(detail) => {
                write!(out, "{name} = {{")?;
                write_detail_fields(out, detail)?;
                writeln!(out, "}}")?;
            }
        }
    }
    Ok(())
}

/// Write the inline table fields for a detailed package spec.
fn write_detail_fields(out: &mut String, detail: &PipfilePackageDetail) -> Result<()> {
    let mut fields: Vec<String> = Vec::new();

    if let Some(ref version) = detail.version {
        fields.push(format!("version = \"{version}\""));
    }
    if !detail.extras.is_empty() {
        let extras: Vec<_> = detail.extras.iter().map(|e| format!("\"{e}\"")).collect();
        fields.push(format!("extras = [{}]", extras.join(", ")));
    }
    if let Some(ref git) = detail.git {
        fields.push(format!("git = \"{git}\""));
    }
    if let Some(ref git_ref) = detail.git_ref {
        fields.push(format!("ref = \"{git_ref}\""));
    }
    if let Some(ref path) = detail.path {
        fields.push(format!("path = \"{path}\""));
    }
    if detail.editable {
        fields.push("editable = true".to_owned());
    }
    if let Some(ref index) = detail.index {
        fields.push(format!("index = \"{index}\""));
    }
    if let Some(ref markers) = detail.markers {
        fields.push(format!("markers = \"{markers}\""));
    }
    if let Some(ref sys_platform) = detail.sys_platform {
        fields.push(format!("sys_platform = \"{sys_platform}\""));
    }

    write!(out, "{}", fields.join(", "))?;
    Ok(())
}

/// Escape a string for TOML double-quoted values.
fn escape_toml_value(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use crate::pipfile::model::Pipfile;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
            .join("Pipfile")
    }

    #[test]
    fn round_trip_minimal() {
        let pipfile = Pipfile::from_path(&fixture("minimal")).unwrap();
        let toml = pipfile.to_toml_string().unwrap();

        // Re-parse the generated TOML
        let reparsed: Pipfile = toml::from_str(&toml).unwrap();

        assert_eq!(reparsed.source.len(), 1);
        assert_eq!(reparsed.source[0].name, "pypi");
        assert_eq!(reparsed.packages.len(), 1);
        assert!(reparsed.packages.contains_key("requests"));
    }

    #[test]
    fn round_trip_with_scripts() {
        let pipfile = Pipfile::from_path(&fixture("with-scripts")).unwrap();
        let toml = pipfile.to_toml_string().unwrap();

        let reparsed: Pipfile = toml::from_str(&toml).unwrap();
        assert_eq!(reparsed.scripts.len(), 3);
        assert_eq!(reparsed.scripts["test"], "pytest -vvs");
    }

    #[test]
    fn round_trip_complex_specs() {
        let pipfile = Pipfile::from_path(&fixture("complex-specs")).unwrap();
        let toml = pipfile.to_toml_string().unwrap();

        let reparsed: Pipfile = toml::from_str(&toml).unwrap();
        assert_eq!(reparsed.packages.len(), pipfile.packages.len());
        assert_eq!(reparsed.dev_packages.len(), pipfile.dev_packages.len());
    }

    #[test]
    fn write_and_read_back() {
        let pipfile = Pipfile::from_path(&fixture("minimal")).unwrap();

        let dir = tempfile::TempDir::new().unwrap();
        let output = dir.path().join("Pipfile");
        pipfile.write_to(&output).unwrap();

        let reparsed = Pipfile::from_path(&output).unwrap();
        assert_eq!(reparsed.source.len(), 1);
        assert_eq!(reparsed.packages.len(), 1);
    }
}
