//! Tests for Pipfile parsing using the data model structs.
//!
//! These tests validate that our test fixtures parse correctly and that
//! the data model handles all the Pipfile spec variants.

use std::path::PathBuf;

use ripenv::pipfile::Pipfile;

/// Return the path to a test fixture Pipfile.
fn fixture(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures");
    path.push(name);
    path.push("Pipfile");
    path
}

#[test]
fn parse_minimal_pipfile() {
    let pipfile = Pipfile::from_path(&fixture("minimal")).expect("Failed to parse minimal Pipfile");

    assert_eq!(pipfile.source.len(), 1);
    assert_eq!(pipfile.source[0].name, "pypi");
    assert_eq!(pipfile.source[0].url, "https://pypi.org/simple");
    assert!(pipfile.source[0].verify_ssl);

    assert_eq!(pipfile.packages.len(), 1);
    assert!(pipfile.packages.contains_key("requests"));

    assert!(pipfile.dev_packages.is_empty());

    let requires = pipfile.requires.expect("Missing [requires]");
    assert_eq!(requires.python_version.as_deref(), Some("3.12"));
}

#[test]
fn parse_with_dev_deps() {
    let pipfile =
        Pipfile::from_path(&fixture("with-dev-deps")).expect("Failed to parse with-dev-deps");

    assert_eq!(pipfile.packages.len(), 2);
    assert_eq!(pipfile.dev_packages.len(), 2);
    assert!(pipfile.dev_packages.contains_key("pytest"));
    assert!(pipfile.dev_packages.contains_key("pytest-cov"));
}

#[test]
fn parse_multiple_sources() {
    let pipfile =
        Pipfile::from_path(&fixture("multiple-sources")).expect("Failed to parse multiple-sources");

    assert_eq!(pipfile.source.len(), 2);
    assert_eq!(pipfile.source[0].name, "pypi");
    assert_eq!(pipfile.source[1].name, "private");
    assert!(
        pipfile.source[1]
            .url
            .contains("my-private-index.example.com")
    );
}

#[test]
fn parse_vcs_and_editable() {
    use ripenv::pipfile::model::PipfilePackage;

    let pipfile =
        Pipfile::from_path(&fixture("vcs-and-editable")).expect("Failed to parse vcs-and-editable");

    // Check git package
    let git_pkg = pipfile
        .packages
        .get("my-git-pkg")
        .expect("Missing my-git-pkg");
    match git_pkg {
        PipfilePackage::Detailed(detail) => {
            assert!(detail.git.is_some());
            assert_eq!(detail.git_ref.as_deref(), Some("main"));
        }
        PipfilePackage::Simple(_) => panic!("Expected Detailed for git package"),
    }

    // Check editable package
    let editable_pkg = pipfile
        .packages
        .get("my-local-pkg")
        .expect("Missing my-local-pkg");
    match editable_pkg {
        PipfilePackage::Detailed(detail) => {
            assert!(detail.editable);
            assert_eq!(detail.path.as_deref(), Some("./local-pkg"));
        }
        PipfilePackage::Simple(_) => panic!("Expected Detailed for editable package"),
    }
}

#[test]
fn parse_with_scripts() {
    let pipfile =
        Pipfile::from_path(&fixture("with-scripts")).expect("Failed to parse with-scripts");

    assert_eq!(pipfile.scripts.len(), 3);
    assert_eq!(pipfile.scripts["test"], "pytest -vvs");
    assert_eq!(pipfile.scripts["serve"], "flask run --debug");
    assert_eq!(pipfile.scripts["lint"], "ruff check .");
}

#[test]
fn parse_complex_specs() {
    use ripenv::pipfile::model::PipfilePackage;

    let pipfile =
        Pipfile::from_path(&fixture("complex-specs")).expect("Failed to parse complex-specs");

    // Package with extras
    let requests = pipfile.packages.get("requests").expect("Missing requests");
    match requests {
        PipfilePackage::Detailed(detail) => {
            assert_eq!(detail.version.as_deref(), Some(">=2.32.0"));
            assert_eq!(detail.extras, vec!["security"]);
        }
        PipfilePackage::Simple(_) => panic!("Expected Detailed for requests with extras"),
    }

    // Package with sys_platform marker
    let stdeb = pipfile.packages.get("stdeb").expect("Missing stdeb");
    match stdeb {
        PipfilePackage::Detailed(detail) => {
            assert!(detail.sys_platform.is_some());
        }
        PipfilePackage::Simple(_) => panic!("Expected Detailed for stdeb"),
    }

    // Package with markers
    let legacy_cgi = pipfile
        .packages
        .get("legacy-cgi")
        .expect("Missing legacy-cgi");
    match legacy_cgi {
        PipfilePackage::Detailed(detail) => {
            assert!(detail.markers.is_some());
        }
        PipfilePackage::Simple(_) => panic!("Expected Detailed for legacy-cgi"),
    }

    // Editable dev package with extras
    let pipenv_pkg = pipfile
        .dev_packages
        .get("pipenv")
        .expect("Missing pipenv dev dep");
    match pipenv_pkg {
        PipfilePackage::Detailed(detail) => {
            assert!(detail.editable);
            assert_eq!(detail.path.as_deref(), Some("."));
            assert_eq!(detail.extras, vec!["tests", "dev"]);
        }
        PipfilePackage::Simple(_) => panic!("Expected Detailed for pipenv dev dep"),
    }

    // Pipenv settings
    let settings = pipfile.pipenv.expect("Missing [pipenv] section");
    assert!(settings.allow_prereleases);
}

#[test]
fn simple_version_string_parses() {
    use ripenv::pipfile::model::PipfilePackage;

    let pipfile = Pipfile::from_path(&fixture("minimal")).expect("Failed to parse");

    let requests = pipfile.packages.get("requests").expect("Missing requests");
    match requests {
        PipfilePackage::Simple(version) => {
            assert_eq!(version, "*");
        }
        PipfilePackage::Detailed(_) => panic!("Expected Simple for wildcard version"),
    }
}

#[test]
fn missing_pipfile_returns_error() {
    let result = Pipfile::from_path(&PathBuf::from("/nonexistent/Pipfile"));
    assert!(result.is_err());
}
